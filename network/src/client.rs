use std::io;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use packet::{ClientPacket, ServerPacket};


pub struct ClientConn {
    sock: UdpSocket,
}

impl ClientConn {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(bind_addr: T, remote_addr: U) -> io::Result<ClientConn> {
        let sock = UdpSocket::bind(bind_addr)?;
        sock.connect(remote_addr)?;

        Ok(ClientConn {
            sock,
        })
    }

    pub fn clone(&self) -> ClientConn {
        ClientConn {
            sock: self.sock.try_clone().unwrap(),
        }
    }

    pub fn send(&self, pack: &ClientPacket) -> bool {
        match pack.serialize() {
            Some(ref data) => self.sock.send(data).is_ok(),
            None => false,
        }
    }

    pub fn recv(&self) -> (SocketAddr, ServerPacket) {
        let mut buff = [0; 1024];
        loop {
            match self.sock.recv_from(&mut buff) {
                Ok((_, addr)) => match ServerPacket::from(&buff) {
                    Some(packet) => return (addr, packet),
                    _ => {},
                },
                Err(_) => {}, // TODO: Handle errors properly
            }
        }
    }
}
