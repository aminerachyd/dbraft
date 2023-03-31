use serde::{Deserialize, Serialize};
use std::io::{self, ErrorKind, Result};

use super::event::{
    DatabaseRequest, DatabaseResponse, Event, HeartbeatMessage, InstanceEvent, RaftMessage,
    RaftResponse, WatchdogEvent,
};
impl<T: for<'a> Deserialize<'a> + Serialize + Clone> DatabaseResponse<T> {
    pub fn parse_from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let result = serde_json::from_slice(&bytes[..]);

        if result.is_ok() {
            Ok(result.unwrap())
        } else {
            Err(io::Error::from(ErrorKind::InvalidData))
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}

impl<T: for<'a> Deserialize<'a> + Serialize> DatabaseRequest<T> {
    pub fn parse_from_bytes(bytes: Vec<u8>) -> Result<Self> {
        let result = serde_json::from_slice(&bytes[..]);

        if result.is_ok() {
            Ok(result.unwrap())
        } else {
            Err(io::Error::from(ErrorKind::InvalidData))
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        serde_json::to_vec(&self).unwrap()
    }
}

impl SerializeDeserialize for Event {}
impl SerializeDeserialize for InstanceEvent {}
impl SerializeDeserialize for WatchdogEvent {}
impl SerializeDeserialize for HeartbeatMessage {}
impl SerializeDeserialize for RaftMessage {}
impl SerializeDeserialize for RaftResponse {}

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
