use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use get_if_addrs;
use network::Error;
use network::packet::ServerPacket;
use network::packet::ClientPacket;

pub struct ClientConn {
    sock: UdpSocket,
}

impl ClientConn {
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(bind_addr: T, remote_addr: U) -> Result<ClientConn, Error> {
        let sock = ClientConn::bind_udp(&bind_addr)?;
        sock.connect(remote_addr)?;
        // sock.set_nonblocking(true);

        Ok(ClientConn {
            sock,
        })
    }

    pub fn bind_udp<T: ToSocketAddrs>(bind_addr: &T) -> Result<UdpSocket, Error> {
        let sock = UdpSocket::bind(&bind_addr);
        match sock {
            Ok(s) => Ok(s),
            Err(e) => {
                let new_bind = bind_addr.to_socket_addrs()?
                                        .next().unwrap()
                                        .port() + 1;
                let ip = get_if_addrs::get_if_addrs().unwrap()[0].ip();
                let new_addr = SocketAddr::new(
                    ip,
                    new_bind
                );
                println!("Binding local port failed, trying {}", new_addr);
                ClientConn::bind_udp(&new_addr)
            },
        }
    }

    pub fn clone(&self) -> ClientConn {
        ClientConn {
            sock: self.sock.try_clone().unwrap(),
        }
    }
    /*
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
    }*/
}
