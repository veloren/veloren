
use network::packet::{Packet};

use std::io::{Read, Write};
use std::net::TcpStream;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::Error;

pub struct PacketReceiver {
    stream: TcpStream
}

impl PacketReceiver {
    pub fn new(stream: TcpStream) -> PacketReceiver {
        PacketReceiver {
            stream
        }
    }

    pub fn clone_stream(&mut self) -> TcpStream { self.stream.try_clone().unwrap() }

    pub fn recv_packet<P: Packet>(&mut self) -> Result<P, Error> {
        let size = self.stream.read_u32::<LittleEndian>()?;
        let mut data: Vec<u8> = Vec::with_capacity(size as usize);
        data.resize(size as usize, 0);
        self.stream.read_exact(data.as_mut())?;
        Ok(P::from(&data).unwrap())
    }
}

pub struct PacketSender {
    stream: TcpStream,
}

impl PacketSender {
    pub fn new(stream: TcpStream) -> PacketSender {
        PacketSender {
           stream,
        }
    }

    pub fn clone_stream(&mut self) -> TcpStream { self.stream.try_clone().unwrap() }

    pub fn send_packet<P: Packet>(&mut self, packet: &P) -> Result<(), Error> {
        let data = packet.serialize().unwrap();
        self.stream.write_u32::<LittleEndian>(data.len() as u32)?;
        self.stream.write_all(&data)?;
        Ok(())
    }
}