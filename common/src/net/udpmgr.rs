// Standard
use std::net::UdpSocket;
use std::thread::JoinHandle;
use std::sync::{Arc, RwLock};
use std::net::{ToSocketAddrs, SocketAddr};
use std::thread;

// Parent
use super::udp::Udp;

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

    pub fn stop_udp(mgr: Arc<UdpMgr>, udp: Arc<Udp>) {
        let mut subscriber = mgr.subscriber.write();
        let subscriber = subscriber.as_mut().unwrap();
        let _sockets = mgr.sockets.write().as_mut().unwrap();
        let mut udp_info = None;
        for s in subscriber.iter() {
            if Arc::ptr_eq(&s.udp, &udp) {
                udp_info = Some(s);
                break;
            }
        }
        let udp_info = udp_info.expect("udp does not exist");
        let mut socketusers = 0;
        for ref s in subscriber.iter() {
            if Arc::ptr_eq(&s.socket_info, &udp_info.socket_info) {
                socketusers += 1;
            }
        }
        if socketusers == 1 {
            // stop socket
            //actually i am to lazy to stop them now. sorry
        }
        let index = subscriber.iter().position(|x| Arc::ptr_eq(&x.udp, &udp_info.udp)).unwrap();
        subscriber.remove(index);
    }

    fn recv_worker_udp(&self, socket: UdpSocket) {
        loop {
            const MAX_UDP_SIZE : usize = 65535; // might not work in IPv6 Jumbograms
            //TODO: not read multiple frames and drop all but the first here
            let mut buff = vec![0; MAX_UDP_SIZE];
            let (size, remote) = socket.recv_from(&mut buff).unwrap();
            buff.resize(size, 0);
            println!("rcved sth of  {} bytes on {}", size, socket.local_addr().unwrap());
            let subscriber = self.subscriber.read().unwrap();
            for c in subscriber.iter() {
                if remote == c.remote && socket.local_addr().unwrap() == c.socket_info.socket.local_addr().unwrap() {
                    println!("forwarded it {} - {}", c.remote, c.socket_info.socket.local_addr().unwrap());
                    c.udp.received_raw_packet(&buff);
                }
            }
        }
    }
}
