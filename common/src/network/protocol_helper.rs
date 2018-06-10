use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use get_if_addrs;
use network::Error;
use network::packet::ServerPacket;
use network::packet::ClientPacket;

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
            bind_udp(&new_addr)
        },
    }
}
