use std::{
    collections::HashMap,
    io::{self, ErrorKind, Result},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum InstanceEvent {
    Register { addr: String },
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

impl SerializeDeserialize for InstanceEvent {}
impl SerializeDeserialize for WatchdogEvent {}
impl SerializeDeserialize for HeartbeatMessage {}

pub trait SerializeDeserialize: Sized + Serialize + for<'a> Deserialize<'a> {
    fn parse_from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let result = serde_json::from_slice(&bytes[..]);

        if result.is_ok() {
            Ok(result.unwrap())
        } else {
            Err(io::Error::from(ErrorKind::InvalidData))
        }
    }

    fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}
