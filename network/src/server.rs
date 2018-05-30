use std::{io, thread, cmp, hash};
use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr};
use std::collections::HashSet;
use packet::{ClientPacket, ServerPacket, Serialize};

pub struct ServerConn {
    sock: UdpSocket,
    players: HashSet<PlayerHandle>,
}

impl ServerConn {
    pub fn new(bind_addr: &str) -> io::Result<ServerConn> {
        Ok(ServerConn {
            sock: UdpSocket::bind(bind_addr)?,
            players: HashSet::new(),
        })
    }

    pub fn listen(&mut self) {
        let mut buff: [u8; 256] = [0; 256];
        self.sock.recv_from(&mut buff);
    }

    pub fn send(&self, addr: &str, pack: ServerPacket) -> bool{
        match pack.serialize() {
            Some(ref data) => self.sock.send(data).is_ok(),
            None => false,
        }
    }
}

pub struct PlayerHandle {
    id: u32,
    sock: UdpSocket,
}

impl cmp::Eq for PlayerHandle {}

impl hash::Hash for PlayerHandle {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl cmp::PartialEq for PlayerHandle {
    fn eq(&self, other: &PlayerHandle) -> bool { self.id == other.id }
}
