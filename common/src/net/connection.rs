// Standard
use std::net::UdpSocket;
use std::thread::JoinHandle;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::net::{TcpStream, ToSocketAddrs, SocketAddr};
use std::thread;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;

// Library
use bincode;
use get_if_addrs::get_if_addrs;

// Parent
use super::tcp::Tcp;
use super::udpmgr::UdpMgr;
use super::udp::Udp;
use super::protocol::Protocol;
use super::message::{Message};
use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError};
use super::Error;

pub trait Callback<RM: Message> {
    fn recv(&self, Result<RM, Error>);
}

#[derive(Clone, Debug, Serialize, Deserialize)]
enum ConnectionMessage {
    OpenedUdp { host: SocketAddr },
    Shutdown,
    Ping,
}

impl Message for ConnectionMessage {
    fn from_bytes(data: &[u8]) -> Result<ConnectionMessage, Error> {
        bincode::deserialize(data).map_err(|_e| Error::CannotDeserialize)
    }

    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(&self).map_err(|_e| Error::CannotSerialize)
    }
}

pub struct Connection<RM: Message> {
    // sorted by prio and then cronically
    tcp: Tcp,
    udpmgr: Arc<UdpMgr>,
    udp: Mutex<Option<Udp>>,
    callback: Mutex<Box<Fn(Result<RM, Error>)+ Send>>,
    callbackobj: Mutex<Option<Arc<Callback<RM> + Send + Sync>>>,
    packet_in: Mutex<HashMap<u64, IncommingPacket>>,
    packet_out: Mutex<Vec<VecDeque<OutgoingPacket>>>,
    packet_out_count: RwLock<u64>,
    running: AtomicBool,
    send_thread: Mutex<Option<JoinHandle<()>>>,
    recv_thread: Mutex<Option<JoinHandle<()>>>,
    send_thread_udp: Mutex<Option<JoinHandle<()>>>,
    recv_thread_udp: Mutex<Option<JoinHandle<()>>>,
    next_id: Mutex<u64>,
}

impl<'a, RM: Message + 'static> Connection<RM> {
    pub fn new<A: ToSocketAddrs>(remote: &A, callback: Box<Fn(Result<RM, Error>) + Send>, cb: Option<Arc<Callback<RM> + Send + Sync>>, udpmgr: Arc<UdpMgr>) -> Result<Arc<Connection<RM>>, Error> {
        let packet_in = HashMap::new();
        let mut packet_out = Vec::new();
        for _i in 0..255 {
            packet_out.push(VecDeque::new());
        }

        let m = Connection {
            tcp: Tcp::new(&remote)?,
            udpmgr,
            udp: Mutex::new(None),
            callback:  Mutex::new(callback),
            callbackobj: Mutex::new(cb),
            packet_in: Mutex::new(packet_in),
            packet_out_count: RwLock::new(0),
            packet_out: Mutex::new(packet_out),
            running: AtomicBool::new(true),
            send_thread: Mutex::new(None),
            recv_thread: Mutex::new(None),
            send_thread_udp: Mutex::new(None),
            recv_thread_udp: Mutex::new(None),
            next_id: Mutex::new(1),
        };

        Ok(Arc::new(m))
    }

    pub fn new_stream(stream: TcpStream, callback: Box<Fn(Result<RM, Error>) + Send>, cb: Option<Arc<Callback<RM> + Send + Sync>>, udpmgr: Arc<UdpMgr>) -> Result<Arc<Connection<RM>>, Error> {
        let packet_in = HashMap::new();
        let mut packet_out = Vec::new();
        for _i in 0..255 {
            packet_out.push(VecDeque::new());
        }

        let m = Connection {
            tcp: Tcp::new_stream(stream)?,
            udpmgr,
            udp: Mutex::new(None),
            callback:  Mutex::new(callback),
            callbackobj: Mutex::new(cb),
            packet_in: Mutex::new(packet_in),
            packet_out_count: RwLock::new(0),
            packet_out: Mutex::new(packet_out),
            running: AtomicBool::new(true),
            send_thread: Mutex::new(None),
            recv_thread: Mutex::new(None),
            send_thread_udp: Mutex::new(None),
            recv_thread_udp: Mutex::new(None),
            next_id: Mutex::new(1),
        };

        let m = Arc::new(m);
        Ok(m)
    }

    pub fn set_callback(&mut self, callback: Box<Fn(Result<RM, Error>) + Send + Sync>) {
        self.callback = Mutex::new(callback);
    }

    pub fn callback(&self) -> MutexGuard<Box<Fn(Result<RM, Error>) + Send>> {
        self.callback.lock().unwrap()
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

    pub fn callbackobj(&self) -> MutexGuard<Option<Arc<Callback<RM> + Send + Sync>>> {
        self.callbackobj.lock().unwrap()
    }

    pub fn start<'b>(manager: &'b Arc<Connection<RM>>) {
        let m = manager.clone();
        let mut rt = manager.recv_thread.lock().unwrap();
        *rt = Some(thread::spawn(move || {
            m.recv_worker();
        }));

        let m = manager.clone();
        let mut st = manager.send_thread.lock().unwrap();
        *st = Some(thread::spawn(move || {
            m.send_worker();
        }));
    }

