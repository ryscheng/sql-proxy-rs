#[macro_use]
extern crate log;

use std::{collections::HashMap, env};

use mariadb_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};

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
    let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:5432".to_string());      // Postgres

    // determine address of the MariaDB instance we are proxying for
    //let db_addr = args.next().unwrap_or_else(|| "postgres-server:3306".to_string());    // MariaDB 
    let db_addr = args.next().unwrap_or_else(|| "postgres-server:5432".to_string());      // Postgres
    
    // determine what type of database it is
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

    info!("Proxy listening on: {}", bind_addr);
    server.run(CounterHandler::new()).await;
}
