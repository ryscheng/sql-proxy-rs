use crate::packet::Packet;

#[derive(Debug)]
pub enum Direction {
    Forward,  // corresponds to handle_request
    Backward, // corresponds to handle_response
}

/// Packet handlers need to implement this trait
pub trait PacketHandler {
    fn handle_request(&mut self, p: &Packet) -> Packet;
    fn handle_response(&mut self, p: &Packet) -> Packet;
}
