// Standard
use std::sync::Mutex;
use std::io::{Write, Read};
use std::net::{TcpStream, ToSocketAddrs};

// Library
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

// Parent
use super::protocol::Protocol;
use super::Error;
use super::packet::Frame;

pub struct Tcp {
    stream_in: Mutex<TcpStream>,
    stream_out: Mutex<TcpStream>,
}

impl Tcp {
    pub fn new<A: ToSocketAddrs>(remote: &A) -> Result<Tcp, Error> {
        let stream = TcpStream::connect(&remote)?;
        stream.set_nodelay(true)?;
        Ok(Tcp {
            stream_in: Mutex::new(stream.try_clone()?),
            stream_out: Mutex::new(stream),
        })
    }

    pub fn new_stream(stream: TcpStream) -> Result<Tcp, Error> {
        stream.set_nodelay(true)?;
        Ok(Tcp {
            stream_in: Mutex::new(stream.try_clone()?),
            stream_out: Mutex::new(stream),
        })
    }
}

impl Protocol for Tcp {
    fn send(&self, frame: Frame) -> Result<(), Error> {
        let mut stream = self.stream_out.lock().unwrap();
        match frame {
            Frame::Header{id, length} => {
                stream.write_u8(1)?; // 1 is const for Header
                stream.write_u64::<LittleEndian>(id)?;
                stream.write_u64::<LittleEndian>(length)?;
                Ok(())
            }
            Frame::Data{id, frame_no, data} => {
                stream.write_u8(2)?; // 2 is const for Data
                stream.write_u64::<LittleEndian>(id)?;
                stream.write_u64::<LittleEndian>(frame_no)?;
                stream.write_u64::<LittleEndian>(data.len() as u64)?;
                stream.write_all(&data)?;
                Ok(())
            }
        }
    }

    //blocking
    fn recv(&self) -> Result<Frame, Error> {
        let mut stream = self.stream_in.lock().unwrap();
        let frame = stream.read_u8()? as u8;
        match frame {
            1 => {
                let id = stream.read_u64::<LittleEndian>()? as u64;
                let length = stream.read_u64::<LittleEndian>()? as u64;
                Ok(Frame::Header{
                    id,
                    length,
                })
            },
            2 => {
                let id = stream.read_u64::<LittleEndian>()? as u64;
                let frame_no = stream.read_u64::<LittleEndian>()? as u64;
                let packet_size = stream.read_u64::<LittleEndian>()? as u64;
                let mut data = vec![0; packet_size as usize];
                stream.read_exact(&mut data)?;
                Ok(Frame::Data{
                    id,
                    frame_no,
                    data,
                })
            },
            x => {
                error!("invalid frame recieved: {}", x);
                Err(Error::CannotDeserialize)
            }
        }
    }
}
