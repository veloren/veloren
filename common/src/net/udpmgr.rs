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
    socket_info: Arc<SocketInfo>,
    remote: SocketAddr,
    udp: Arc<Udp>,
}

struct SocketInfo {
    socket: UdpSocket,
    recv_thread: JoinHandle<()>,
}

pub struct UdpMgr {
    subscriber: RwLock<Vec<UdpInfo>>,
    sockets: RwLock<Vec<Arc<SocketInfo>>>,
}

// One Socket can handle multiple receivers, thats why we need a Manager here
// Receiving can only be done onces and must be routed to the correct recveiver
impl UdpMgr {
    pub fn new() -> Arc<UdpMgr> {
        Arc::new(UdpMgr{
            subscriber: RwLock::new(Vec::new()),
            sockets: RwLock::new(Vec::new()),
        })
    }

    pub fn start_udp<A: ToSocketAddrs>(mgr: Arc<UdpMgr>, listen: &A, remote: &A) -> Arc<Udp> {
        let mut socket_info = None;
        let listen = listen.to_socket_addrs().unwrap().next().unwrap();
        let remote = remote.to_socket_addrs().unwrap().next().unwrap();
        {
            let subscriber = mgr.subscriber.read().unwrap();
            for c in &(*subscriber) {
                if c.socket_info.socket.local_addr().unwrap() == listen {
                    socket_info = Some(c.socket_info.clone());
                    break;
                }
            }
        }

        if let None = socket_info {
            // if non eist for this socket create
            let socket = UdpSocket::bind(listen).unwrap();
            let socketclone = socket.try_clone().unwrap();
            let mgrclone = mgr.clone();
            let recv_thread = thread::spawn(move || {
                mgrclone.recv_worker_udp(socketclone);
            });
            let socketclone = socket.try_clone().unwrap();
            let si = Arc::new(SocketInfo {
                socket: socketclone,
                recv_thread,
            });
            mgr.sockets.write().as_mut().unwrap().push(si.clone());
            socket_info = Some(si.clone());
            debug!("listen on new udp socket, started a new thread {}", listen);
        }
        let socket_info = socket_info.unwrap();

        let udp = Arc::new(Udp::new_stream(socket_info.socket.try_clone().unwrap(), remote).unwrap());
        debug!("created udp listnen on {} for remote {}", listen, remote);
        let ui = UdpInfo{
            socket_info,
            remote,
            udp: udp.clone(),
        };

        mgr.subscriber.write().as_mut().unwrap().push(ui);
        return udp.clone();
        /*
        manager.send(ConnectionMessage::OpenedUdp{ host: listen });*/
    }
    /*
    pub fn stop_udp<A: ToSocketAddrs>(mgr: Arc<UdpMgr>, listen: &A, remote: &A) -> Arc<Udp> {
        
    }*/

    fn recv_worker_udp(&self, socket: UdpSocket) {
        loop {
            const MAX_UDP_SIZE : usize = 65535; // might not work in IPv6 Jumbograms
            //TODO: not read multiple frames and drop all but the first here
            let mut buff = vec![0; MAX_UDP_SIZE];
            println!("lock for msg");
            let (size, remote) = socket.recv_from(&mut buff).unwrap();
            buff.resize(size, 0);
            println!("rcved sth of  {} bytes on {}", size, socket.local_addr().unwrap());
            let subscriber = self.subscriber.read().unwrap();
            for c in subscriber.iter() {
                if remote == c.remote && socket.local_addr().unwrap() == c.socket_info.socket.local_addr().unwrap() {
                    println!("forwarded it {} - {}", c.remote, c.socket_info.socket.local_addr().unwrap());
                    c.udp.received_raw_packet(buff.clone());
                }
            }
        }
    }
}
