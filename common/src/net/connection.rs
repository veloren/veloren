use std::thread::JoinHandle;
use super::tcp::Tcp;
use super::message::{Message, ServerMessage, ClientMessage};
use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError, PacketData};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::io::{Write, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

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
    }

    fn send_worker(&self) {
        loop {
            //println!("send");
            // TODO: notice sender faster then activly pool for new packages
            if *self.packet_out_count.read().unwrap() == 0 {
                thread::sleep(time::Duration::from_millis(1));
                continue;
            }
            // find next package
            let mut packets = self.packet_out.lock().unwrap();
            for i in 0..255 {
                if packets[i].len() != 0 {
                    // build part
                    match packets[i][0].generateFrame(60000) {
                        Ok(frame) => {
                            // send it
                            println!("yay");
                            self.tcp.send(frame);
                        },
                        Err(FrameError::SendDone) => {
                            println!("nay");
                            packets[i].pop_front();
                        },
                    }

                    break;
                }
            }
        }
    }

    fn recv_worker(&self) {
        loop {
            println!("recv");
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
                            println!("load");
                            if packet.unwrap().loadDataFrame(frame) {
                                //convert
                                println!("finsihed");
                                let packet = packets.get_mut(&id);
                                let msg = RM::from(packet.unwrap().data());
                                let msg = Box::new(msg.unwrap());
                                //trigger callback
                                let f = self.callback.lock().unwrap();
                                //f(msg);
                                //let co = self.callbackobj.lock().unwrap();

                                //self.callbackobj.lock().unwrap().as_mut().unwrap().recv(msg);
                                let mut co = self.callbackobj.lock();
                                match co.unwrap().as_mut() {
                                    Some(cb) => {
                                        //cb.recv(msg);
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
