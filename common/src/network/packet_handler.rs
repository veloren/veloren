use network::Error;
use std::sync::Mutex;
use std::net::TcpStream;
use Uid;
use std::thread::JoinHandle;
use std::thread;
use std::time;


use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write, ErrorKind};
use network::packet::ClientPacket;
use network::packet::ServerPacket;

/*
This PacketHandler abstracts away the underlying stream or streams.alloc
Currently only TCP is used. but input is based on Client or ServerPackets, and theorectically other streams could be implemented here.
Sending and Receiving is non-blocking. That means, if a package is not fully received yet the PacketHandler will buffer it internally
and sends an Error Event MessageInProgress. That means you as a client should wait for a few milliseconds and then try it again.
*/
pub struct PacketHandler {
    stream: TcpStream,
    has_read_size: bool,
    read_size: usize,
}

impl PacketHandler {
    pub fn new(stream: TcpStream) -> PacketHandler {
        stream.set_nonblocking(true);
        PacketHandler {
            stream,
            has_read_size: false,
            read_size: 0,
        }
    }

    pub fn send_packet(&mut self, packet: &ClientPacket) -> Result<(), Error> {
        let data = packet.serialize()?;
        self.stream.write_u32::<LittleEndian>(data.len() as u32);
        println!("Send len{:?} {:?}", data.len(), &data);
        self.stream.write_all(&data);
        Ok(())
    }

    pub fn send_packet_2(&mut self, packet: &ServerPacket) -> Result<(), Error> {
        let data = packet.serialize()?;
        self.stream.write_u32::<LittleEndian>(data.len() as u32);
        println!("Send len{:?} {:?}", data.len(), &data);
        self.stream.write_all(&data);
        Ok(())
    }

    pub fn recv_packet(&mut self) -> Result<ServerPacket, Error> {
        if !self.has_read_size {
            match self.stream.read_u32::<LittleEndian>() {
                Ok(s) => {
                    // we store the size of the package in case of other bytes are not send yet
                    self.read_size = s as usize;
                    self.has_read_size = true;
                    println!("Recv Size: {}", s);
                    if (s > 1000) {
                        panic!("something wrong must have happened, we dont have so bug packages yet")
                    }
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {return Err(Error::MessageInProgress);}
                Err(e) => {println!("{}", e)},
            }
        }

        if self.has_read_size {
            // we have read size, now we can read the package
            let mut data: Vec<u8> = Vec::with_capacity(self.read_size);
            data.resize(self.read_size, 0);
            match self.stream.read_exact(data.as_mut()) {
                Ok(s) => {},
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {return Err(Error::MessageInProgress);}
                Err(e) => {println!("{}", e)},
            }
            println!("Revc len{:?} {:?}", self.read_size, &data);
            self.has_read_size = false;
            return ServerPacket::from(&data);
        }

        return Err(Error::MessageInProgress);
    }

    pub fn recv_packet_2(&mut self) -> Result<ClientPacket, Error> {
        if !self.has_read_size {
            match self.stream.read_u32::<LittleEndian>() {
                Ok(s) => {
                    // we store the size of the package in case of other bytes are not send yet
                    self.read_size = s as usize;
                    self.has_read_size = true;
                    println!("Recv Size: {}", s);
                    if (s > 1000) {
                        panic!("something wrong must have happened, we dont have so bug packages yet")
                    }
                },
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {return Err(Error::MessageInProgress);}
                Err(e) => {println!("{}", e)},
            }
        }

        if self.has_read_size {
            // we have read size, now we can read the package
            let mut data: Vec<u8> = Vec::with_capacity(self.read_size);
            data.resize(self.read_size, 0);
            match self.stream.read_exact(data.as_mut()) {
                Ok(s) => {},
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {return Err(Error::MessageInProgress);}
                Err(e) => {println!("{}", e)},
            }
            println!("Revc len{:?} {:?}", self.read_size, &data);
            self.has_read_size = false;
            return ClientPacket::from(&data);
        }

        return Err(Error::MessageInProgress);
    }
}
