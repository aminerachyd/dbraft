mod db;
mod db_connexion;
mod listener;

use db::database::{run_database, Database};
use listener::*;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();

    let db = Database::<String>::new(0);

    tokio::spawn(async {
        // tcp_endpoint::start_endpoint(42069).await.unwrap();
        http_endpoint::start_endpoint(42069).await.unwrap();
    });

    run_database(db, 8888).await.unwrap();
}
