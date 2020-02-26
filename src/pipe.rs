
use std::sync::{Arc, Mutex};
use tokio::net::tcp::{ReadHalf, WriteHalf};
use tokio::net::{TcpStream};
use crate::packet_handler::PacketHandler;

pub struct Pipe {
  packet_handler: Arc<Mutex<dyn PacketHandler+Send>>,
  reader: Arc<TcpStream>,
  writer: Arc<TcpStream>,
  packet_buf: Vec<u8>,
  read_buf: Vec<u8>,
  write_buf: Vec<u8>,
}

impl Pipe {
  pub fn new(packet_handler: Arc<Mutex<dyn PacketHandler+Send>>, reader: Arc<TcpStream>, writer: Arc<TcpStream>) -> Pipe {
    Pipe {
      packet_handler: packet_handler,
      reader: reader,
      writer: writer,
      packet_buf: Vec::with_capacity(4096),
      read_buf: vec![0_u8; 4096],
      write_buf: Vec::with_capacity(4096),
    }
  }
  pub async fn run(&self) {
  }
}
