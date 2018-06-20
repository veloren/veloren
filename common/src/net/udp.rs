use std::net::SocketAddr;
use net::protocol::Protocol;
use std::sync::{Mutex, RwLock};
use std::io::{Write, Read, Cursor};
use std::net::{UdpSocket, ToSocketAddrs};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::Error;
use super::packet::{Frame};

pub struct Udp<A: ToSocketAddrs> {
    socket: RwLock<UdpSocket>,
    remote: A,
}

impl<A: ToSocketAddrs> Udp<A> {
    pub fn new(listen: A, remote: A) -> Result<Udp<A>, Error> {
        let socket = UdpSocket::bind(listen)?;
        socket.connect(&remote);
        Ok(Udp {
            socket: RwLock::new(socket),
            remote: remote,
        })
    }

    pub fn new_stream(socket: UdpSocket, remote: A) -> Result<Udp<A>, Error> {
        Ok(Udp {
            socket: RwLock::new(socket),
            remote,
        })
    }
}

impl<A: ToSocketAddrs> Protocol for Udp<A> {
    fn send(&self, frame: Frame) -> Result<(), Error> {
        let socket = self.socket.read().unwrap();
        match frame {
            Frame::Header{id, length} => {
                let mut buff = Vec::with_capacity(17);
                buff.write_u8(1)?; // 1 is const for Header
                buff.write_u64::<LittleEndian>(id)?;
                buff.write_u64::<LittleEndian>(length)?;
                socket.send_to(&buff, &self.remote)?;
                Ok(())
            }
            Frame::Data{id, frame_no, data} => {
                let mut buff = Vec::with_capacity(25+data.len());
                buff.write_u8(2)?; // 2 is const for Data
                buff.write_u64::<LittleEndian>(id)?;
                buff.write_u64::<LittleEndian>(frame_no)?;
                buff.write_u64::<LittleEndian>(data.len() as u64)?;
                buff.write_all(&data)?;
                socket.send_to(&buff, &self.remote)?;
                Ok(())
            }
        }
    }

    //blocking
    fn recv(&self) -> Result<Frame, Error> {
        let socket = self.socket.read().unwrap();
        const MAX_UDP_SIZE : usize = 65535; // might not work in IPv6 Jumbograms
        //TODO: maybe on demand resizing might be more efficient. and maybe its needed to not read multiple frames and drop all but the first here
        let mut buff = Vec::with_capacity(MAX_UDP_SIZE);
        buff.resize(MAX_UDP_SIZE, 0);
        /*let ret = socket.peek_from(&mut buff)?;
        if ret.1 == self.remote {

        }*/
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
