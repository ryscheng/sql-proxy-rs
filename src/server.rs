
use futures::stream::StreamExt;
use tokio::net::TcpListener;

use crate::packet_handler::PacketHandler;

#[derive(Debug)]
pub struct Server {
  listener: TcpListener
}

impl Server {
  pub async fn new(bind_addr: String, db_addr: String) -> Server {
    Server { 
      listener: TcpListener::bind(bind_addr).await.unwrap()
    }
  }

  pub async fn run(&mut self, packet_handler: &PacketHandler) {
    let mut incoming = self.listener.incoming();
    while let Some(conn) = incoming.next().await {
      match conn {
        Ok(mut client_socket) => {
          info!("Accepted connection from {:?}", client_socket.peer_addr());
          // TODO: create db connection
          tokio::spawn(async move {
            let (mut client_reader, mut client_writer) = client_socket.split();

            match tokio::io::copy(&mut client_reader, &mut client_writer).await {
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


  }
}
