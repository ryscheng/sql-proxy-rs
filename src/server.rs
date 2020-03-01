use std::sync::{Arc, Mutex};

use futures::{stream::StreamExt, try_join};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    packet_handler::{Direction, PacketHandler},
    pipe::Pipe,
};

#[derive(Debug)]
pub struct Server {
    db_addr: String,
    listener: TcpListener,
}

impl Server {
    pub async fn new(bind_addr: String, db_addr: String) -> Server {
        Server {
            db_addr,
            listener: TcpListener::bind(bind_addr)
                .await
                .expect("Unable to bind to bind_addr"),
        }
    }

    pub async fn run<T: PacketHandler + Send + Sync + 'static>(&mut self, packet_handler: T) {
        let packet_handler = Arc::new(Mutex::new(packet_handler));
        let mut incoming = self.listener.incoming();
        while let Some(conn) = incoming.next().await {
            match conn {
                Ok(mut client_socket) => {
                    let client_addr = match client_socket.peer_addr() {
                        Ok(addr) => addr.to_string(),
                        Err(_e) => String::from("Unknown"),
                    };
                    debug!("Accepted connection from {}", client_addr);
                    let db_addr = self.db_addr.clone();
                    let handler_ref = packet_handler.clone();
                    tokio::spawn(async move {
                        let (client_reader, client_writer) = client_socket.split();
                        let mut server_socket = TcpStream::connect(db_addr)
                            .await
                            .expect("Connecting to SQL database failed");
                        let (server_reader, server_writer) = server_socket.split();
                        let mut forward_pipe = Pipe::new(
                            client_addr.clone(),
                            handler_ref.clone(),
                            Direction::Forward,
                            client_reader,
                            server_writer,
                        );
                        let mut backward_pipe = Pipe::new(
                            client_addr.clone(),
                            handler_ref.clone(),
                            Direction::Backward,
                            server_reader,
                            client_writer,
                        );
                        match try_join!(forward_pipe.run(), backward_pipe.run()) {
                            Ok(((), ())) => {
                                trace!("Pipe closed successfully");
                            }
                            Err(e) => {
                                error!("Pipe closed with error: {}", e);
                            }
                        };
                        debug!("Closing connection from {:?}", client_socket.peer_addr());
                    });
                }
                Err(err) => {
                    // Handle error by printing to STDOUT.
                    error!("accept error = {:?}", err);
                }
            };
        }
        info!("Server run() complete");
    }
}
