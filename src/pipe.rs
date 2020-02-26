
use std::sync::{Arc, Mutex};
use tokio::net::{TcpStream};
use tokio::io::{AsyncReadExt};
use crate::packet_handler::PacketHandler;

pub struct Pipe {
  name: String,
  packet_handler: Arc<Mutex<dyn PacketHandler+Send>>,
  source: Arc<TcpStream>,
  sink: Arc<TcpStream>,
  packet_buf: Vec<u8>,
  write_buf: Vec<u8>,
}

impl Pipe {

  pub fn new(
      name: String,
      packet_handler: Arc<Mutex<dyn PacketHandler+Send>>,
      reader: Arc<TcpStream>,
      writer: Arc<TcpStream>) -> Pipe {
    Pipe {
      name: name,
      packet_handler: packet_handler,
      source: reader,
      sink: writer,
      packet_buf: Vec::with_capacity(4096),
      write_buf: Vec::with_capacity(4096),
    }
  }

  pub async fn run(&mut self) {
    let mut read_buf = vec![0_u8; 4096];
    let mut source = Arc::get_mut(&mut self.source).unwrap();

    loop {
      // Read from the source
      let n = source.read(&mut read_buf[..]).await;
      let n = match n {
        Ok(size) => size,
        Err(error) => {
          error!("Error reading from {}: {}", self.name, error);
          return;
        }
      };

      if n <= 0 {
        error!("Read {} bytes from {}. Closing pipe.", n, self.name);
        return;
      }
      trace!("{} bytes read", n);
      self.packet_buf.extend_from_slice(&read_buf[0..n]);

      //return;
    }
  }

}
