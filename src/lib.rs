#![recursion_limit = "512"]

#[macro_use]
extern crate log;

pub mod packet;
pub mod packet_handler;
pub mod pipe;
pub mod server;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
