use std::thread::Thread;
use std::thread;
use std::net::SocketAddr;
use net::protocol::Protocol;
use std::sync::{Mutex, RwLock};
use std::io::{Write, Read, Cursor};
use std::net::{UdpSocket, ToSocketAddrs};
use std::collections::vec_deque::VecDeque;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::Error;
use super::packet::{Frame};

pub struct Udp {
    socket: RwLock<UdpSocket>,
    remote: SocketAddr,
    in_buffer: RwLock<VecDeque<Vec<u8>>>,
    waiting_threads: RwLock<Vec<Thread>>, //is a vec really needed here
}

impl Udp {
    pub fn new<A: ToSocketAddrs>(listen: A, remote: A) -> Result<Udp, Error> {
        let socket = UdpSocket::bind(listen)?;
        let remote = remote.to_socket_addrs().unwrap().next().unwrap();
        socket.connect(&remote);
        Ok(Udp {
            socket: RwLock::new(socket),
            remote: remote,
            in_buffer: RwLock::new(VecDeque::new()),
            waiting_threads: RwLock::new(Vec::new()),
        })
    }

    pub fn new_stream<A: ToSocketAddrs>(socket: UdpSocket, remote: A) -> Result<Udp, Error> {
        let remote = remote.to_socket_addrs().unwrap().next().unwrap();
        Ok(Udp {
            socket: RwLock::new(socket),
            remote,
            in_buffer: RwLock::new(VecDeque::new()),
            waiting_threads: RwLock::new(Vec::new()),
        })
    }

    pub fn received_raw_packet(&self, rawpacket: Vec<u8>) {
        self.in_buffer.write().unwrap().push_back(rawpacket);
        let lock = self.waiting_threads.read().unwrap();
        for t in lock.iter() {
            t.unpark();
        }
    }
}

impl Protocol for Udp {
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
        {
            if self.in_buffer.read().unwrap().is_empty() {
                self.waiting_threads.write().unwrap().push(thread::current());
                while self.in_buffer.read().unwrap().is_empty() {
                    // hope a unpark does never happen in between those two statements
                    thread::park();
                }
            }
        }
        let lock = self.in_buffer.read().unwrap();
        let data = lock.front().unwrap();
        let mut cur = Cursor::new(data);
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
