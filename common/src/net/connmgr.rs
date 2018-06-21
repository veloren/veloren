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

struct ConnInfo {
    socket: UdpSocket,
    udp: Udp<SocketAddr>,
    recv_threads: JoinHandle<()>,
}

//Some things need to be syncronized between Connections, udp recieving for example. there can only be one thread receiving on one UDP socket. and UDP sockets are shared between Connections
pub struct ConnMgr {
    conns: RwLock<Vec<ConnInfo>>,
}

impl ConnMgr {
    pub fn new() -> ConnMgr {
        ConnMgr{
            conns: RwLock::new(Vec::new()),
        }
    }

    pub fn start_udp<RM: Message>(&self, manager: &Arc<Connection<RM>>, listen: SocketAddr, remote: SocketAddr) {
        let mut alreadyExists = false;
        let socket;
        {
            let conns = self.conns.read().unwrap();
            for c in *conns {
                if c.socket.local_addr().unwrap() == listen {
                    alreadyExists = true;
                    socket = c.socket;
                    break;
                }
            }
        }
        if !alreadyExists {
            socket = UdpSocket::bind(listen).unwrap();
        }

        let udp = Udp::new_stream(socket, remote).unwrap();

        let recv_threads =  thread::spawn(move || {
            self.recv_worker_udp(udp);
        });

        let conn_info = ConnInfo{
            socket,
            udp,
            recv_threads,
        };

        self.conns.write().as_mut().unwrap().push(conn_info);
        /*
        manager.send(ConnectionMessage::OpenedUdp{ host: listen });*/
    }

    pub fn open_udp<'b>(manager: &'b Arc<Connection<RM>>, listen: SocketAddr, sender: SocketAddr) {
        if let Some(..) = *manager.udp.lock().unwrap() {
            panic!("not implemented");
        }
        *manager.udp.lock().unwrap() = Some(Udp::new(listen, sender).unwrap());
        manager.send(ConnectionMessage::OpenedUdp{ host: listen });

        let m = manager.clone();
        let mut rt = manager.recv_thread_udp.lock().unwrap();
        *rt = Some(thread::spawn(move || {
            m.recv_worker_udp();
        }));

        let m = manager.clone();
        let mut st = manager.send_thread_udp.lock().unwrap();
        *st = Some(thread::spawn(move || {
            m.send_worker_udp();
        }));
    }

    fn recv_worker_udp(&self, udp: Udp<SocketAddr>) {
        loop {
            let frame = udp.recv();
            match frame {
                Ok(frame) => {

                },
                Err(e) => {
                    error!("Udp Error {:?}", e);
                    thread::sleep(time::Duration::from_millis(1000));
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
                Connection::<RM>::bind_udp(&new_addr)
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
