use std::{
    fmt::Debug,
    io::{self, ErrorKind, Result},
    process::exit,
};

use dbraft::{read_from_stream, write_to_stream};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::{
    communication::event::{DatabaseRequest, DatabaseResponse, RaftResponse},
    raft::raft_session::raft_session::RaftSession,
};

use super::store::datastore::{DataStore, HashMapStore};

pub struct Database<T> {
    store: Box<dyn DataStore<T>>,
    raft_connexion: Option<RaftSession>,
}

impl Database<String> {
    pub fn new() -> Self {
        Database {
            store: Box::new(HashMapStore::new()),
            raft_connexion: None,
        }
    }

    pub async fn handle_request(
        &mut self,
        request: DatabaseRequest<String>,
    ) -> Result<DatabaseResponse<String>> {
        // Asking Raft instances
        let raft_response = self
            .raft_connexion
            .as_mut()
            .unwrap()
            .ask_peers(request.clone())
            .await;

        match raft_response {
            Ok(raft_response) => match raft_response {
                RaftResponse::Committed => match request {
                    DatabaseRequest::Get(id) => self.get(id),
                    DatabaseRequest::Put { id, item } => self.put(id, item),
                },
                RaftResponse::VoteGranted => {
                    todo!()
                }
            },
            Err(err) => Err(io::Error::from(err)),
        }
    }

    fn get(&self, id: String) -> Result<DatabaseResponse<String>> {
        let opt = self.store.get(&id);

        match opt {
            Some(item) => {
                info!("Get success: id->{:?} | item->{:?}", id, item);
                Ok(DatabaseResponse::GetSuccess(item.clone()))
            }
            None => {
                error!("Get error, element not found");
                Err(io::Error::from(ErrorKind::NotFound))
            }
        }
    }

    fn put(&mut self, id: String, item: String) -> Result<DatabaseResponse<String>> {
        let res = self.store.put(id.clone(), item.clone());

        match res {
            Ok(()) => {
                info!("Put success: id->{:?} | item->{:?}", id, item.clone());
                Ok(DatabaseResponse::PutSuccess { id, item })
            }
            Err(err) => {
                error!("Error putting element");
                Err(err)
            }
        }
    }
}

pub async fn run_database<T: for<'a> Deserialize<'a> + Serialize + Debug + Clone + 'static>(
    mut database: Database<String>,
    port: u32,
    raft_addr: String,
) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    info!("Database started, listening on port {}", port);

    let raft_stream = TcpStream::connect(raft_addr).await;
    if raft_stream.is_ok() {
        database.raft_connexion = Some(RaftSession::new(raft_stream.unwrap()));
        info!("Database connected to local Raft instance");
    } else {
        error!("Couldn't connect to local Raft instance, exiting");
        exit(1);
    }

    let mut stream_listener = TcpListenerStream::new(listener);

    while let Some(stream) = stream_listener.try_next().await.unwrap() {
        info!("Received connection");

        let (read_stream, write_stream) = stream.into_split();

        let bytes = read_from_stream(read_stream).await;

        let request = DatabaseRequest::<String>::parse_from_bytes(bytes);

        if request.is_ok() {
            let request = request.unwrap();

            let response = database.handle_request(request).await;

            if response.is_ok() {
                let response = response.unwrap().into_bytes();

                write_to_stream(write_stream, response).await.unwrap();
            } else {
            }
        } else {
            error!("Unknown type of request");
        }
    }

    Ok(())
}
