
use crate::packet::Packet;

/// Packet handlers need to implement this trait
pub trait PacketHandler {
  fn handle_request(&mut self, p: &Packet) -> Packet;
  fn handle_response(&mut self, p: &Packet) -> Packet;
}
