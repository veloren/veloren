
use network::packet::{Packet};

use std::io::{Read, Write};
use std::net::TcpStream;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::cell::RefCell;
use std::io::Error;

pub struct PacketReceiver {
    stream: RefCell<TcpStream>
}

impl PacketReceiver {
    pub fn new(stream: TcpStream) -> PacketReceiver {
        PacketReceiver {
            stream: RefCell::new(stream)
        }
    }

    pub fn clone_stream(&self) -> TcpStream { self.stream.borrow_mut().try_clone().unwrap() }

    pub fn recv_packet<P: Packet>(&self) -> Result<P, Error> {
        let mut stream = self.stream.borrow_mut();
        let size = stream.read_u32::<LittleEndian>()?;
        let mut data: Vec<u8> = Vec::with_capacity(size as usize);
        data.resize(size as usize, 0);
        stream.read_exact(data.as_mut())?;
        Ok(P::from(&data).unwrap())
    }
}

pub struct PacketSender {
    stream: RefCell<TcpStream>
}

impl PacketSender {
    pub fn new(stream: TcpStream) -> PacketSender {
        PacketSender {
            stream: RefCell::new(stream)
        }
    }

    pub fn clone_stream(&self) -> TcpStream { self.stream.borrow_mut().try_clone().unwrap() }

    pub fn send_packet<P: Packet>(&self, packet: &P) -> Result<(), Error> {
        let mut stream = self.stream.borrow_mut();
        let data = packet.serialize().unwrap();
        stream.write_u32::<LittleEndian>(data.len() as u32)?;
        stream.write_all(&data)?;
        Ok(())
    }
}