use std::io::{Error, ErrorKind, Result};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
};

use crate::communication::{
    event::{DatabaseRequest, Event, RaftResponse},
    impl_event::SerializeDeserialize,
};

pub struct RaftSession {
    stream: TcpStream,
}

impl RaftSession {
    pub fn new(stream: TcpStream) -> Self {
        RaftSession { stream }
    }

    pub async fn ask_peers(
        &mut self,
        database_request: DatabaseRequest<String>,
    ) -> Result<RaftResponse> {
        let (read_stream, write_stream) = self.stream.split();

        // Write to Raft instance
        let event = Event::DatabaseRequest(database_request);
        write_to_stream(write_stream, event.into_bytes()).await;

        // Read response
        let response = read_from_stream(read_stream).await;

        dbg!(response);

        unimplemented!()
    }
}

async fn read_from_stream(stream: ReadHalf<'_>) -> Option<Vec<u8>> {
    let buf_reader = BufReader::new(stream);

    let mut lines = buf_reader.lines();

    let mut result = String::new();
    while let Some(line) = lines.next_line().await.unwrap() {
        result = format!("{}\n", line);
    }
    Some(result.into())
}

async fn write_to_stream(mut stream: WriteHalf<'_>, bytes: Vec<u8>) -> Result<()> {
    stream.write_all(&bytes[..]).await?;

    match stream.flush().await {
        Ok(()) => Ok(()),
        Err(_) => Err(Error::from(ErrorKind::AddrNotAvailable)),
    }
}
