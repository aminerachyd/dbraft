use std::io::Result;

use async_trait::async_trait;
use dbraft::write_to_stream;
use tokio::net::TcpStream;

use crate::communication::event::{Event, HeartbeatMessage};

use super::impl_event::SerializeDeserialize;

#[async_trait]
pub trait P2PSend {
    async fn send(addr: &String, data: &Vec<u8>) -> Result<()> {
        let stream = TcpStream::connect(addr).await?;

        let write_stream = stream.into_split().1;

        write_to_stream(write_stream, data.to_owned()).await
    }
}

#[async_trait]
pub trait Broadcast: P2PSend {
    async fn broadcast(addr_list: Vec<&String>, data: Vec<u8>) {
        for addr in addr_list {
            Self::send(addr, &data).await;
        }
    }
}

#[async_trait]
pub trait Heartbeat: P2PSend {
    async fn service_is_alive(addr: &String) -> bool {
        let heartbeat = Event::HeartbeatMessage(HeartbeatMessage::Test).into_bytes();
        match Self::send(addr, &heartbeat).await {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}
