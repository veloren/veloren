use std::net::TcpStream;
use common::network::packet_handler::{PacketSender, PacketReceiver};

pub struct Session {
    sender: PacketSender,
    receiver: PacketReceiver,
}

impl Session {
    pub fn new(stream: TcpStream) -> Session {
        stream.set_nonblocking(true); // quickfix for client
        Session {
            sender: PacketSender::new(Some(stream.try_clone().unwrap()), None),
            receiver: PacketReceiver::new(Some(stream.try_clone().unwrap()), None),
        }
    }

    pub fn get_sender(&mut self) -> &mut PacketSender { &mut self.sender }
    pub fn get_receiver(&mut self) -> &mut PacketReceiver { &mut self.receiver }
}
