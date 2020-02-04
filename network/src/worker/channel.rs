use crate::{
    api::Promise,
    internal::{RemoteParticipant, VELOREN_MAGIC_NUMBER, VELOREN_NETWORK_VERSION},
    message::{InCommingMessage, MessageBuffer, OutGoingMessage},
    worker::types::{Frame, Mid, Pid, RtrnMsg, Sid, Stream},
};
use enumset::EnumSet;
use mio_extras::channel::Sender;
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
    time::Instant,
};
use tracing::*;

pub(crate) trait Channel {
    /*
        uninitialized_dirty_speed_buffer: is just a already allocated buffer, that probably is already dirty because it's getting reused to save allocations, feel free to use it, but expect nothing
        aprox_time is the time taken when the events come in, you can reuse it for message timeouts, to not make any more syscalls
    */
    /// Execute when ready to read
    fn read(
        &mut self,
        uninitialized_dirty_speed_buffer: &mut [u8; 65000],
        aprox_time: Instant,
        rtrn_tx: &Sender<RtrnMsg>,
    );
    /// Execute when ready to write
    fn write(&mut self, uninitialized_dirty_speed_buffer: &mut [u8; 65000], aprox_time: Instant);
    fn open_stream(&mut self, prio: u8, promises: EnumSet<Promise>) -> u32;
    fn close_stream(&mut self, sid: u32);
    fn handshake(&mut self);
    fn shutdown(&mut self);
    fn send(&mut self, outgoing: OutGoingMessage);
}

#[derive(Debug)]
pub(crate) struct ChannelState {
    pub stream_id_pool: Option<tlid::Pool<tlid::Wrapping<Sid>>>, /* TODO: stream_id unique per
                                                                  * participant */
    pub msg_id_pool: Option<tlid::Pool<tlid::Wrapping<Mid>>>, //TODO: msg_id unique per
    // participant
    pub local_pid: Pid,
    pub remote_pid: Option<Pid>,
    pub remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
    pub streams: Vec<Stream>,
    pub send_queue: VecDeque<Frame>,
    pub recv_queue: VecDeque<InCommingMessage>,
    pub send_handshake: bool,
    pub send_pid: bool,
    pub send_config: bool,
    pub send_shutdown: bool,
    pub recv_handshake: bool,
    pub recv_pid: bool,
    pub recv_config: bool,
    pub recv_shutdown: bool,
}

/*
 Participant A
 Participant B
 A sends Handshake
 B receives Handshake and answers with Handshake
 A receives Handshake and answers with ParticipantId
 B receives ParticipantId and answeres with ParticipantId
 A receives ParticipantId and answers with Configuration for Streams and Messages
 ---
 A and B can now concurrently open Streams and send messages
 ---
 Shutdown phase
*/

impl ChannelState {
    const WRONG_NUMBER: &'static [u8] = "Handshake does not contain the magic number requiered by \
                                         veloren server.\nWe are not sure if you are a valid \
                                         veloren client.\nClosing the connection"
        .as_bytes();
    const WRONG_VERSION: &'static str = "Handshake does not contain a correct magic number, but \
                                         invalid version.\nWe don't know how to communicate with \
                                         you.\n";

    pub fn new(local_pid: Pid, remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>) -> Self {
        ChannelState {
            stream_id_pool: None,
            msg_id_pool: None,
            local_pid,
            remote_pid: None,
            remotes,
            streams: Vec::new(),
            send_queue: VecDeque::new(),
            recv_queue: VecDeque::new(),
            send_handshake: false,
            send_pid: false,
            send_config: false,
            send_shutdown: false,
            recv_handshake: false,
            recv_pid: false,
            recv_config: false,
            recv_shutdown: false,
        }
    }

    pub fn can_send(&self) -> bool {
        self.remote_pid.is_some()
            && self.recv_handshake
            && self.send_pid
            && self.recv_pid
            && (self.send_config || self.recv_config)
            && !self.send_shutdown
            && !self.recv_shutdown
    }

