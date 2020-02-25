extern crate env_logger;
extern crate futures;
#[macro_use] extern crate log;
extern crate tokio;

use std::env;
use futures::stream::StreamExt;
use tokio::net::TcpListener;
use tokio::prelude::*;

#[tokio::main]
async fn main() {
  env_logger::init();

  info!("Passthrough MariaDB proxy... ");

  // determine address for the proxy to bind to
  let bind_addr = env::args().nth(1).unwrap_or("0.0.0.0:3306".to_string());
  // determine address of the MySQL instance we are proxying for
  let mysql_addr = env::args().nth(2).unwrap_or("127.0.0.1:3306".to_string());

  let mut listener = TcpListener::bind(bind_addr.clone()).await.unwrap();

  let server = async move {
    let mut incoming = listener.incoming();
    while let Some(conn) = incoming.next().await {
      match conn {
        Ok(mut socket) => {
          info!("Accepted connection from {:?}", socket.peer_addr());
          tokio::spawn(async move {
            let (mut reader, mut writer) = socket.split();

            match tokio::io::copy(&mut reader, &mut writer).await {
              Ok(amt) => {
                println!("wrote {} bytes", amt);
              }
              Err(err) => {
                eprintln!("IO error {:?}", err);
              }
            }
          });
        }
        Err(err) => {
            // Handle error by printing to STDOUT.
            error!("accept error = {:?}", err);
        }
      }
    }
  };

  info!("Proxy listening on: {}", bind_addr);
  server.await;

}
