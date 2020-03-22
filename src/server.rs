use futures::{
    channel::{mpsc, oneshot},
    future::FutureExt,
    lock::Mutex,
    select,
    stream::StreamExt,
};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

use crate::{
    packet::{DatabaseType, Packet},
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
            db_type,
            db_addr,
            listener: TcpListener::bind(bind_addr)
                .await
                .expect("Unable to bind to bind_addr"),
            kill_switches: Vec::new(),
        }
    }

    async fn create_pipes<T: PacketHandler + Send + Sync + 'static>(
        db_addr: String,
        db_type: DatabaseType,
        mut client_socket: TcpStream,
        handler_ref: Arc<Mutex<T>>,
        kill_switch_receiver: oneshot::Receiver<()>,
    ) {
        let client_addr = match client_socket.peer_addr() {
            Ok(addr) => addr.to_string(),
            Err(_e) => String::from("Unknown"),
        };
        tokio::spawn(async move {
            debug!(
                "Server.create_pipes: Spawning new task to manage connection from {}",
                client_addr
            );
            // Create new connections to the server for each client socket
            let mut server_socket = TcpStream::connect(db_addr.clone())
                .await
                .unwrap_or_else(|_| panic!("Connecting to SQL database ({}) failed", db_addr));
            let (server_reader, server_writer) = server_socket.split();
            let (client_reader, client_writer) = client_socket.split();
            // Create channels to short-circuit at the proxy
            // - tx: use to send directly to other's sink
            // - rx: receive and directly dump into sink
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

            let (fb_tx, fb_rx) = mpsc::channel::<Packet>(128);
            let (bf_tx, bf_rx) = mpsc::channel::<Packet>(128);
            trace!("Server.create_pipes: starting forward/backwards pipes");
            // select! will continuously run all futures until one returns
            // - pipes are infinite loops, and never expect to exit unless error
            // - any return will close this connection
            select! {
                _ = forward_pipe.run(fb_tx, bf_rx).fuse() => {
                    trace!("Pipe closed via forward pipe");
                },
                _ = backward_pipe.run(bf_tx, fb_rx).fuse() => {
                    trace!("Pipe closed via backward pipe");
                },
                _ = kill_switch_receiver.fuse() => {
                    trace!("Pipe closed via kill switch");
                }
            }
            debug!("Closing connection from {:?}", client_socket.peer_addr());
        });
    }

    pub async fn run<T: PacketHandler + Send + Sync + 'static>(
        &mut self,
        packet_handler: T,
        kill_switch_receiver: oneshot::Receiver<()>,
    ) {
        trace!("Server.run(): enter");
        let db_addr = self.db_addr.clone();
        let db_type = self.db_type;
        let packet_handler = Arc::new(Mutex::new(packet_handler));
        let mut incoming = self.listener.incoming().fuse();
        let mut kill_switch_receiver = kill_switch_receiver.fuse();
        loop {
            //while let Some(conn) = incoming.next().await {
            trace!("Server.run(): loop starts");
            select! {
                some_conn = incoming.next() => {
                    trace!("Server.run(): new incoming connection");
                    if let Some(conn) = some_conn {
                        match conn {
                            Ok(mut client_socket) => {
                                trace!("Server.run(): got the client_socket");
                                let (tx, rx) = oneshot::channel();
                                self.kill_switches.push(tx);
                                Server::create_pipes(db_addr.clone(), db_type, client_socket, packet_handler.clone(), rx).await;
                            },
                            Err(err) => {
                                // Handle error by printing to STDOUT.
                                error!("Server.run() accept error = {:?}", err);
                            },
                        };
                    } else {
                        debug!("Server.run() accept completed. no more incoming connections");
                        break;
                    }
                },
                _ = kill_switch_receiver => {
                    info!("Server.run(): Received a kill switch at the server");
                    // Kill all pipes
                    let mut i = 0;
                    while let Some(s) = self.kill_switches.pop() {
                        let _ = s.send(());
                        i += 1;
                    }
                    debug!("Server.run(): killed {} pipes", i);
                    break;
                },
            }
        } // end loop
        info!("Server.run() complete");
    }
}
