
use std::sync::{Arc, Mutex};
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::net::{TcpStream};
use crate::packet_handler::PacketHandler;

pub struct Pipe {
  pub packet_handler: Arc<Mutex<PacketHandler>>,
  pub reader: Arc<TcpStream>,
  pub writer: Arc<TcpStream>,
}

impl Pipe {
  
}
