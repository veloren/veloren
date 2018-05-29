mod packet;

// Reexports
pub use packet::ServerPacket as ServerPacket;
pub use packet::ClientPacket as ClientPacket;

use std::io;
use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr};
use packet::Serialize;

pub struct ServerConn {
    sock: UdpSocket,
}

impl ServerConn {
    pub fn new(bind_addr: &str, remote_addr: &str) -> io::Result<ServerConn> {
        Ok(ServerConn {
            sock: UdpSocket::bind(bind_addr)?,
        })
    }

    pub fn send(&self, addr: &str, pack: ServerPacket) -> io::Result<()> {
        self.sock.send(&pack.serialize())?;
        Ok(())
    }
}

pub struct ClientHandle {
    addr: SocketAddr,
}

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

    pub fn send(&self, pack: ServerPacket) -> io::Result<()> {
        self.sock.send(&pack.serialize())?;
        Ok(())
    }
}