    pub fn handle(&mut self, frame: Frame, rtrn_tx: &Sender<RtrnMsg>) {
        match frame {
            Frame::Handshake {
                magic_number,
                version,
            } => {
                if magic_number != VELOREN_MAGIC_NUMBER {
                    error!("tcp connection with invalid handshake, closing connection");
                    self.wrong_shutdown(Self::WRONG_NUMBER);
                }
                if version != VELOREN_NETWORK_VERSION {
                    error!("tcp connection with wrong network version");
                    self.wrong_shutdown(
                        format!(
                            "{} Our Version: {:?}\nYour Version: {:?}\nClosing the connection",
                            Self::WRONG_VERSION,
                            VELOREN_NETWORK_VERSION,
                            version,
                        )
                        .as_bytes(),
                    );
                }
                debug!("handshake completed");
                self.recv_handshake = true;
                if self.send_handshake {
                    self.send_queue.push_back(Frame::ParticipantId {
                        pid: self.local_pid,
                    });
                    self.send_pid = true;
                } else {
                    self.send_queue.push_back(Frame::Handshake {
                        magic_number: VELOREN_MAGIC_NUMBER.to_string(),
                        version: VELOREN_NETWORK_VERSION,
                    });
                    self.send_handshake = true;
                }
            },
            Frame::Configure {
                stream_id_pool,
                msg_id_pool,
            } => {
                self.recv_config = true;
                //TODO remove range from rp! as this could probably cause duplicate ID !!!
                let mut remotes = self.remotes.write().unwrap();
                if let Some(pid) = self.remote_pid {
                    if !remotes.contains_key(&pid) {
                        remotes.insert(pid, RemoteParticipant::new());
                    }
                    if let Some(rp) = remotes.get_mut(&pid) {
                        self.stream_id_pool = Some(stream_id_pool);
                        self.msg_id_pool = Some(msg_id_pool);
                    }
                }
                info!("recv config. This channel is now configured!");
            },
            Frame::ParticipantId { pid } => {
                if self.remote_pid.is_some() {
                    error!(?pid, "invalid message, cant change participantId");
                    return;
                }
                self.remote_pid = Some(pid);
                debug!(?pid, "Participant send their ID");
                self.recv_pid = true;
                if self.send_pid {
                    let mut remotes = self.remotes.write().unwrap();
                    if !remotes.contains_key(&pid) {
                        remotes.insert(pid, RemoteParticipant::new());
                    }
                    if let Some(rp) = remotes.get_mut(&pid) {
                        self.stream_id_pool = Some(rp.stream_id_pool.subpool(1000000).unwrap());
                        self.msg_id_pool = Some(rp.msg_id_pool.subpool(1000000).unwrap());
                        self.send_queue.push_back(Frame::Configure {
                            stream_id_pool: rp.stream_id_pool.subpool(1000000).unwrap(),
                            msg_id_pool: rp.msg_id_pool.subpool(1000000).unwrap(),
                        });
                        self.send_config = true;
                        info!(?pid, "this channel is now configured!");
                    }
                } else {
                    self.send_queue.push_back(Frame::ParticipantId {
                        pid: self.local_pid,
                    });
                    self.send_pid = true;
                }
            },
            Frame::Shutdown {} => {
                self.recv_shutdown = true;
                info!("shutting down channel");
            },
            Frame::OpenStream {
                sid,
                prio,
                promises,
            } => {
                if let Some(pid) = self.remote_pid {
                    let stream = Stream::new(sid, prio, promises.clone());
                    self.streams.push(stream);
                    info!("opened a stream");
                } else {
                    error!("called OpenStream before PartcipantID!");
                }
            },
            Frame::CloseStream { sid } => {
                if let Some(pid) = self.remote_pid {
                    self.streams.retain(|stream| stream.sid() != sid);
                    info!("closed a stream");
                }
            },
            Frame::DataHeader { mid, sid, length } => {
                debug!("Data Header {}", sid);
                let imsg = InCommingMessage {
                    buffer: MessageBuffer { data: Vec::new() },
                    length,
                    mid,
                    sid,
                };
                let mut found = false;
                for s in &mut self.streams {
                    if s.sid() == sid {
                        //TODO: move to Hashmap, so slow
                        s.to_receive.push_back(imsg);
                        found = true;
                        break;
                    }
                }
                if !found {
                    error!("couldn't find stream with sid: {}", sid);
                }
            },
            Frame::Data {
                id,
                start,
                mut data,
            } => {
                debug!("Data Package {}, len: {}", id, data.len());
                let mut found = false;
                for s in &mut self.streams {
                    let mut pos = None;
                    for i in 0..s.to_receive.len() {
                        let m = &mut s.to_receive[i];
                        if m.mid == id {
                            found = true;
                            m.buffer.data.append(&mut data);
                            if m.buffer.data.len() as u64 == m.length {
                                pos = Some(i);
                                break;
                            };
                        };
                    }
                    if let Some(pos) = pos {
                        for m in s.to_receive.drain(pos..pos + 1) {
                            info!("receied message: {}", m.mid);
                            //self.recv_queue.push_back(m);
                            rtrn_tx.send(RtrnMsg::Receive(m));
                        }
                    }
                }
                if !found {
                    error!("couldn't find stream with mid: {}", id);
                }
            },
            Frame::Raw(data) => {
                info!("Got a Raw Package {:?}", data);
            },
        }
    }

    // This function will tick all streams according to priority and add them to the
    // send queue
    pub(crate) fn tick_streams(&mut self) {
        //ignoring prio for now
        //TODO: fix prio
        if let Some(msg_id_pool) = &mut self.msg_id_pool {
            for s in &mut self.streams {
                let mut remove = false;
                let sid = s.sid();
                if let Some(m) = s.to_send.front_mut() {
                    let to_send = std::cmp::min(m.buffer.data.len() as u64 - m.cursor, 1400);
                    if to_send > 0 {
                        if m.cursor == 0 {
                            let mid = msg_id_pool.next();
                            m.mid = Some(mid);
                            self.send_queue.push_back(Frame::DataHeader {
                                mid,
                                sid,
                                length: m.buffer.data.len() as u64,
                            });
                        }
                        self.send_queue.push_back(Frame::Data {
                            id: m.mid.unwrap(),
                            start: m.cursor,
                            data: m.buffer.data[m.cursor as usize..(m.cursor + to_send) as usize]
                                .to_vec(),
                        });
                    };
                    m.cursor += to_send;
                    if m.cursor == m.buffer.data.len() as u64 {
                        remove = true;
                        debug!(?m.mid, "finish message")
                    }
                }
                if remove {
                    s.to_send.pop_front();
                }
            }
        }
    }

    fn wrong_shutdown(&mut self, raw: &[u8]) {
        #[cfg(debug_assertions)]
        {
            debug!("sending client instructions before killing");
            self.send_queue.push_back(Frame::Raw(raw.to_vec()));
            self.send_queue.push_back(Frame::Shutdown {});
            self.send_shutdown = true;
        }
    }
}
