use std::io;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use packet::{ClientPacket, ServerPacket};

pub struct ServerConn {
    bind_addr: SocketAddr,
    sock: UdpSocket,
    players: Vec<Arc<PlayerHandle>>,
}

impl ServerConn {
    pub fn new<A: ToSocketAddrs>(bind_addr: A) -> io::Result<ServerConn> {
        Ok(ServerConn {
            bind_addr: bind_addr.to_socket_addrs()?.next().unwrap(),
            sock: UdpSocket::bind(bind_addr)?,
            players: Vec::new(),
        })
    }

    pub fn recv(&mut self) -> (SocketAddr, ClientPacket) {
        let mut buff = [0; 1024];
        loop {
            match self.sock.recv_from(&mut buff) {
                Ok((_, addr)) => match ClientPacket::from(&buff) {
                    Some(packet) => return (addr, packet),
                    _ => {},
                },
                Err(_) => {}, // TODO: Handle errors properly
            }
        }
    }

    pub fn send_to<A: ToSocketAddrs>(&self, tgt_addr: A, pack: ServerPacket) -> bool{
        match pack.serialize() {
            Some(ref data) => self.sock.send_to(data, tgt_addr).is_ok(),
            None => false,
        }
    }
}

pub struct PlayerHandle {
    alias: String,
    sock: UdpSocket,
}

impl PlayerHandle {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(alias: &str, bind_addr: T, addr: U) -> io::Result<PlayerHandle> {
        let sock = UdpSocket::bind(bind_addr)?;
        sock.connect(addr)?;

        Ok(PlayerHandle {
            alias: alias.to_string(),
            sock,
        })
    }

    pub fn send(&self, pack: ServerPacket) -> bool{
        match pack.serialize() {
            Some(ref data) => self.sock.send(data).is_ok(),
            None => false,
        }
    }
}
