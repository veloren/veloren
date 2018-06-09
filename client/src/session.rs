use std::sync::Mutex;
use std::net::TcpStream;
use common::Uid;
use std::thread::JoinHandle;
use std::thread;
use std::time;


use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::io::{Read, Write, ErrorKind};
use common::network::packet::ClientPacket;
use common::network::Error;
use common::network::packet::ServerPacket;

pub struct Session {
    stream: TcpStream,
    //read_buffer: Mutex<Vec<u8>>,
    has_read_size: bool,
    read_size: usize,
}

impl Session {
    pub fn new(stream: TcpStream) -> Session {
        Session {
            stream,
            //read_buffer: Mutex::new(Vec::<u8>::new()),
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

    pub fn recv_packet(&mut self) -> Result<ServerPacket, Error> {/*
        const RECV_BUFFER_SIZE : usize = 100;
        let mut part: Vec<u8> = Vec::with_capacity(RECV_BUFFER_SIZE);
        part.resize(RECV_BUFFER_SIZE, 0);
        println!("beep");
        match self.stream.read(&mut part) {
            Ok(0) => {},
            Ok(s) => {
                println!("{}",s);
                println!("Revc len{:?} {:?}", s, &part);
                ///self.stream.read(buf)
                //self.stream.read_exact(part.as_mut()).unwrap();
                let p = ServerPacket::from(&part);
                return p;
            },
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(Error::NetworkErr(e)),
        }
        println!("boop");
        return Err(Error::MessageInProgress);
        /*
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        if !buf.is_empty() {
            Err(Error::new(ErrorKind::UnexpectedEof,
                           "failed to fill whole buffer"))
        } else {
            Ok(())
        }*/
*/

        match self.stream.read_u32::<LittleEndian>() {
            Ok(s) => {
                // we store the size of the package in case of other bytes are not send yet
                self.read_size = s as usize;
                self.has_read_size = true;
                if (s > 1000) {
                    panic!("something wrong must have happened, we dont have so bug packages yet")
                }
            },
            Err(e) => {},
        }

        if self.has_read_size {
            // we have read size, now we can read the package
            let mut data: Vec<u8> = Vec::with_capacity(self.read_size);
            data.resize(self.read_size, 0);
            match self.stream.read_exact(data.as_mut()) {
                Ok(s) => {},
                Err(e) => {},
            }
            println!("Revc len{:?} {:?}", self.read_size, &data);
            self.has_read_size = false;
            return ServerPacket::from(&data);
        }

        return Err(Error::MessageInProgress);
    }
}
