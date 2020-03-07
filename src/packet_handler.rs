use futures::future::Future;

use crate::packet::Packet;

#[derive(Debug)]
pub enum Direction {
    Forward,  // corresponds to handle_request
    Backward, // corresponds to handle_response
}

/// Packet handlers need to implement this trait
pub trait PacketHandler {
    fn handle_request(&mut self, p: &Packet) -> Box<dyn Future<Output = Packet>+Unpin+Send>;
    fn handle_response(&mut self, p: &Packet) -> Box<dyn Future<Output = Packet>+Unpin+Send>;
}
