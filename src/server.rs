use std::sync::Arc;
use futures::{
    channel::oneshot::Receiver,
    future::FutureExt, // for `.fuse()`
    lock::Mutex,
    select,
    stream::StreamExt,
    try_join
};
use tokio::net::{TcpListener, TcpStream};

use crate::{
    packet::DatabaseType,
    packet_handler::{Direction, PacketHandler},
    pipe::Pipe,
};

#[derive(Debug)]
pub struct Server {
    db_type: DatabaseType,
    db_addr: String,
    listener: TcpListener,
}

impl Server {
    pub async fn new(bind_addr: String, db_type: DatabaseType, db_addr: String) -> Server {
        Server {
            db_type,
            db_addr,
            listener: TcpListener::bind(bind_addr)
                .await
                .expect("Unable to bind to bind_addr"),
        }
    }

    async fn create_pipes<T: PacketHandler + Send + Sync + 'static>(db_addr: String, db_type: DatabaseType, mut client_socket: TcpStream, handler_ref: Arc<Mutex<T>>) {
        let client_addr = match client_socket.peer_addr() {
            Ok(addr) => addr.to_string(),
            Err(_e) => String::from("Unknown"),
        };
        debug!("Accepted connection from {}", client_addr);
        tokio::spawn(async move {
            let (client_reader, client_writer) = client_socket.split();
            let mut server_socket = TcpStream::connect(db_addr.clone())
                .await
                .unwrap_or_else(|_| {
                    panic!("Connecting to SQL database ({}) failed", db_addr)
                });
            let (server_reader, server_writer) = server_socket.split();
            let mut forward_pipe = Pipe::new(
                client_addr.clone(),
                db_type,
                handler_ref.clone(),
                Direction::Forward,
                client_reader,
                server_writer,
            );
            let mut backward_pipe = Pipe::new(
                client_addr.clone(),
                db_type,
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

    pub async fn run<T: PacketHandler + Send + Sync + 'static>(&mut self, packet_handler: T) {
        let db_addr = self.db_addr.clone();
        let db_type = self.db_type;
        let packet_handler = Arc::new(Mutex::new(packet_handler));
        let mut incoming = self.listener.incoming();
        while let Some(conn) = incoming.next().await {
            match conn {
                Ok(mut client_socket) => {
                    Server::create_pipes(db_addr.clone(), db_type, client_socket, packet_handler.clone());
                },
                Err(err) => {
                    // Handle error by printing to STDOUT.
                    error!("accept error = {:?}", err);
                },
            };
        } // end loop
        info!("Server run() complete");
    }
}
