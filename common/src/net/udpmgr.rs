use get_if_addrs::get_if_addrs;
use std::net::UdpSocket;
use std::thread::JoinHandle;
use super::tcp::Tcp;
use super::udp::Udp;
use super::protocol::Protocol;
use super::message::{Message};
use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError, PacketData};
use super::connection::{Connection};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::net::{TcpStream, ToSocketAddrs, SocketAddr};
use std::thread;
use std::time;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;
use bincode;

use super::Error;

struct UdpInfo {
    socket: UdpSocket,
    remote: SocketAddr,
    udp: Arc<Udp<SocketAddr>>,
    recv_threads: Arc<JoinHandle<()>>,
}

//Some things need to be syncronized between Udp, recieving for example. there can only be one thread receiving on one UDP socket. and UDP sockets are shared between Connections
pub struct UdpMgr {
    subscriber: RwLock<Vec<UdpInfo>>,
}

impl UdpMgr {
    pub fn new() -> UdpMgr {
        UdpMgr{
            subscriber: RwLock::new(Vec::new()),
        }
    }

    pub fn start_udp(&self, listen: SocketAddr, remote: SocketAddr) {
        let mut socket = None;
        let mut recv_threads = None;
        {
            let subscriber = self.subscriber.read().unwrap();
            for c in &(*subscriber) {
                if c.socket.local_addr().unwrap() == listen {
                    socket = Some(c.socket.try_clone().unwrap());
                    recv_threads = Some(c.recv_threads.clone());
                    break;
                }
            }
        }

        if let None = socket {
            socket = Some(UdpSocket::bind(listen).unwrap());
        }
        let socket = socket.unwrap();
        let udp = Arc::new(Udp::new_stream(socket, remote).unwrap());
        if let None = recv_threads {
            recv_threads = Some(Arc::new(thread::spawn(move || {
                self.recv_worker_udp(udp.clone(), socket.try_clone().unwrap());
            })));
        }
        let recv_threads = recv_threads.unwrap();

        let conn_info = UdpInfo{
            socket,
            remote,
            udp,
            recv_threads,
        };

        self.subscriber.write().as_mut().unwrap().push(conn_info);
        /*
        manager.send(ConnectionMessage::OpenedUdp{ host: listen });*/
    }

    fn recv_worker_udp(&self, udp: Arc<Udp<SocketAddr>>, socket: UdpSocket) {
        loop {
            const MAX_UDP_SIZE : usize = 65535; // might not work in IPv6 Jumbograms
            //TODO: not read multiple frames and drop all but the first here
            let mut buff = Vec::with_capacity(MAX_UDP_SIZE);
            buff.resize(MAX_UDP_SIZE, 0);
            let (size, remote) = socket.recv_from(&mut buff).unwrap();
            let subscriber = self.subscriber.read().unwrap();
            for c in *subscriber {
                if remote == c.remote {
                    c.udp.received_raw_packet(buff);
                }
            }
        }
    }

    fn bind_udp<T: ToSocketAddrs>(bind_addr: &T) -> Result<UdpSocket, Error> {
        let sock = UdpSocket::bind(&bind_addr);
        match sock {
            Ok(s) => Ok(s),
            Err(_e) => {
                let new_bind = bind_addr.to_socket_addrs()?
                                        .next().unwrap()
                                        .port() + 1;
                let ip = get_if_addrs().unwrap()[0].ip();
                let new_addr = SocketAddr::new(
                    ip,
                    new_bind
                );
                warn!("Binding local port failed, trying {}", new_addr);
                UdpMgr::bind_udp(&new_addr)
            },
        }
    }

/*
    pub fn recv<P: Packet>(&self) -> Result<P, Error> {
        let mut stream = self.stream_in.lock().unwrap();
        let packet_size = stream.read_u32::<LittleEndian>()? as usize;
        let mut buff = vec![0; packet_size];
        stream.read_exact(&mut buff)?;
        Ok(P::from(&buff)?)
    }*/
}
