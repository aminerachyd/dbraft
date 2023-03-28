use std::io::Result;

use dbraft::{read_from_stream, write_to_stream};
use log::info;
use tokio::net::{tcp::OwnedReadHalf, TcpStream};

use crate::communication::event::{DatabaseRequest, DatabaseResponse};

pub struct DBConnexion {
    connexion: TcpStream,
}

pub async fn connect_to_database(addr: String) -> Result<DBConnexion> {
    let connexion = TcpStream::connect(addr).await?;
    info!("Client connected to database");

    Ok(DBConnexion { connexion })
}

impl DBConnexion {
    pub async fn get(self, id: String) -> Result<String> {
        let get_request = DatabaseRequest::<String>::Get(id).into_bytes();

        let (read_stream, write_stream) = self.connexion.into_split();

        write_to_stream(write_stream, get_request).await;

        Self::read_database_response(read_stream).await
    }

    pub async fn put(self, id: String, item: String) -> Result<String> {
        let put_request = DatabaseRequest::<String>::Put { id, item }.into_bytes();

        let (read_stream, write_stream) = self.connexion.into_split();

        write_to_stream(write_stream, put_request).await;

        Self::read_database_response(read_stream).await
    }

    async fn read_database_response(stream: OwnedReadHalf) -> Result<String> {
        let bytes = read_from_stream(stream).await;

        let response = DatabaseResponse::<String>::parse_from_bytes(bytes.unwrap());

        match response {
            Ok(db_response) => match db_response {
                DatabaseResponse::GetSuccess(item) => Ok(item),
                DatabaseResponse::PutSuccess { id: _, item } => Ok(item),
            },
            Err(err) => Err(err),
        }
    }
}
