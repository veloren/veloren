use std::net::TcpStream;
use common::Uid;
use std::thread::JoinHandle;
use std::thread;


use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write};
use common::network::packet::ClientPacket;
use common::network::Error;
use common::network::packet::ServerPacket;

pub struct Session {
    stream: TcpStream,
}

impl Session {
    pub fn new(stream: TcpStream) -> Session {
        Session {
            stream,
        }
    }

    pub fn send_packet(&mut self, packet: &ClientPacket) -> Result<(), Error> {
        let data = packet.serialize()?;
        self.stream.write_u32::<LittleEndian>(data.len() as u32);
        println!("Send len{:?} {:?}", data.len(), &data);
        self.stream.write_all(&data);
        Ok(())
    }

    pub fn recv_packet(&mut self) -> Result<ServerPacket, Error> {
        let size = self.stream.read_u32::<LittleEndian>().unwrap() as usize;
        let mut data: Vec<u8> = Vec::with_capacity(size);
        data.resize(size, 0);
        println!("Revc len{:?} {:?}", size, &data);
        self.stream.read_exact(data.as_mut()).unwrap();
        ServerPacket::from(&data)
    }
}
