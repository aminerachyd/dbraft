use std::{collections::HashMap, io::Result, sync::Arc};

use dbraft::{read_from_stream, write_to_stream};
use log::{error, info};
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::watchdog::event::WatchdogResponse;

use super::event::{Event, SerializeDeserialize};

pub struct Watchdog {
    raft_instances: Arc<RwLock<HashMap<u32, String>>>,
}

struct RaftInstance {
    id: u32,
    addr: String,
}

impl Watchdog {
    pub fn new() -> Self {
        Watchdog {
            raft_instances: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self, port: u32) -> Result<()> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

        info!("Watchdog started, listening on port {}", port);

        let mut stream_listener = TcpListenerStream::new(listener);

        while let Some(stream) = stream_listener.try_next().await.unwrap() {
            let raft_instances = Arc::clone(&self.raft_instances);

            tokio::spawn(async {
                Self::handle_stream(stream, raft_instances).await;
            });
        }

        Ok(())
    }

    async fn handle_stream(stream: TcpStream, raft_instances: Arc<RwLock<HashMap<u32, String>>>) {
        let (read_stream, write_stream) = stream.into_split();

        let bytes = read_from_stream(read_stream).await.unwrap();

        let request = Event::parse_from_bytes(bytes);

        match request {
            Ok(event) => {
                info!("Received event {:?}", event);
                Self::handle_event(event, write_stream, raft_instances).await;
            }
            Err(_) => {
                error!("Unknown event");
            }
        }
    }

    async fn handle_event(
        event: Event,
        stream: OwnedWriteHalf,
        raft_instances: Arc<RwLock<HashMap<u32, String>>>,
    ) {
        match event {
            Event::Register { addr } => {
                let read_raft_instances = raft_instances.read().await;

                let max_id = read_raft_instances.keys().max();
                let mut new_id = 0;

                if max_id.is_some() {
                    new_id = max_id.unwrap() + 1;
                }

                drop(read_raft_instances);

                raft_instances.write().await.insert(new_id, addr.clone());

                dbg!("here");

                info!(
                    "Registered new instance at addr {} with id {}",
                    addr, new_id
                );

                let watchdog_response = WatchdogResponse::Registered { id: new_id };

                write_to_stream(stream, watchdog_response.into_bytes()).await;
            }
        }
    }

    async fn get_instances(
        raft_instances: Arc<RwLock<HashMap<u32, String>>>,
    ) -> HashMap<u32, String> {
        let raft_instances = &*raft_instances.read().await;

        raft_instances.to_owned()
    }

    async fn register_instance(
        raft_instances: Arc<RwLock<HashMap<u32, String>>>,
        RaftInstance { id, addr }: RaftInstance,
    ) {
        let raft_instances = &mut *raft_instances.write().await;
        raft_instances.insert(id, addr);
    }

    async fn remove_instance(
        raft_instances: Arc<RwLock<HashMap<u32, String>>>,
        RaftInstance { id, addr: _ }: RaftInstance,
    ) {
        let raft_instances = &mut *raft_instances.write().await;

        raft_instances.remove(&id);
    }
}
