use std::{
    fmt::Debug,
    io::{self, ErrorKind, Result},
};

use dbraft::{read_from_stream, write_to_stream};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::db::request::Request;

use super::{
    db_response::DatabaseResponse,
    store::datastore::{DataStore, HashMapStore},
};

pub struct Database<T> {
    store: Box<dyn DataStore<T>>,
}

impl<T: Clone + Debug + 'static> Database<T> {
    pub fn new() -> Self {
        Database {
            store: Box::new(HashMapStore::new()),
        }
    }

    pub fn handle_request(&mut self, request: Request<T>) -> Result<DatabaseResponse<T>> {
        match request {
            Request::Get(id) => self.get(id),
            Request::Put { id, item } => self.put(id, item),
        }
    }

    fn get(&self, id: String) -> Result<DatabaseResponse<T>> {
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

    fn put(&mut self, id: String, item: T) -> Result<DatabaseResponse<T>> {
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
    mut database: Database<T>,
    port: u32,
) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;

    info!("Database started, listening on port {}", port);

    let mut stream_listener = TcpListenerStream::new(listener);

    while let Some(stream) = stream_listener.try_next().await.unwrap() {
        info!("Received connection");

        let (read_stream, write_stream) = stream.into_split();

        let bytes = read_from_stream(read_stream).await;

        let request = Request::<T>::parse_from_bytes(bytes.unwrap());

        if request.is_ok() {
            let request = request.unwrap();

            let response = database.handle_request(request);

            if response.is_ok() {
                let response = response.unwrap().into_bytes();

                write_to_stream(write_stream, response).await;
            } else {
            }
        } else {
            error!("Unknown type of request");
        }
    }

    Ok(())
}
