
use std::sync::{Arc, Mutex};
use tokio::net::{TcpStream};
use tokio::io::{AsyncReadExt};
use crate::packet::Packet;
use crate::packet_handler::{Direction, PacketHandler};

pub struct Pipe {
  name: String,
  packet_handler: Arc<Mutex<dyn PacketHandler+Send>>,
  direction: Direction,
  source: Arc<TcpStream>,
  sink: Arc<TcpStream>,
}

impl Pipe {

  pub fn new(
      name: String,
      packet_handler: Arc<Mutex<dyn PacketHandler+Send>>,
      direction: Direction,
      reader: Arc<TcpStream>,
      writer: Arc<TcpStream>) -> Pipe {
    Pipe {
      name: name,
      packet_handler: packet_handler,
      direction: direction,
      source: reader,
      sink: writer,
    }
  }

  pub async fn run(&mut self) {
    let mut source = Arc::get_mut(&mut self.source).unwrap();
    let mut read_buf: Vec<u8> = vec![0_u8; 4096];
    let mut packet_buf: Vec<u8> = Vec::with_capacity(4096);
    let mut write_buf: Vec<u8> = Vec::with_capacity(4096);

    loop {
      // Read from the source to read_buf, append to packet_buf
      let n = source.read(&mut read_buf[..]).await;
      let n = match n {
        Ok(size) => size,
        Err(error) => {
          error!("Error reading from {}, closing pipe: {}", self.name, error);
          return;
        }
      };

      if n <= 0 {
        error!("Read {} bytes from {}, closing pipe.", n, self.name);
        return;
      }
      trace!("{} bytes read", n);
      packet_buf.extend_from_slice(&read_buf[0..n]);

      // Process all packets in packet_buf, put into write_buf
      while let Some(packet) = get_packet(&mut packet_buf) {
        trace!("Processing packet");

      }


      
      // Write to sink

      //return;
    }
  }

}

fn get_packet(packet_buf: &mut Vec<u8>) -> Option<Packet> {
  // Check for header
  if packet_buf.len() > 3 {
    let l = parse_packet_length(packet_buf);
    let s = 4 + l;
    // Check for entire packet size
    if packet_buf.len() >= s {
      let p = Packet { bytes: packet_buf.drain(0..s).collect() };
      Some(p)
    } else {
      None
    }
  } else {
    None
  }
}

/// Parse the MySQL packet length (3 byte little-endian)
fn parse_packet_length(header: &[u8]) -> usize {
  (((header[2] as u32) << 16) |
    ((header[1] as u32) << 8) |
    header[0] as u32) as usize
}
