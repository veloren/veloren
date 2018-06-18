use std::sync::Mutex;
use std::io::{Write, Read};
use std::net::{TcpStream, ToSocketAddrs};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::Error;
use super::Packet;

pub struct SendConn {
    stream_out: Mutex<TcpStream>,
}

impl SendConn {
    pub fn new<A: ToSocketAddrs>(remote: A) -> Result<SendConn, Error> {
        let stream = TcpStream::connect(remote)?;
        stream.set_nodelay(true)?;
        Ok(SendConn {
            stream_out: Mutex::new(stream),
        })
    }

    pub fn send<P: Packet>(&self, packet: P) -> Result<(), Error> {
        let data = packet.to_bytes()?;
        let mut stream = self.stream_out.lock().unwrap();
        stream.write_u32::<LittleEndian>(data.len() as u32)?;
        Ok(stream.write_all(&data)?)
    }
}

pub struct RecvConn {
    stream_in: Mutex<TcpStream>,
}

impl RecvConn {
    pub fn new<A: ToSocketAddrs>(remote: A) -> Result<RecvConn, Error> {
        let stream = TcpStream::connect(remote)?;
        stream.set_nodelay(true)?;
        Ok(RecvConn {
            stream_in: Mutex::new(stream),
        })
    }

    pub fn recv<P: Packet>(&self) -> Result<P, Error> {
        let mut stream = self.stream_in.lock().unwrap();
        let packet_size = stream.read_u32::<LittleEndian>()? as usize;
        let mut buff = vec![0; packet_size];
        stream.read_exact(&mut buff)?;
        Ok(P::from_bytes(&buff)?)
    }
}

pub struct Conn {
    send_conn: SendConn,
    recv_conn: RecvConn,
}

impl Conn {
    pub fn new<A: ToSocketAddrs>(remote: A) -> Result<Conn, Error> {
        let stream = TcpStream::connect(remote)?;
        stream.set_nodelay(true)?;
        Ok(Conn {
            send_conn: SendConn { stream_out: Mutex::new(stream.try_clone()?) },
            recv_conn: RecvConn { stream_in: Mutex::new(stream) },
        })
    }

    pub fn send<P: Packet>(&self, packet: P) -> Result<(), Error> {
        self.send_conn.send(packet)
    }

    pub fn recv<P: Packet>(&self) -> Result<P, Error> {
        self.recv_conn.recv()
    }

    pub fn split(self) -> (SendConn, RecvConn) {
        (self.send_conn, self.recv_conn)
    }
}
