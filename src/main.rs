mod db;
mod db_connexion;
mod listener;
mod raft;
mod watchdog;

use std::io::Result;

use db::database::{run_database, Database};
use listener::*;
use raft::raft::Raft;
use simple_logger::SimpleLogger;
use watchdog::watchdog::Watchdog;

#[tokio::main]
async fn main() {
    SimpleLogger::new().init().unwrap();

    let watchdog_port = 7000;
    let watchdog_addr = format!("0.0.0.0:{watchdog_port}");

    if let Err(_) = start_watchdog(7000).await {
        let db = Database::<String>::new();
        let raft = Raft::new();

        tokio::spawn(async {
            // tcp_endpoint::start_endpoint(42069).await.unwrap();
            http_endpoint::start_endpoint(42069).await.unwrap();
        });

        tokio::spawn(async { raft.run(6942, watchdog_addr).await });

        run_database(db, 8888).await.unwrap();
    }
}

async fn start_watchdog(port: u32) -> Result<()> {
    Watchdog::new().run(port).await
}
