use std::sync::Mutex;
use std::io::{Write, Read};
use std::net::{TcpStream, ToSocketAddrs};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::Error;
use super::packet::Packet;

pub struct Conn {
    stream_in: Mutex<TcpStream>,
    stream_out: Mutex<TcpStream>,
}

impl Conn {
    pub fn new<A: ToSocketAddrs>(remote: A) -> Result<Conn, Error> {
        let stream = TcpStream::connect(remote)?;
        Ok(Conn {
            stream_in: Mutex::new(stream.try_clone()?),
            stream_out: Mutex::new(stream),
        })
    }

    pub fn send<P: Packet>(&mut self, packet: P) -> Result<(), Error> {
        let data = packet.serialize()?;
        let mut stream = self.stream_out.lock().unwrap();
        stream.write_u32::<LittleEndian>(data.len() as u32)?;
        Ok(stream.write_all(&data)?)
    }

    pub fn recv<P: Packet>(&mut self, packet: P) -> Result<P, Error> {
        let data = packet.serialize()?;
        let mut stream = self.stream_out.lock().unwrap();
        let mut buff = Vec::with_capacity(stream.read_u32::<LittleEndian>()? as usize);
        stream.read_exact(buff.as_mut_slice())?;
        Ok(P::from(&buff)?)
    }
}