    pub fn stop<'b>(manager: &'b Arc<Connection<RM>>) {
        let m = manager.clone();
        m.running.store(false, Ordering::Relaxed);
        // non blocking stop for now
    }

    pub fn send<M: Message>(&self, message: M) {
        let mut id = self.next_id.lock().unwrap();
        self.packet_out.lock().unwrap()[16].push_back(OutgoingPacket::new(message, *id));
        *id += 1;
        let mut p = self.packet_out_count.write().unwrap();
        *p += 1;
        let rt = self.send_thread.lock();
        if let Some(cb) = rt.unwrap().as_mut() {
            //trigger sending
            cb.thread().unpark();
        }
    }

    fn trigger_callback(&self, msg: Result<RM, Error>) {
        //trigger callback
        let f = self.callback.lock().unwrap();
        let co = self.callbackobj.lock();
        match co.unwrap().as_mut() {
            Some(cb) => {
                cb.recv(msg);
            },
            None => {
                f(msg);
            },
        }
    }

    fn send_worker(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            if *self.packet_out_count.read().unwrap() == 0 {
                thread::park();
                continue;
            }
            // find next package
            let mut packets = self.packet_out.lock().unwrap();
            for i in 0..255 {
                if packets[i].len() != 0 {
                    // build part
                    const SPLIT_SIZE: u64 = 2000;
                    match packets[i][0].generate_frame(SPLIT_SIZE) {
                        Ok(frame) => {
                            // send it
                            self.tcp.send(frame).unwrap();
                        },
                        Err(FrameError::SendDone) => {
                            packets[i].pop_front();
                            let mut p = self.packet_out_count.write().unwrap();
                            *p -= 1;
                        },
                    }

                    break;
                }
            }
        }
    }

    fn recv_worker(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            let frame = self.tcp.recv();
            match frame {
                Ok(frame) => {
                    match frame {
                        Frame::Header{id, ..} => {
                            let msg = IncommingPacket::new(frame);
                            let mut packets = self.packet_in.lock().unwrap();
                            packets.insert(id, msg);
                        }
                        Frame::Data{id, ..} => {
                            let mut packets = self.packet_in.lock().unwrap();
                            let packet = packets.get_mut(&id);
                            if packet.unwrap().load_data_frame(frame) {
                                //convert
                                let packet = packets.get_mut(&id);
                                let data = packet.unwrap().data();
                                debug!("received packet: {:?}", &data);
                                let msg = RM::from_bytes(data);
                                self.trigger_callback(Ok(msg.unwrap()));
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Net Error {:?}", &e);
                    self.trigger_callback(Err(e));
                }
            }
        }
    }

    fn send_worker_udp(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            if *self.packet_out_count.read().unwrap() == 0 {
                thread::park();
                continue;
            }
            // find next package
            let mut packets = self.packet_out.lock().unwrap();
            for i in 0..255 {
                if packets[i].len() != 0 {
                    // build part
                    const SPLIT_SIZE: u64 = 2000;
                    match packets[i][0].generate_frame(SPLIT_SIZE) {
                        Ok(frame) => {
                            // send it
                            let mut udp = self.udp.lock().unwrap();
                            udp.as_mut().unwrap().send(frame).unwrap();
                        },
                        Err(FrameError::SendDone) => {
                            packets[i].pop_front();
                            let mut p = self.packet_out_count.write().unwrap();
                            *p -= 1;
                        },
                    }

                    break;
                }
            }
        }
    }

    fn recv_worker_udp(&self) {
        loop {
            if !self.running.load(Ordering::Relaxed) {
                break;
            }
            let mut udp = self.udp.lock().unwrap();
            let frame = udp.as_mut().unwrap().recv();
            match frame {
                Ok(frame) => {
                    match frame {
                        Frame::Header{id, ..} => {
                            let msg = IncommingPacket::new(frame);
                            let mut packets = self.packet_in.lock().unwrap();
                            packets.insert(id, msg);
                        }
                        Frame::Data{id, ..} => {
                            let mut packets = self.packet_in.lock().unwrap();
                            let packet = packets.get_mut(&id);
                            if packet.unwrap().load_data_frame(frame) {
                                //convert
                                let packet = packets.get_mut(&id);
                                let data = packet.unwrap().data();
                                debug!("received packet: {:?}", &data);
                                let msg = RM::from_bytes(data);
                                self.trigger_callback(Ok(msg.unwrap()));
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Net Error {:?}", &e);
                    self.trigger_callback(Err(e));
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
