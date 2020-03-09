use crate::packet::Packet;

#[derive(Debug)]
pub enum Direction {
    Forward,  // corresponds to handle_request
    Backward, // corresponds to handle_response
}

/// Packet handlers need to implement this trait
#[async_trait::async_trait]
pub trait PacketHandler {
    async fn handle_request(&mut self, p: &Packet) -> Packet;
    async fn handle_response(&mut self, p: &Packet) -> Packet;
}
