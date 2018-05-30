use std::io;
use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr};
use packet::{ClientPacket, ServerPacket, Serialize};

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
}
