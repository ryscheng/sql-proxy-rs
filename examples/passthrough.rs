#[macro_use]
extern crate log;
extern crate env_logger;

use std::env;
use std::net::{SocketAddr};


fn main() {
  env_logger::init();

  info!("Passthrough MariaDB proxy... ");

  // determine address for the proxy to bind to
  let bind_addr = env::args().nth(1).unwrap_or("127.0.0.1:3306".to_string());
  let bind_addr = bind_addr.parse::<SocketAddr>().unwrap();

  // determine address of the MySQL instance we are proxying for
  let mysql_addr = env::args().nth(2).unwrap_or("127.0.0.1:3306".to_string());
  let mysql_addr = mysql_addr.parse::<SocketAddr>().unwrap();
}
