mod communication;
mod db;
mod db_connexion;
mod listener;
mod raft;
mod watchdog;

use std::io::Result;

use db::database::{run_database, Database};
use dbraft::InstancePorts;
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
        let mut ports = InstancePorts::new();
        while start_dbraft_instance(watchdog_addr.clone(), ports.clone())
            .await
            .is_err()
        {
            ports.add(1);
        }
    }
}

async fn start_watchdog(port: u32) -> Result<()> {
    Watchdog::new().run(port).await
}

async fn start_dbraft_instance(watchdog_addr: String, ports: InstancePorts) -> Result<()> {
    let (mut endpoint_port, mut raft_port, mut db_port) = (42069, 8000, 9000);

    tokio::spawn(async move {
        // tcp_endpoint::start_endpoint(ports.endpoint).await.unwrap();
        while http_endpoint::start_endpoint(endpoint_port).await.is_err() {
            endpoint_port += 1;
        }
    });

    let raft = Raft::new();
    tokio::spawn(async move {
        while raft
            .clone()
            .run(raft_port, watchdog_addr.clone())
            .await
            .is_err()
        {
            raft_port += 1;
        }
    });

    let mut db = Database::<String>::new();
    while run_database(db, db_port).await.is_err() {
        db = Database::<String>::new();
        db_port += 1;
    }
    Ok(())
}
