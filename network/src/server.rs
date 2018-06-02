use std::io;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use packet::{ClientPacket, ServerPacket};

use Error;

pub struct ServerConn {
    sock: UdpSocket,
}

impl ServerConn {
    pub fn new<A: ToSocketAddrs>(bind_addr: A) -> Result<ServerConn, Error> {
        Ok(ServerConn {
            sock: UdpSocket::bind(bind_addr)?,
        })
    }

    pub fn clone(&self) -> ServerConn {
        ServerConn {
            sock: self.sock.try_clone().unwrap(),
        }
    }

    pub fn recv(&mut self) -> Result<(SocketAddr, ClientPacket), Error> {
        let mut buff = [0; 1024];
        match self.sock.recv_from(&mut buff) {
            Ok((_, addr)) => Ok((addr, ClientPacket::from(&buff)?)),
            Err(e) => Err(Error::NetworkErr(e)),
        }
    }

    pub fn send_to<A: ToSocketAddrs>(&self, sock_addr: A, pack: &ServerPacket) -> Result<(), Error> {
        match pack.serialize() {
            Ok(ref data) => { self.sock.send_to(data, sock_addr)?; Ok(()) },
            Err(e) => Err(e),
        }
    }
}
