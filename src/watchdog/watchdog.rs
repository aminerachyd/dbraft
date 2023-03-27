use std::{collections::HashMap, io::Result, sync::Arc, thread, time::Duration};

use dbraft::{read_from_stream, write_to_stream};
use futures::TryStreamExt;
use log::{error, info};
use tokio::{
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_stream::wrappers::TcpListenerStream;

use crate::{
    communication::send::{Broadcast, Heartbeat, P2PSend},
    raft::raft::Raft,
    watchdog::event::WatchdogEvent,
};

use super::event::{InstanceEvent, SerializeDeserialize};

type RaftInstances = Arc<RwLock<HashMap<u32, String>>>;

pub struct Watchdog {
    raft_instances: RaftInstances,
}

impl P2PSend for Watchdog {}
impl Broadcast for Watchdog {}
impl Heartbeat for Watchdog {}

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

        let raft_instances = Arc::clone(&self.raft_instances);

        // Periodically check alive instances and broadcast list
        tokio::spawn(async move {
            loop {
                Self::check_alive_instances(&raft_instances).await;

                thread::sleep(Duration::from_secs(10));
            }
        });

        while let Some(stream) = stream_listener.try_next().await.unwrap() {
            let raft_instances = Arc::clone(&self.raft_instances);

            tokio::spawn(async {
                Self::handle_stream(stream, raft_instances).await;
            });
        }

        Ok(())
    }

    async fn handle_stream(stream: TcpStream, raft_instances: RaftInstances) {
        let (read_stream, write_stream) = stream.into_split();

        let bytes = read_from_stream(read_stream).await.unwrap();

        let request = InstanceEvent::parse_from_bytes(bytes);

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
        event: InstanceEvent,
        stream: OwnedWriteHalf,
        raft_instances: RaftInstances,
    ) {
        match event {
            InstanceEvent::Register { addr } => {
                let watchdog_response = Self::register_new_instance(addr, raft_instances).await;
                write_to_stream(stream, watchdog_response.into_bytes()).await;
            }
        }
    }

    async fn register_new_instance(addr: String, raft_instances: RaftInstances) -> WatchdogEvent {
        let read_raft_instances = raft_instances.read().await;

        let max_id = read_raft_instances.keys().max();

        let mut new_id = 0;

        if max_id.is_some() {
            new_id = max_id.unwrap() + 1;
        }

        drop(read_raft_instances);

        raft_instances.write().await.insert(new_id, addr.clone());

        info!(
            "Registered new instance at addr {} with id {}",
            addr, new_id
        );

        WatchdogEvent::InstanceRegistered { id: new_id }
    }

    async fn check_alive_instances(raft_instances: &RaftInstances) {
        let read_raft_instances = raft_instances.read().await;

        let mut dead_instances: Vec<(u32, String)> = Vec::new();

        info!("Checking health of instances");

        for (id, addr) in &*read_raft_instances {
            if !Self::service_is_alive(addr).await {
                dead_instances.push((id.clone(), addr.clone()));
            }
        }

        if dead_instances.len() > 0 {
            tokio::task::spawn_blocking({
                let lock = Arc::clone(&raft_instances);
                move || {
                    let mut lock = lock.blocking_write();

                    let instances = &mut *lock;

                    for (id, addr) in dead_instances {
                        info!("Instance at {} is dead, removing it", addr.clone());
                        instances.remove(&id);
                    }

                    drop(lock);
                }
            });
        }
        Self::broadcast_instances_list(&raft_instances).await;
    }

    async fn broadcast_instances_list(raft_instances: &RaftInstances) {
        let raft_instances = &*raft_instances.read().await;
        let raft_instances_vec: Vec<&String> = raft_instances.iter().map(|(k, v)| v).collect();

        let data = WatchdogEvent::UpdateRaftInstances {
            peers: raft_instances.to_owned(),
        }
        .into_bytes();

        info!("Broadcasting instances list");
        Self::broadcast(raft_instances_vec, data).await;
    }

    async fn remove_instance(raft_instances: Arc<RwLock<HashMap<u32, String>>>, id: u32) {
        let raft_instances = &mut *raft_instances.write().await;

        raft_instances.remove(&id);
    }
}
