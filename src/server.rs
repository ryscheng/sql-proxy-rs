use std::sync::Arc;
use std::rc::Rc;
use futures::{
    channel::oneshot,
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
    kill_switches: Vec<oneshot::Sender<()>>,
}

impl Server {
    pub async fn new(bind_addr: String, db_type: DatabaseType, db_addr: String) -> Server {
        Server {
            db_type: db_type,
            db_addr: db_addr,
            listener: TcpListener::bind(bind_addr).await.expect("Unable to bind to bind_addr"),
            kill_switches: Vec::new(),
        }
    }

    async fn create_pipes<T: PacketHandler + Send + Sync + 'static>(db_addr: String, db_type: DatabaseType, mut client_socket: TcpStream, handler_ref: Arc<Mutex<T>>, kill_switch_receiver: oneshot::Receiver<()>) {
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
            select! {
                _ = forward_pipe.run().fuse() => {
                    trace!("Pipe closed via forward pipe");
                },
                _ = backward_pipe.run().fuse() => {
                    trace!("Pipe closed via backward pipe");
                },
                _ = kill_switch_receiver.fuse() => {
                    trace!("Pipe closed via kill switch");
                }
            }
            debug!("Closing connection from {:?}", client_socket.peer_addr());
        });

    }

    pub async fn run<T: PacketHandler + Send + Sync + 'static>(&mut self, packet_handler: T, kill_switch_receiver: oneshot::Receiver<()>) {
        let db_addr = self.db_addr.clone();
        let db_type = self.db_type;
        let packet_handler = Arc::new(Mutex::new(packet_handler));
        let mut incoming = self.listener.incoming().fuse();
        let mut kill_switch_receiver = kill_switch_receiver.fuse();
        loop {
        //while let Some(conn) = incoming.next().await {
            select! {
                some_conn = incoming.next() => {
                    if let Some(conn) = some_conn {
                        match conn {
                            Ok(mut client_socket) => {
                                let (tx, rx) = oneshot::channel();
                                self.kill_switches.push(tx);
                                Server::create_pipes(db_addr.clone(), db_type, client_socket, packet_handler.clone(), rx);
                            },
                            Err(err) => {
                                // Handle error by printing to STDOUT.
                                error!("accept error = {:?}", err);
                            },
                        };
                    } else {
                        debug!("Server.run() accept completed. no more incoming connections");
                        break;
                    }
                },
                _ = kill_switch_receiver => {
                    info!("Received a kill switch at the server");
                    // Kill all pipes
                    while let Some(s) = self.kill_switches.pop() {
                        s.send(());
                    }
                    break
                },
            }
        } // end loop
        info!("Server run() complete");
    }
}
