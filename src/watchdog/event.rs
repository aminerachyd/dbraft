use std::io::{self, ErrorKind, Result};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    Register { addr: String },
}

#[derive(Serialize, Deserialize)]
pub enum WatchdogResponse {
    Registered { id: u32 },
}

impl SerializeDeserialize for Event {}
impl SerializeDeserialize for WatchdogResponse {}

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
