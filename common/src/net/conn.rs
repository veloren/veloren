use std::sync::Mutex;
use std::io::{Write, Read};
use std::net::{TcpStream, ToSocketAddrs};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::Error;
use super::Packet;

pub struct Conn {
    stream_in: Mutex<TcpStream>,
    stream_out: Mutex<TcpStream>,
}

impl Conn {
    pub fn new<A: ToSocketAddrs>(remote: A) -> Result<Conn, Error> {
        let stream = TcpStream::connect(remote)?;
        stream.set_nodelay(true)?;
        Ok(Conn {
            stream_in: Mutex::new(stream.try_clone()?),
            stream_out: Mutex::new(stream),
        })
    }

    pub fn send<P: Packet>(&self, packet: P) -> Result<(), Error> {
        let data = packet.serialize()?;
        let mut stream = self.stream_out.lock().unwrap();
        stream.write_u32::<LittleEndian>(data.len() as u32)?;
        Ok(stream.write_all(&data)?)
    }

    pub fn recv<P: Packet>(&self) -> Result<P, Error> {
        let mut stream = self.stream_in.lock().unwrap();
        let packet_size = stream.read_u32::<LittleEndian>()? as usize;
        let mut buff = vec![0; packet_size];
        stream.read_exact(&mut buff)?;
        Ok(P::from(&buff)?)
    }
}
