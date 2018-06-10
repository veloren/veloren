use network::packet::{Packet};

use std::io::{Read, Write};
use std::net::TcpStream;
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Error;

/*
This PacketHandlers abstracts away the underlying tcp_stream or upd_socket
Currently only TCP is used. but input is based on Client or ServerPackets, and theorectically other tcp_streams could be implemented here.
Sending and Receiving is blocking.
*/

enum Interface {
    Tcp,
    Udp,
}

pub struct PacketReceiver {
    tcp_stream: Option<TcpStream>,
    udp_sock: Option<UdpSocket>,
}

impl PacketReceiver {
    pub fn new(tcp_stream: Option<TcpStream>, udp_sock: Option<UdpSocket>) -> PacketReceiver {
        PacketReceiver {
            tcp_stream,
            udp_sock,
        }
    }

    pub fn clone_tcp_stream(&mut self) -> TcpStream { self.tcp_stream.as_mut().unwrap().try_clone().unwrap() }

    pub fn recv_packet<P: Packet>(&mut self) -> Result<P, Error> {
        // not so nice yet
        if self.tcp_stream.is_some() {
            self.recv_tcp_packet()
        } else {
            self.recv_udp_packet()
        }
    }

    pub fn recv_tcp_packet<P: Packet>(&mut self) -> Result<P, Error> {
        let size = self.tcp_stream.as_mut().unwrap().read_u32::<LittleEndian>()?;
        let mut data: Vec<u8> = Vec::with_capacity(size as usize);
        data.resize(size as usize, 0);
        self.tcp_stream.as_mut().unwrap().read_exact(data.as_mut())?;
        Ok(P::from(&data).unwrap())
    }

    pub fn recv_udp_packet<P: Packet>(&mut self) -> Result<P, Error> {
        let mut buff = Vec::with_capacity(1024);
        buff.resize(1024, 0);
        self.udp_sock.as_mut().unwrap().recv_from(&mut buff).unwrap();
        Ok(P::from(&buff).unwrap())
    }

}

pub struct PacketSender {
    tcp_stream: Option<TcpStream>,
    udp_sock: Option<UdpSocket>,
}

impl PacketSender {
    pub fn new(tcp_stream: Option<TcpStream>, udp_sock: Option<UdpSocket>) -> PacketSender {
        PacketSender {
            tcp_stream,
            udp_sock,
        }
    }

    pub fn clone_tcp_stream(&mut self) -> TcpStream { self.tcp_stream.as_mut().unwrap().try_clone().unwrap() }

    fn determine_send_socket<P: Packet>(&self, packet: &P) -> Interface {
        if self.tcp_stream.is_some() {
            Interface::Tcp
        } else {
            Interface::Udp
        }
    }

    pub fn send_packet<P: Packet>(&mut self, packet: &P) -> Result<(), Error> {
        let data = packet.serialize().unwrap();
        match self.determine_send_socket(packet) {
            Interface::Tcp => {
                self.tcp_stream.as_mut().unwrap().write_u32::<LittleEndian>(data.len() as u32)?;
                self.tcp_stream.as_mut().unwrap().write_all(&data)?;
                Ok(())
            },
            Interface::Udp => {
                self.udp_sock.as_mut().unwrap().send(&data)?;
                Ok(())
            },
        }

    }
}
