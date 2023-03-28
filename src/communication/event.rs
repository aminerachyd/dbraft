use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    InstanceEvent(InstanceEvent),
    WatchdogEvent(WatchdogEvent),
    HeartbeatMessage(HeartbeatMessage),
    DatabaseRequest(DatabaseRequest<String>),
    RaftResponse(RaftResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum InstanceEvent {
    Register { addr: String },
    Ping,
    Pong,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum WatchdogEvent {
    InstanceRegistered { id: u32 },
    UpdateRaftInstances { peers: HashMap<u32, String> },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum HeartbeatMessage {
    Test,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DatabaseRequest<T> {
    Get(String),
    Put { id: String, item: T },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DatabaseResponse<T: Clone> {
    GetSuccess(T),
    PutSuccess { id: String, item: T },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum RaftResponse {
    Committed,
}
