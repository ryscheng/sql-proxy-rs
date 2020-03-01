#[macro_use]
extern crate log;

use mariadb_proxy::{
    packet::{DatabaseType, Packet},
    packet_handler::PacketHandler,
};

struct PassthroughHandler {}

// Just forward the packet
impl PacketHandler for PassthroughHandler {
    fn handle_request(&mut self, p: &Packet) -> Packet {
        p.clone()
    }

    fn handle_response(&mut self, p: &Packet) -> Packet {
        p.clone()
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    info!("Passthrough MariaDB proxy... ");

    let mut args = std::env::args().skip(1); // skip program name
                                             // determine address for the proxy to bind to
    let bind_addr = args.next().unwrap_or_else(|| "0.0.0.0:3306".to_string());
    // determine address of the MariaDB instance we are proxying for
    let db_addr = args.next().unwrap_or_else(|| "postgres:3306".to_string());

    let mut server =
        mariadb_proxy::server::Server::new(bind_addr.clone(), DatabaseType::PostgresSQL, db_addr)
            .await;
    info!("Proxy listening on: {}", bind_addr);
    server.run(PassthroughHandler {}).await;
}
