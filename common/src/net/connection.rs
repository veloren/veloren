use std::thread::JoinHandle;
use super::tcp::Tcp;
use super::message::{Message};
use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError, PacketData};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;

use super::Error;

pub trait Callback<RM: Message> {
    fn recv(&self, Box<RM>);
}

pub struct Connection<RM: Message> {
    // sorted by prio and then cronically
    tcp: Tcp,
    callback: Mutex<Box<Fn(Box<RM>)+ Send>>,
    callbackobj: Mutex<Option<Box<Arc<Callback<RM> + Send + Sync>>>>,
    packet_in: Mutex<HashMap<u64, IncommingPacket>>,
    packet_out: Mutex<Vec<VecDeque<OutgoingPacket>>>,
    packet_out_count: RwLock<u64>,
    send_thread: Mutex<Option<JoinHandle<()>>>,
    recv_thread: Mutex<Option<JoinHandle<()>>>,
    next_id: Mutex<u64>,
}

impl<'a, RM: Message + 'static> Connection<RM> {
    pub fn new<A: ToSocketAddrs>(remote: A, callback: Box<Fn(Box<RM>) + Send>, cb: Option<Box<Arc<Callback<RM> + Send + Sync>>>) -> Result<Arc<Connection<RM>>, Error> {
        let mut packet_in = HashMap::new();
        let mut packet_out = Vec::new();
        for i in 0..255 {
            packet_out.push(VecDeque::new());
        }

        let m = Connection {
            tcp: Tcp::new(remote)?,
            callback:  Mutex::new(callback),
            callbackobj: Mutex::new(cb),
            packet_in: Mutex::new(packet_in),
            packet_out_count: RwLock::new(0),
            packet_out: Mutex::new(packet_out),
            send_thread: Mutex::new(None),
            recv_thread: Mutex::new(None),
            next_id: Mutex::new(1),
        };

        Ok(Arc::new(m))
    }

    pub fn new_stream(stream: TcpStream, callback: Box<Fn(Box<RM>) + Send>, cb: Option<Box<Arc<Callback<RM> + Send + Sync>>>) -> Result<Arc<Connection<RM>>, Error> {
        let mut packet_in = HashMap::new();
        let mut packet_out = Vec::new();
        for i in 0..255 {
            packet_out.push(VecDeque::new());
        }

        let m = Connection {
            tcp: Tcp::new_stream(stream)?,
            callback:  Mutex::new(callback),
            callbackobj: Mutex::new(cb),
            packet_in: Mutex::new(packet_in),
            packet_out_count: RwLock::new(0),
            packet_out: Mutex::new(packet_out),
            send_thread: Mutex::new(None),
            recv_thread: Mutex::new(None),
            next_id: Mutex::new(1),
        };

        let m = Arc::new(m);
        Ok(m)
    }

    pub fn set_callback(&mut self, callback: Box<Fn(Box<RM>) + Send + Sync>) {
        self.callback = Mutex::new(callback);
    }

    pub fn callback(&self) -> MutexGuard<Box<Fn(Box<RM>) + Send>> {
        self.callback.lock().unwrap()
    }

    pub fn callbackobj(&self) -> MutexGuard<Option<Box<Arc<Callback<RM> + Send + Sync>>>> {
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

    pub fn send<M: Message>(&self, message: M) {
        let mut id = self.next_id.lock().unwrap();
        self.packet_out.lock().unwrap()[16].push_back(OutgoingPacket::new(message, *id));
        *id += 1;
        let mut p = self.packet_out_count.write().unwrap();
        *p += 1;
        let mut rt = self.send_thread.lock();
        if let Some(cb) = rt.unwrap().as_mut() {
            //trigger sending
            cb.thread().unpark();
        }
    }

    fn send_worker(&self) {
        loop {
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
                    match packets[i][0].generateFrame(SPLIT_SIZE) {
                        Ok(frame) => {
                            // send it
                            self.tcp.send(frame);
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
                            let mut packet = packets.get_mut(&id);
                            if packet.unwrap().loadDataFrame(frame) {
                                //convert
                                let packet = packets.get_mut(&id);
                                let data = packet.unwrap().data();
                                debug!("received packet: {:?}", &data);
                                let msg = RM::from(data);
                                let msg = Box::new(msg.unwrap());
                                //trigger callback
                                let f = self.callback.lock().unwrap();
                                let mut co = self.callbackobj.lock();
                                match co.unwrap().as_mut() {
                                    Some(cb) => {
                                        cb.recv(msg);
                                    },
                                    None => {
                                        f(msg);
                                    },
                                }
                            }
                        }
                    }
                },
                Err(e) => {
                    error!("Error {:?}", e);
                    thread::sleep(time::Duration::from_millis(1000));
                }
            }
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
