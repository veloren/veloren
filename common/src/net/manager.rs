use std::thread::JoinHandle;
use super::Conn;
use super::message::{Message, ServerMessage, ClientMessage};
use super::packet::{OutgoingPacket, IncommingPacket, Frame, FrameError, PacketData};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::io::{Write, Read};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time;
use std::collections::vec_deque::VecDeque;
use std::collections::HashMap;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

use super::Error;

pub struct Manager {
    // sorted by prio and then cronically
    conn: Conn,
    callbackS: Mutex<Box<Fn(ServerMessage)+ Send>>,
    callbackC: Mutex<Box<Fn(ClientMessage)+ Send>>,
    packet_in: Mutex<HashMap<u64, IncommingPacket>>,
    packet_out: Mutex<Vec<VecDeque<OutgoingPacket>>>,
    packet_out_count: RwLock<u64>,
    send_thread: Mutex<Option<JoinHandle<()>>>,
    recv_thread: Mutex<Option<JoinHandle<()>>>,
}

impl Manager {
    pub fn new<A: ToSocketAddrs, M:Message>(remote: A, callbackS: Box<Fn(ServerMessage)+ Send>, callbackC: Box<Fn(ClientMessage) + Send>) -> Result<Arc::<Manager>, Error> {
        let mut packet_in = HashMap::new();
        let mut packet_out = Vec::new();
        for i in 0..255 {
            packet_out.push(VecDeque::new());
        }

        let m = Manager {
            conn: Conn::new(remote)?,
            callbackS: Mutex::new(callbackS),
            callbackC:  Mutex::new(callbackC),
            packet_in: Mutex::new(packet_in),
            packet_out_count: RwLock::new(0),
            packet_out: Mutex::new(packet_out),
            send_thread: Mutex::new(None),
            recv_thread: Mutex::new(None),
        };

        Ok(Arc::new(m))
    }

    pub fn start<M: Message>(manager: &Arc<Manager>) {
        let m = manager.clone();
        let mut rt = manager.recv_thread.lock().unwrap();
        *rt = Some(thread::spawn(move || {
            m.recv_worker::<M>();
        }));

        let m = manager.clone();
        let mut st = manager.send_thread.lock().unwrap();
        *st = Some(thread::spawn(move || {
            m.send_worker();
        }));
    }

    pub fn send<M: Message>(&self, message: M) {
        self.packet_out.lock().unwrap()[16].push_back(OutgoingPacket::new(message));
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
                            self.conn.send(frame);
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

    fn recv_worker<M: Message>(&self) {
        loop {
            println!("recv");
            let frame = self.conn.recv();
            match frame {
                Ok(frame) => {
                    match frame {
                        Frame::Header{uid, ..} => {
                            let msg = IncommingPacket::new(frame);
                            let mut packets = self.packet_in.lock().unwrap();
                            packets.insert(uid, msg);
                        }
                        Frame::Data{uid, ..} => {
                            let mut packets = self.packet_in.lock().unwrap();
                            let mut packet = packets.get_mut(&uid);
                            packet.unwrap().loadDataFrame(frame);
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
