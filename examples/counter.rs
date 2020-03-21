#[macro_use]
extern crate log;

use async_std::io;
use futures::channel::oneshot;
use sql_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};
use std::collections::HashMap;

struct CounterHandler {
    count_map: HashMap<String, u64>,
}

impl CounterHandler {
    fn new() -> CounterHandler {
        CounterHandler {
            count_map: HashMap::new(),
        }
    }
}

// Just forward the packet
#[async_trait::async_trait]
impl PacketHandler for CounterHandler {
    async fn handle_request(&mut self, p: &Packet) -> Packet {
        // Print out the packet
        //debug!("[{}]", String::from_utf8_lossy(&p.bytes));
        debug!(
            "c<=s: {:?} packet: {} bytes",
            p.get_packet_type(),
            p.get_size()
        );

        match p.get_query() {
            Ok(sql) => {
                info!("SQL: {}", sql);
                let tokens: Vec<&str> = sql.split(' ').collect();
                let command = tokens[0].to_lowercase();
                let count = self.count_map.entry(command).or_insert(0);
                *count += 1;
                println!("{:?}", self.count_map);
            }
            Err(e) => debug!("{:?} packet: {}", p.get_packet_type(), e),
        };

        p.clone()
    }

    async fn handle_response(&mut self, p: &Packet) -> Packet {
        debug!(
            "c<=s: {:?} packet: {} bytes",
            p.get_packet_type(),
            p.get_size()
        );

        p.clone()
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let mut args = std::env::args().skip(1);

    info!("Counter proxy... ");

    // determine address for the proxy to bind to
    //let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:3306".to_string());    // MariaDB
    let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:5432".to_string()); // Postgres

    // determine address of the MariaDB instance we are proxying for
    //let db_addr = args.next().unwrap_or_else(|| "postgres-server:3306".to_string());    // MariaDB
    let db_addr = args
        .next()
        .unwrap_or_else(|| "postgres-server:5432".to_string()); // Postgres

    // determine what type of database it is
    let db_type_str = args.next().unwrap_or_else(|| "postgres".to_string());
    let mut db_type = DatabaseType::PostgresSQL;
    if db_type_str == "mariadb" {
        db_type = DatabaseType::MariaDB;
    }

    let mut server =
        sql_proxy::server::Server::new(bind_addr.clone(), db_type, db_addr.clone()).await;

    info!("Proxy listening on: {}", bind_addr);
    let (tx, rx) = oneshot::channel(); // kill switch
    tokio::spawn(async move {
        info!("Proxy listening on: {}", bind_addr);
        server.run(CounterHandler::new(), rx).await;
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
