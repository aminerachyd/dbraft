use std::{collections::HashMap, io::Result, process::exit, sync::Arc, thread, time::Duration};

use dbraft::{read_from_stream, write_to_stream};
use log::{error, info};
use rand::Rng;
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::{
    communication::event::{Event, InstanceEvent, WatchdogEvent},
    communication::{
        event::{DatabaseRequest, RaftMessage, RaftResponse},
        impl_event::SerializeDeserialize,
        send::{self, Broadcast, Heartbeat, P2PSend},
    },
};

use super::log_entry::LogEntry;

type RaftInstances = Arc<RwLock<HashMap<u32, String>>>;
type ReplicatedLog = Arc<RwLock<Vec<LogEntry>>>;

#[derive(Clone)]
pub struct Raft {
    id: Option<u32>,
    addr: String,
    peers: RaftInstances,
    current_term: u32,
    voted_for: Option<u32>,
    log: ReplicatedLog,
}

impl P2PSend for Raft {}
impl Heartbeat for Raft {}
impl Broadcast for Raft {}

impl Raft {
    pub fn new() -> Self {
        Raft {
            id: None,
            addr: String::new(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            current_term: 0,
            voted_for: None,
            log: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn connect_to_watchdog(&mut self, watchdog_addr: String) -> Result<()> {
        info!("Connecting to watch dog...");
        let stream = TcpStream::connect(watchdog_addr).await?;

        let (read_stream, write_stream) = stream.into_split();

        let request = Event::InstanceEvent(InstanceEvent::Register {
            addr: self.addr.clone(),
        });

        write_to_stream(write_stream, request.into_bytes()).await;

        let bytes = read_from_stream(read_stream).await;

        let watchdog_response = Event::parse_from_bytes(bytes);

        match watchdog_response {
            Ok(watchdog_response) => match watchdog_response {
                Event::WatchdogEvent(watchdog_event) => match watchdog_event {
                    WatchdogEvent::InstanceRegistered { id } => {
                        self.id = Some(id);
                        info!("Connected to watch dog");
                    }
                    WatchdogEvent::UpdateRaftInstances { peers: _ } => {}
                },
                Event::HeartbeatMessage(_) => {
                    // Receiving heartbeat from another process
                }
                Event::InstanceEvent(_) => {
                    // Receiving instances event
                }
                Event::DatabaseRequest(database_request) => {
                    // TODO
                    dbg!(database_request);
                }
                Event::RaftResponse(raft_response) => {
                    // TODO
                    dbg!(raft_response);
                }
                Event::RaftMessage(raft_message) => {
                    // TODO
                    dbg!(raft_message);
                }
            },
            Err(_) => {
                error!("Unknown watchdog response")
            }
        }

        Ok(())
    }

    pub async fn run(mut self, port: u32, watchdog_addr: String) -> Result<()> {
        let listener = TcpListener::bind(format! {"0.0.0.0:{}",port}).await?;

        info!("Raft instance started, listening on port {}", port);

        self.addr = format!("0.0.0.0:{}", port);

        while self.id.is_none() {
            self.connect_to_watchdog(watchdog_addr.clone()).await?;

            thread::sleep(Duration::from_secs(2));
        }

        // Periodically check Watchdog if alive, if not exit program
        tokio::spawn(async move {
            loop {
                if Self::service_is_alive(&watchdog_addr.clone()).await == false {
                    error!("Watchdog is dead, exiting");
                    exit(1);
                }

                thread::sleep(Duration::from_secs(10));
            }
        });

        // // Periodically ping all instances
        // let peers = Arc::clone(&self.peers);
        // tokio::spawn(async move {
        //     loop {
        //         Self::ping_all_peers(&peers).await;

        //         thread::sleep(Duration::from_secs(15));
        //     }
        // });

        // Leader election timeout
        let peers = Arc::clone(&self.peers);
        let log = Arc::clone(&self.log);
        tokio::spawn(async move {
            loop {
                let timeout = rand::thread_rng().gen_range(150..300);
                thread::sleep(Duration::from_millis(timeout));

                Self::request_votes(&peers, self.current_term, self.id.unwrap(), &log).await;
            }
        });

        // Listen for events
        let mut stream_listener = TcpListenerStream::new(listener);

        while let Some(stream) = stream_listener.try_next().await.unwrap() {
            self.handle_stream(stream).await;
        }

        Ok(())
    }

    async fn handle_stream(&mut self, stream: TcpStream) {
        let (read_stream, write_stream) = stream.into_split();
        let event = Event::parse_from_bytes(read_from_stream(read_stream).await);

        match event {
            Ok(event) => match event {
                Event::WatchdogEvent(watchdog_event) => {
                    self.handle_watchdog_event(watchdog_event).await;
                }
                Event::InstanceEvent(instance_event) => {
                    Self::handle_instance_event(instance_event, write_stream).await;
                }
                Event::HeartbeatMessage(message) => {}
                Event::DatabaseRequest(database_request) => {
                    dbg!(&database_request);
                    self.handle_database_request(database_request).await;
                }
                Event::RaftResponse(raft_response) => {
                    // TODO
                    dbg!(raft_response);
                }
                Event::RaftMessage(raft_message) => {
                    // TODO
                    dbg!(&raft_message);
                    match raft_message {
                        RaftMessage::RequestVote {
                            candidate_term,
                            candidate_id,
                            last_log_index,
                            last_log_term,
                        } => {
                            info!("Received vote from {}", candidate_id);
                            if candidate_term >= self.current_term {
                                // Send VoteGranted to this candidate
                                let peers = self.peers.read().await;
                                let peer_addr = peers.get(&candidate_id).unwrap();
                                let response = RaftResponse::VoteGranted;

                                Self::send(peer_addr, &response.into_bytes()).await;
                                self.voted_for = Some(candidate_id);
                            }
                        }
                        RaftMessage::AppendEntry => {
                            // Heartbeat message + append
                            // Election timeout must reset because leader exists
                            info!("Received {:?}", raft_message);
                        }
                    }
                }
            },
            Err(_) => {
                error!("Unknown event")
            }
        }
    }

    async fn handle_instance_event(instance_event: InstanceEvent, stream: OwnedWriteHalf) {
        match instance_event {
            InstanceEvent::Ping => {
                let pong = Event::InstanceEvent(InstanceEvent::Pong);
                info!("Received Ping, sending Pong");
                write_to_stream(stream, pong.into_bytes()).await.unwrap();
            }
            InstanceEvent::Register { addr: _ } => {
                // We ain't registering here
            }
            InstanceEvent::Pong => {
                info!("Received Pong");
            }
        }
    }

    async fn handle_watchdog_event(&mut self, watchdog_event: WatchdogEvent) {
        match watchdog_event {
            WatchdogEvent::UpdateRaftInstances { peers } => {
                info!("Received instances list {:?}", &peers);
                let write_peers = &mut *self.peers.write().await;
                let _ = std::mem::replace(write_peers, peers);
            }
            WatchdogEvent::InstanceRegistered { id: _ } => {}
        }
    }

    // TODO Forward request to peers
    async fn handle_database_request(&mut self, database_event: DatabaseRequest<String>) {
        let instances = self.peers.read().await;
        let addr_list = instances.iter().map(|(_, addr)| addr).collect();

        let database_event = database_event.into_bytes();

        // Broadcast event to other Raft instances
        Self::broadcast(addr_list, database_event).await;
    }

    async fn ping_all_peers(peers: &RaftInstances) {
        let peers = &*peers.read().await;

        for (_, addr) in peers {
            info!("Sending ping to {}", &addr);
            let ping = Event::InstanceEvent(InstanceEvent::Ping);
            Self::send(&addr, &ping.into_bytes()).await.unwrap();
        }
    }

    async fn request_votes(
        peers: &RaftInstances,
        current_term: u32,
        candidate_id: u32,
        log: &ReplicatedLog,
    ) -> bool {
        let peers = &*peers.read().await;
        let log = &*log.read().await;
        let majority = peers.len() / 2 + 1;
        let mut granted_votes = 0;
        let mut responses: Vec<RaftResponse> = Vec::new();

        info!("Starting leader election round");

        for (id, addr) in peers {
            // TODO peers includes self, does it request its vote ?
            let stream = TcpStream::connect(addr).await.unwrap();

            let (read_stream, write_stream) = stream.into_split();

            let request = RaftMessage::RequestVote {
                candidate_term: current_term,
                candidate_id,
                last_log_index: log.len() as u32 - 1,
                last_log_term: log
                    .last()
                    .unwrap_or(&LogEntry {
                        term: 0,
                        command: "String".to_owned(),
                    })
                    .term,
            };

            write_to_stream(write_stream, request.into_bytes());

            let response = read_from_stream(read_stream).await;

            let raft_response = RaftResponse::parse_from_bytes(response).unwrap();

            responses.push(raft_response);
        }

        responses.iter().for_each(|response| {
            if let RaftResponse::VoteGranted = response {
                granted_votes += 1;
            }
        });

        granted_votes >= majority
    }
}
