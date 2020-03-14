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

    info!("Counter MariaDB proxy... ");

    // determine address for the proxy to bind to
    let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:3306".to_string());
    // determine address of the MariaDB instance we are proxying for
    let db_addr = env::args()
        .nth(2)
        .unwrap_or("postgres-server:3306".to_string());

    let mut server = mariadb_proxy::server::Server::new(
        bind_addr.clone(),
        DatabaseType::PostgresSQL,
        db_addr.clone(),
    )
    .await;

    info!("Proxy listening on: {}", bind_addr);
    server
        .run(CounterHandler {
            count_map: HashMap::new(),
        })
        .await;
}
