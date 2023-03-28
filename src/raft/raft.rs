use std::{
    collections::HashMap,
    io::{self, Result},
    process::exit,
    sync::Arc,
    thread,
    time::Duration,
};

use dbraft::{read_from_stream, write_to_stream};
use log::{error, info};
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::{
    communication::event::{Event, InstanceEvent, SerializeDeserialize, WatchdogEvent},
    communication::send::{self, Heartbeat, P2PSend},
};

type RaftInstances = Arc<RwLock<HashMap<u32, String>>>;

#[derive(Clone)]
pub struct Raft {
    id: Option<u32>,
    addr: String,
    peers: RaftInstances,
}

impl P2PSend for Raft {}
impl Heartbeat for Raft {}

impl Raft {
    pub fn new() -> Self {
        Raft {
            id: None,
            addr: String::new(),
            peers: Arc::new(RwLock::new(HashMap::new())),
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

        if let Some(bytes) = read_from_stream(read_stream).await {
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
                },
                Err(_) => {
                    error!("Unknown watchdog response")
                }
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

                thread::sleep(Duration::from_secs(5));
            }
        });

        // Periodically ping all instances
        let peers = Arc::clone(&self.peers);
        tokio::spawn(async move {
            loop {
                // dbg!(peers.len());

                Self::ping_all_peers(&peers).await;

                thread::sleep(Duration::from_secs(2));
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
        let event = Event::parse_from_bytes(read_from_stream(read_stream).await.unwrap());

        match event {
            Ok(event) => match event {
                Event::WatchdogEvent(watchdog_event) => {
                    self.handle_watchdog_event(watchdog_event).await;
                }
                Event::InstanceEvent(instance_event) => {
                    Self::handle_instance_event(instance_event, write_stream).await;
                }
                Event::HeartbeatMessage(message) => {}
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

    async fn ping_all_peers(peers: &RaftInstances) {
        let peers = &*peers.read().await;

        dbg!(peers.len());

        for (_, addr) in peers {
            info!("Sending ping to {}", &addr);
            let ping = Event::InstanceEvent(InstanceEvent::Ping);
            Self::send(&addr, &ping.into_bytes()).await.unwrap();
        }
    }
}
