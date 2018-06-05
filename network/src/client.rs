use std::io;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use packet::{ClientPacket, ServerPacket};

use Error;

pub struct ClientConn {
    sock: UdpSocket,
}

impl ClientConn {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(bind_addr: T, remote_addr: U) -> Result<ClientConn, Error> {
        let sock = UdpSocket::bind(bind_addr)?;
        sock.connect(remote_addr)?;

        Ok(ClientConn {
            sock,
        })
    }

    pub fn canBindUdp<T: ToSocketAddrs>(bind_addr: T) -> bool {
        let sock = UdpSocket::bind(bind_addr);
        match sock {
            Ok(_) => {true},
            _ => {false},
        }
    }
/*
    pub fn getFreeUdpPort<T: ToSocketAddrs>(bind_addr: T) -> T {
        let sock = UdpSocket::bind(bind_addr);
        match sock {
            Ok(_) => {true},
            _ => {false},
        }
    }*/

    pub fn clone(&self) -> ClientConn {
        ClientConn {
            sock: self.sock.try_clone().unwrap(),
        }
    }

    pub fn recv(&self) -> Result<(SocketAddr, ServerPacket), Error> {
        let mut buff = [0; 1024];
        match self.sock.recv_from(&mut buff) {
            Ok((_, addr)) => Ok((addr, ServerPacket::from(&buff)?)),
            Err(e) => Err(Error::NetworkErr(e)),
        }
    }

    pub fn send(&self, pack: &ClientPacket) -> Result<(), Error> {
        match pack.serialize() {
            Ok(ref data) => { self.sock.send(data)?; Ok(()) },
            Err(e) => Err(e),
        }
    }
}
