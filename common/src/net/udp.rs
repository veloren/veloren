use net::protocol::Protocol;
use std::sync::Mutex;
use std::io::{Write, Read, Cursor};
use std::net::{UdpSocket, ToSocketAddrs};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::Error;
use super::packet::{Frame};

pub struct Udp {
    socket_in: Mutex<UdpSocket>,
    socket_out: Mutex<UdpSocket>,
}

impl Udp {
    pub fn new<A: ToSocketAddrs>(listen: A, remote: A) -> Result<Udp, Error> {
        let socket_in = UdpSocket::bind(listen)?;
        let socket_out = UdpSocket::bind(remote)?;
        Ok(Udp {
            socket_in: Mutex::new(socket_in),
            socket_out: Mutex::new(socket_out),
        })
    }

    pub fn new_stream(listenSocket: UdpSocket, remoteSocket: UdpSocket) -> Result<Udp, Error> {
        Ok(Udp {
            socket_in: Mutex::new(listenSocket),
            socket_out: Mutex::new(remoteSocket),
        })
    }
}

impl Protocol for Udp {
    fn send(&self, frame: Frame) -> Result<(), Error> {
        let socket = self.socket_out.lock().unwrap();
        match frame {
            Frame::Header{id, length} => {
                let mut buff = vec!();
                buff.write_u8(1)?; // 1 is const for Header
                buff.write_u64::<LittleEndian>(id)?;
                buff.write_u64::<LittleEndian>(length)?;
                socket.send(&buff)?;
                Ok(())
            }
            Frame::Data{id, frame_no, data} => {
                let mut buff = vec!();
                buff.write_u8(2)?; // 2 is const for Data
                buff.write_u64::<LittleEndian>(id)?;
                buff.write_u64::<LittleEndian>(frame_no)?;
                buff.write_u64::<LittleEndian>(data.len() as u64)?;
                buff.write_all(&data)?;
                socket.send(&buff)?;
                Ok(())
            }
        }
    }

    //blocking
    fn recv(&self) -> Result<Frame, Error> {
        let socket = self.socket_in.lock().unwrap();
        let mut buff = vec!();
        socket.recv(&mut buff)?;
        let mut cur = Cursor::new(buff);
        let frame = cur.read_u8()? as u8;
        match frame {
            1 => {
                let id = cur.read_u64::<LittleEndian>()? as u64;
                let length = cur.read_u64::<LittleEndian>()? as u64;
                Ok(Frame::Header{
                    id,
                    length,
                })
            },
            2 => {
                let id = cur.read_u64::<LittleEndian>()? as u64;
                let frame_no = cur.read_u64::<LittleEndian>()? as u64;
                let packet_size = cur.read_u64::<LittleEndian>()? as u64;
                let mut data = vec![0; packet_size as usize];
                cur.read_exact(&mut data)?;
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
