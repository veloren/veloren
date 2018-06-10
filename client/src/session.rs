use std::net::TcpStream;
use common::network::Error;
use common::network::packet_handler::PacketHandler;

pub struct Session {
    handler: PacketHandler,
}

impl Session {
    pub fn new(stream: TcpStream) -> Session {
        Session {
            handler: PacketHandler::new(Some(stream), None),
        }
    }

    pub fn get_handler(&mut self) -> &mut PacketHandler { &mut self.handler }
}
