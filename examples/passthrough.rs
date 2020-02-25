extern crate mariadb_proxy;

extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate tokio;

use std::env;
use tokio::prelude::*;
use mariadb_proxy::packet::Packet;
use mariadb_proxy::packet_handler::PacketHandler;

struct PassthroughHandler {}

// Just forward the packet
impl PacketHandler for PassthroughHandler {

  fn handle_request(&mut self, p: &Packet) -> Packet {
    Packet { bytes: p.bytes.clone() }
  }

  fn handle_response(&mut self, p: &Packet) -> Packet {
    Packet { bytes: p.bytes.clone() }
  }

}

#[tokio::main]
async fn main() {
  env_logger::init();

  info!("Passthrough MariaDB proxy... ");

  // determine address for the proxy to bind to
  let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:3306".to_string());
  // determine address of the MariaDB instance we are proxying for
  let db_addr = env::args().nth(2).unwrap_or("127.0.0.1:3306".to_string());

  let mut server = mariadb_proxy::server::Server::new(bind_addr.clone(), db_addr.clone()).await;
  let handler = PassthroughHandler{};
  info!("Proxy listening on: {}", bind_addr);
  server.run(&handler).await;

}
