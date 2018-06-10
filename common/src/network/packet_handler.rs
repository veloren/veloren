use network::packet_handler::Interface::{Tcp, Udp};
use network::Error::NetworkErr;
use network::Error;
use std::sync::Mutex;
use std::net::TcpStream;
use Uid;
use std::thread::JoinHandle;
use std::thread;
use std::time;


use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write, ErrorKind};
use network::packet::{Packet, ClientPacket, ServerPacket};
use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};

/*
This PacketHandler abstracts away the underlying tcp_stream or tcp_streams.alloc
Currently only TCP is used. but input is based on Client or ServerPackets, and theorectically other tcp_streams could be implemented here.
Sending and Receiving is non-blocking. That means, if a package is not fully received yet the PacketHandler will buffer it internally
and sends an Error Event MessageInProgress. That means you as a client should wait for a few milliseconds and then try it again.
*/

enum Interface {
    Tcp,
    Udp,
}

pub struct PacketHandler {
    tcp_stream: Option<TcpStream>,
    udp_sock: Option<UdpSocket>,
    has_read_size: bool,
    read_size: usize,
}

impl PacketHandler {
    pub fn new(tcp_stream: Option<TcpStream>, udp_sock: Option<UdpSocket>) -> PacketHandler {
        if tcp_stream.is_none() && udp_sock.is_none() {
            panic!("Neither TCP nor UDP socket assigned");
        }
        if let Some(tcp_stream) = &tcp_stream {
            tcp_stream.set_nonblocking(true);
        }
        if let Some(udp_sock) = &udp_sock {
            udp_sock.set_nonblocking(true);
        }

        PacketHandler {
            tcp_stream,
            udp_sock,
            has_read_size: false,
            read_size: 0,
        }
    }

    fn determine_send_socket<P: Packet>(&self, packet: &P) -> Interface {
        if self.tcp_stream.is_some() {
            Tcp
        } else {
            Udp
        }
    }

    pub fn send_packet<P: Packet>(&mut self, packet: &P) -> Result<(), Error> {
        let data = packet.serialize()?;
        match self.determine_send_socket(packet) {
            Tcp => {
                self.tcp_stream.as_mut().unwrap().write_u32::<LittleEndian>(data.len() as u32);
                debug!("Send len{:?} {:?}", data.len(), &data);
                self.tcp_stream.as_mut().unwrap().write_all(&data);
                Ok(())
            },
            Udp => {
                panic!("not implemented yet");
                /*
                pub fn send(&self, pack: &ClientPacket) -> Result<(), Error> {
                    match pack.serialize() {
                        Ok(ref data) => { self.sock.send(data)?; Ok(()) },
                        Err(e) => Err(e),
                    }
                }*/
            },
        }

    }

    pub fn recv_packet<P: Packet>(&mut self) -> Result<P, Error> {
        if self.tcp_stream.is_some() {
            let tcp = self.recv_tcp_packet();
            if tcp.is_err() {
                let udp = self.recv_udp_packet();
                return udp;
            } else {
                return tcp;
            }
        } else {
            let udp = self.recv_udp_packet();
            return udp;
        }
    }

    fn recv_tcp_packet<P: Packet>(&mut self) -> Result<P, Error> {
        if !self.has_read_size {
            match self.tcp_stream.as_mut().unwrap().read_u32::<LittleEndian>() {
                Ok(s) => {
                    // we store the size of the package in case of other bytes are not send yet
                    self.read_size = s as usize;
                    self.has_read_size = true;
                    if (s > 10000) {
                        panic!("something wrong must have happened, we dont have so bug packages yet")
                    }
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {return Err(Error::MessageInProgress);}
                Err(e) => {return Err(NetworkErr(e))},
            }
        }

        if self.has_read_size {
            // we have read size, now we can read the package
            let mut data: Vec<u8> = Vec::with_capacity(self.read_size);
            data.resize(self.read_size, 0);
            match self.tcp_stream.as_mut().unwrap().read_exact(data.as_mut()) {
                Ok(s) => {},
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {return Err(Error::MessageInProgress);}
                Err(e) => {return Err(NetworkErr(e))},
            }
            debug!("Revc len{:?} {:?}", self.read_size, &data);
            self.has_read_size = false;
            return P::from(&data);
        }

        return Err(Error::MessageInProgress);
    }

    fn recv_udp_packet<P: Packet>(&mut self) -> Result<P, Error> {
        return Err(Error::MessageInProgress);
        /*
        pub fn recv(&self) -> Result<(SocketAddr, ServerPacket), Error> {
            let mut buff = [0; 1024];
            match self.sock.recv_from(&mut buff) {
                Ok((_, addr)) => Ok((addr, ServerPacket::from(&buff)?)),
                Err(e) => Err(Error::NetworkErr(e)),
            }
        }*/
    }
}
