use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};

#[derive(Clone, Copy)]
pub struct InstancePorts {
    pub db: u32,
    pub endpoint: u32,
    pub raft: u32,
}

impl InstancePorts {
    pub fn new() -> Self {
        InstancePorts {
            db: 8888,
            endpoint: 42069,
            raft: 9000,
        }
    }

    pub fn add(self, number: u32) -> Self {
        InstancePorts {
            db: self.db + number,
            endpoint: self.endpoint + number,
            raft: self.raft + number,
        }
    }
}

pub async fn read_from_stream(stream: OwnedReadHalf) -> Option<Vec<u8>> {
    let buf_reader = BufReader::new(stream);

    let mut lines = buf_reader.lines();

    let mut result = String::new();
    while let Some(line) = lines.next_line().await.unwrap() {
        result = format!("{}\n", line);
    }
    Some(result.into())
}

pub async fn write_to_stream(mut stream: OwnedWriteHalf, bytes: Vec<u8>) {
    stream.write_all(&bytes[..]).await.unwrap();
    stream.flush().await.unwrap();
}
