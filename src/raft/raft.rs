use std::{collections::HashMap, io::Result, thread, time::Duration};

use dbraft::{read_from_stream, write_to_stream};
use log::{error, info};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::watchdog::event::{Event, SerializeDeserialize, WatchdogResponse};

#[derive(Clone)]
pub struct Raft {
    id: Option<u32>,
    addr: String,
    peers: HashMap<String, String>,
}

impl Raft {
    pub fn new() -> Self {
        Raft {
            id: None,
            addr: String::new(),
            peers: HashMap::new(),
        }
    }

    pub async fn connect_to_watchdog(&mut self, watchdog_addr: String) -> Result<()> {
        info!("Connecting to watch dog...");
        let stream = TcpStream::connect(watchdog_addr).await?;

        let (read_stream, write_stream) = stream.into_split();

        let request = Event::Register {
            addr: self.addr.clone(),
        };

        write_to_stream(write_stream, request.into_bytes()).await;

        if let Some(bytes) = read_from_stream(read_stream).await {
            let watchdog_response = WatchdogResponse::parse_from_bytes(bytes);

            match watchdog_response {
                Ok(watchdog_response) => match watchdog_response {
                    WatchdogResponse::Registered { id } => {
                        self.id = Some(id);
                    }
                },
                Err(_) => {
                    error!("Unknown watchdog response")
                }
            }
        }

        info!("Connected to watch dog");
        Ok(())
    }

    pub async fn run(mut self, port: u32, watchdog_addr: String) -> Result<()> {
        let listener = TcpListener::bind(format! {"0.0.0.0:{}",port}).await?;

        info!("Raft instance started, listening on port {}", port);

        self.addr = format!("0.0.0.0:{}", port);

        while self.id.is_none() {
            self.connect_to_watchdog(watchdog_addr.clone()).await;

            thread::sleep(Duration::from_secs(2));
        }

        let mut stream_listener = TcpListenerStream::new(listener);

        while let Some(mut stream) = stream_listener.try_next().await.unwrap() {
            stream
                .write_all("Hello from Raft instance".as_bytes())
                .await?;
        }

        Ok(())
    }
}
