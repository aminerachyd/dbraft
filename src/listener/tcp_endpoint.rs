use std::io::Result;

use log::info;
use tokio::{io::AsyncWriteExt, net::TcpListener};
use tokio_stream::{wrappers::TcpListenerStream, StreamExt};

use crate::db_connexion::db_connexion::connect_to_database;

pub async fn start_endpoint(port: u32) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Endpoint started, listening on port {}", port);

    let mut stream_listener = TcpListenerStream::new(listener);

    while let Some(mut stream) = stream_listener.try_next().await.unwrap() {
        info!("Endpoint received connection");

        tokio::spawn(async move {
            let res = client_test().await;

            stream
                .write_all(format!("{}\n\n", res).as_bytes())
                .await
                .unwrap();

            stream.shutdown().await.unwrap();
        });
    }

    Ok(())
}

async fn client_test() -> String {
    // Write
    connect_to_database("0.0.0.0:8888".to_owned())
        .await
        .unwrap()
        .put("1".to_owned(), "Hello".to_owned())
        .await
        .unwrap();

    // Read
    connect_to_database("0.0.0.0:8888".to_owned())
        .await
        .unwrap()
        .get("1".to_owned())
        .await
        .unwrap()
}
