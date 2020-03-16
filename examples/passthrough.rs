#[macro_use]
extern crate log;

use async_std::io;
use futures::channel::oneshot;
use mariadb_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};

struct PassthroughHandler {}

// Just forward the packet
#[async_trait::async_trait]
impl PacketHandler for PassthroughHandler {
    async fn handle_request(&mut self, p: &Packet) -> Packet {
        debug!("c=>s: {:?} packet: {} bytes", p.get_packet_type(), p.get_size());
        p.clone()
    }

    async fn handle_response(&mut self, p: &Packet) -> Packet {
        debug!("c<=s: {:?} packet: {} bytes", p.get_packet_type(), p.get_size());
        p.clone()
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut args = std::env::args().skip(1);

    info!("Passthrough proxy... ");

    // determine address for the proxy to bind to
    //let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:3306".to_string());    // MariaDB
    let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:5432".to_string());      // Postgres

    // determine address of the MariaDB instance we are proxying for
    //let db_addr = args.next().unwrap_or_else(|| "mariadb-server:3306".to_string());    // MariaDB 
    let db_addr = args.next().unwrap_or_else(|| "postgres-server:5432".to_string());      // Postgres
    
    // determine what type of database it is
    //let db_type_str = args.next().unwrap_or_else(|| "mariadb".to_string());
    let db_type_str = args.next().unwrap_or_else(|| "postgres".to_string());
    let mut db_type = DatabaseType::PostgresSQL;
    if db_type_str == "mariadb" {
        db_type = DatabaseType::MariaDB;
    }

    let mut server = mariadb_proxy::server::Server::new(
        bind_addr.clone(),
        db_type,
        db_addr.clone(),
    )
    .await;

    let (tx, rx) = oneshot::channel(); // kill switch
    tokio::spawn(async move {
        info!("Proxy listening on: {}", bind_addr);
        server.run(PassthroughHandler {}, rx).await;
    });

    // Run until use hits enter
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.read_line(&mut line).await {
        Ok(_) => tx.send(()).unwrap(),
        Err(_) => tx.send(()).unwrap(),
    };
    info!("...exiting");
}
