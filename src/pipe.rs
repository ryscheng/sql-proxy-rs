use byteorder::{BigEndian, ByteOrder};
use futures::{
    channel::mpsc::{Sender, Receiver},
    lock::Mutex,
    select,
    FutureExt,
    StreamExt,
};
//use futures_util::{
//    future::FutureExt,
//    stream::StreamExt,
//};
use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt, Result};

use crate::{
    packet::{DatabaseType, Packet},
    packet_handler::{Direction, PacketHandler},
};

pub struct Pipe<T: AsyncReadExt, U: AsyncWriteExt> {
    name: String,
    db_type: DatabaseType,
    packet_handler: Arc<Mutex<dyn PacketHandler + Send>>,
    direction: Direction,
    source: T,
    sink: U,
}

impl<T: AsyncReadExt + Unpin, U: AsyncWriteExt + Unpin> Pipe<T, U> {
    pub fn new(
        name: String,
        db_type: DatabaseType,
        packet_handler: Arc<Mutex<dyn PacketHandler + Send>>,
        direction: Direction,
        reader: T,
        writer: U,
    ) -> Pipe<T, U> {
        Pipe {
            name,
            db_type,
            packet_handler,
            direction,
            source: reader,
            sink: writer,
        }
    }

    pub async fn run(&mut self, other_pipe_sender: Sender<Packet>, other_pipe_receiver: Receiver<Packet>) -> Result<()> {
        trace!("[{}]: Running {:?} pipe loop...", self.name, self.direction);
        //let source = Arc::get_mut(&mut self.source).unwrap();
        //let sink = Arc::get_mut(&mut self.sink).unwrap();
        let mut other_pipe_receiver = other_pipe_receiver.into_future().fuse();
        let mut read_buf: Vec<u8> = vec![0_u8; 4096];
        let mut packet_buf: Vec<u8> = Vec::with_capacity(4096);
        let mut write_buf: Vec<u8> = Vec::with_capacity(4096);

        loop {
            select! {
                // Read from the source to read_buf, append to packet_buf
                result = self.source.read(&mut read_buf[..]).fuse() => {
                    match result {
                        Ok(n) => {
                            //let n = self.source.read(&mut read_buf[..]).await?;
                            if n == 0 {
                                let e = self.create_error(format!("Read {} bytes, closing pipe.", n));
                                warn!("{}", e.to_string());
                                return Err(e);
                            }
                            self.trace(format!("{} bytes read from source", n));
                            packet_buf.extend_from_slice(&read_buf[0..n]);

                            // Process all packets in packet_buf, put into write_buf
                            while let Some(packet) = get_packet(self.db_type, &mut packet_buf) {
                                self.trace("Processing packet".to_string());
                                {
                                    // Scope for self.packet_handler Mutex
                                    let mut h = self.packet_handler.lock().await;
                                    let transformed_packet: Packet = match self.direction {
                                        Direction::Forward => h.handle_request(&packet).await,
                                        Direction::Backward => h.handle_response(&packet).await,
                                    };
                                    write_buf.extend_from_slice(&transformed_packet.bytes);
                                }
                            } // end while

                        },
                        Err(e) => {
                            warn!("[{}:{:?}]: Error reading from source", self.name, self.direction);
                            return Err(e);
                        },
                    }; // end match
                },
                // Support short-circuit
                (packet, recv) = other_pipe_receiver => {
                    self.process_short_circuit(packet, &mut write_buf)?;
                    other_pipe_receiver = recv.into_future().fuse();
                },
            } // end select!
            

            // Write all to sink
            let n = self.sink.write(&write_buf[..]).await?;
            let _: Vec<u8> = write_buf.drain(0..n).collect();
            self.trace(format!("{} bytes written to sink", n));
        } // end loop
    } // end fn run

    fn process_short_circuit(&self, packet: Option<Packet>, write_buf: &mut Vec<u8>) -> Result<()> {
        if let Some(p) = packet {
            self.trace(format!("Got short circuit packet of {} bytes", p.get_size()));
            write_buf.extend_from_slice(&p.bytes);
            Ok(())
        } else {
            let e = self.create_error("other_pipe_receiver prematurely closed".to_string());
            warn!("{}", e.to_string());
            Err(e)
        }
    }

    fn trace(&self, string: String) {
        trace!(
            "[{}:{:?}]: {}",
            self.name,
            self.direction,
            string
        );
    }

    fn create_error(&self, string: String) -> Error {
        Error::new(
            ErrorKind::Other,
            format!(
                "[{}:{:?}]: {}",
                self.name, self.direction, string
            ),
        )
    }
} // end impl

fn get_packet(db_type: DatabaseType, packet_buf: &mut Vec<u8>) -> Option<Packet> {
    match db_type {
        DatabaseType::MariaDB => {
            // Check for header
            if packet_buf.len() > 3 {
                let l: usize = (((packet_buf[2] as u32) << 16)
                    | ((packet_buf[1] as u32) << 8)
                    | packet_buf[0] as u32) as usize;
                let s = 4 + l;
                // Check for entire packet size
                if packet_buf.len() >= s {
                    let p = Packet::new(DatabaseType::MariaDB, packet_buf.drain(0..s).collect());
                    Some(p)
                } else {
                    None
                }
            } else {
                None
            }
        }
        DatabaseType::PostgresSQL => {
            if packet_buf.len() > 5 {
                let list = [
                    'R', 'K', 'B', '2', '3', 'C', 'd', 'c', 'f', 'G', 'H', 'W', 'D', 'I', 'E', 'F',
                    'V', 'p', 'v', 'n', 'N', 'A', 't', 'S', 'P', '1', 's', 'Q', 'Z', 'T', 'X',
                ];
                let id = packet_buf[0] as char;

                if list.contains(&id) {
                    let l = BigEndian::read_u32(&packet_buf[1..5]) as usize;
                    let s = 1 + l;
                    trace!(
                        "get_packet(PostgresSQL): type={}, size={}, length={}",
                        id,
                        s,
                        l
                    );
                    // Check for entire packet size
                    if packet_buf.len() >= s {
                        let p = Packet::new(
                            DatabaseType::PostgresSQL,
                            packet_buf.drain(0..s).collect(),
                        );
                        Some(p)
                    } else {
                        None
                    }
                } else {
                    let l = BigEndian::read_u32(&packet_buf[0..4]) as usize;
                    let s = l;
                    trace!(
                        "get_packet(PostgresSQL): firstbyte={:#04x}, size={}, length={}",
                        packet_buf[0],
                        s,
                        l
                    );
                    // Check for entire packet size
                    if packet_buf.len() >= s {
                        let p = Packet::new(
                            DatabaseType::PostgresSQL,
                            packet_buf.drain(0..s).collect(),
                        );
                        Some(p)
                    } else {
                        None
                    }
                }
            } else {
                None
            }
        }
    }
}
