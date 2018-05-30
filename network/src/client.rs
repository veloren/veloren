use std::io;
use std::net::UdpSocket;
use packet::{ClientPacket, ServerPacket};

pub struct ClientConn {
    sock: UdpSocket,
}

impl ClientConn {
    pub fn new(bind_addr: &str, remote_addr: &str) -> io::Result<ClientConn> {
        let sock = UdpSocket::bind(bind_addr)?;
        sock.connect(remote_addr)?;

        Ok(ClientConn {
            sock,
        })
    }

    pub fn send(&self, pack: ClientPacket) -> bool {
        match pack.serialize() {
            Some(ref data) => self.sock.send(data).is_ok(),
            None => false,
        }
    }

    pub fn recv(&self) -> Option<ServerPacket> {
        let mut data = [0; 4096];
        match self.sock.recv(&mut data) {
            Ok(_) => ServerPacket::from(&data),
            Err(_) => None, // TODO: Handle error?
        }
    }
}
