extern crate mariadb_proxy;

extern crate env_logger;
extern crate futures;
extern crate futures_util;
#[macro_use] extern crate log;
extern crate tokio;

use std::env;
use futures_util::future::FutureExt;
use hyper::client::{Client, HttpConnector};
use hyper::body::{Body};
use mariadb_proxy::packet::{Packet, PacketType};
use mariadb_proxy::packet_handler::{PacketHandler};

struct CounterHandler {
  http_client: Client<HttpConnector, Body>,
}

// Just forward the packet
impl PacketHandler for CounterHandler {

  fn handle_request(&mut self, p: &Packet) -> Packet {
    // Print out the packet
    //debug!("[{}]", String::from_utf8_lossy(&p.bytes));

    match p.packet_type() {
      Ok(PacketType::ComQuery) => {
        let payload = &p.bytes[5..];
        let sql = String::from_utf8(payload.to_vec()).expect("Invalid UTF-8");
        info!("SQL: {}", sql);
        let mut url: String = "http://localhost:26657/broadcast_tx_commit?tx=".to_owned();
        url.push_str(&sql);
        info!("Pushing to Tendermint: {}", url);
        let _fut = self.http_client.get(url.parse().unwrap()).then(|res| async move {
          let response = res.unwrap();
          debug!("Response: {}", response.status());
          debug!("Headers: {:#?}\n", response.headers());
        });

      },
      _ => {
        debug!("{:?} packet", p.packet_type())
      },
    }

    Packet { bytes: p.bytes.clone() }
  }

  fn handle_response(&mut self, p: &Packet) -> Packet {
    Packet { bytes: p.bytes.clone() }
  }

}

#[tokio::main]
async fn main() {
  env_logger::init();

  info!("Tendermint MariaDB proxy... ");

  // determine address for the proxy to bind to
  let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:3306".to_string());
  // determine address of the MariaDB instance we are proxying for
  let db_addr = env::args().nth(2).unwrap_or("mariadb:3306".to_string());

  let mut server = mariadb_proxy::server::Server::new(bind_addr.clone(), db_addr.clone()).await;
  info!("Proxy listening on: {}", bind_addr);
  server.run(CounterHandler { http_client: Client::new() }).await;
}

