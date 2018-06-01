use std::io;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use packet::{ClientPacket, ServerPacket};


pub struct ServerConn {
    sock: UdpSocket,
}

impl ServerConn {
    pub fn new<A: ToSocketAddrs>(bind_addr: A) -> io::Result<ServerConn> {
        Ok(ServerConn {
            sock: UdpSocket::bind(bind_addr)?,
        })
    }

    pub fn clone(&self) -> ServerConn {
        ServerConn {
            sock: self.sock.try_clone().unwrap(),
        }
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

    pub fn send_to<A: ToSocketAddrs>(&self, sock_addr: A, pack: &ServerPacket) -> bool{
        match pack.serialize() {
            Some(ref data) => self.sock.send_to(data, sock_addr).is_ok(),
            None => false,
        }
    }
}
