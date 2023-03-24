use std::io::{self, ErrorKind, Result};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum DatabaseResponse<T: Clone> {
    GetSuccess(T),
    PutSuccess { id: String, item: T },
}

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
