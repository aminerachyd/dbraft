use std::{collections::HashMap, io::Result, process::exit, thread, time::Duration};

use dbraft::{read_from_stream, write_to_stream};
use log::{error, info};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::{
    communication::send::{Heartbeat, P2PSend},
    watchdog::event::{InstanceEvent, SerializeDeserialize, WatchdogEvent},
};

#[derive(Clone)]
pub struct Raft {
    id: Option<u32>,
    addr: String,
    peers: HashMap<u32, String>,
}

impl P2PSend for Raft {}
impl Heartbeat for Raft {}

impl Raft {
    pub fn new() -> Self {
        Raft {
            id: None,
            addr: String::new(),
            peers: HashMap::new(),
        }
    }

    pub async fn connect_to_watchdog(&mut self, watchdog_addr: String) -> Result<()> {
        info!("Connecting to watch dog...");
        let stream = TcpStream::connect(watchdog_addr).await?;

        let (read_stream, write_stream) = stream.into_split();

        let request = InstanceEvent::Register {
            addr: self.addr.clone(),
        };

        write_to_stream(write_stream, request.into_bytes()).await;

        if let Some(bytes) = read_from_stream(read_stream).await {
            let watchdog_response = WatchdogEvent::parse_from_bytes(bytes);

            match watchdog_response {
                Ok(watchdog_response) => match watchdog_response {
                    WatchdogEvent::InstanceRegistered { id } => {
                        self.id = Some(id);
                    }
                    WatchdogEvent::UpdateRaftInstances { peers: _ } => {}
                },
                Err(_) => {
                    error!("Unknown watchdog response")
                }
            }
        }

        info!("Connected to watch dog");
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

        // Listen for events
        let mut stream_listener = TcpListenerStream::new(listener);

        while let Some(stream) = stream_listener.try_next().await.unwrap() {
            self.handle_stream(stream).await;
        }

        Ok(())
    }

    async fn handle_stream(&mut self, stream: TcpStream) {
        let (read_stream, write_stream) = stream.into_split();
        let watchdog_event =
            WatchdogEvent::parse_from_bytes(read_from_stream(read_stream).await.unwrap());

        if let Ok(watchdog_event) = watchdog_event {
            match watchdog_event {
                WatchdogEvent::UpdateRaftInstances { peers } => {
                    info!("Received instances list {:?}", &peers);
                    self.peers = peers;
                }
                WatchdogEvent::InstanceRegistered { id: _ } => {}
            }
        }
    }
}
