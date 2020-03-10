use crate::{
    api::Promise,
    message::{InCommingMessage, MessageBuffer, OutGoingMessage},
    mpsc::MpscChannel,
    tcp::TcpChannel,
    types::{
        Frame, IntStream, Pid, RtrnMsg, Sid, DEFAULT_SID_SIZE, VELOREN_MAGIC_NUMBER,
        VELOREN_NETWORK_VERSION,
    },
    udp::UdpChannel,
};
use enumset::EnumSet;
use futures::{executor::block_on, sink::SinkExt};
use rand::{thread_rng, Rng};
use std::{
    collections::{HashMap, VecDeque},
    sync::{mpsc, Arc, RwLock},
};
use tlid;
use tracing::*;

pub(crate) trait ChannelProtocol {
    type Handle: ?Sized + mio::Evented;
    /// Execute when ready to read
    fn read(&mut self) -> Vec<Frame>;
    /// Execute when ready to write
    fn write<I: std::iter::Iterator<Item = Frame>>(&mut self, frames: &mut I);
    /// used for mio
    fn get_handle(&self) -> &Self::Handle;
}

#[derive(Debug)]
pub(crate) enum ChannelProtocols {
    Tcp(TcpChannel),
    Udp(UdpChannel),
    Mpsc(MpscChannel),
}

#[derive(Debug)]
pub(crate) struct Channel {
    pub stream_id_pool: Option<tlid::Pool<tlid::Wrapping<Sid>>>, /* TODO: stream_id unique per
                                                                  * participant */
    // participantd
    pub randomno: u64,
    pub local_pid: Pid,
    pub remote_pid: Option<Pid>,
    pub sid_backup_per_participant: Arc<RwLock<HashMap<Pid, tlid::Pool<tlid::Checked<Sid>>>>>,
    pub streams: Vec<IntStream>,
    pub send_queue: VecDeque<Frame>,
    pub protocol: ChannelProtocols,
    pub return_pid_to: Option<std::sync::mpsc::Sender<Pid>>, //use for network::connect()
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

impl Channel {
    const WRONG_NUMBER: &'static [u8] = "Handshake does not contain the magic number requiered by \
                                         veloren server.\nWe are not sure if you are a valid \
                                         veloren client.\nClosing the connection"
        .as_bytes();
    const WRONG_VERSION: &'static str = "Handshake does not contain a correct magic number, but \
                                         invalid version.\nWe don't know how to communicate with \
                                         you.\n";

    pub fn new(
        local_pid: Pid,
        protocol: ChannelProtocols,
        sid_backup_per_participant: Arc<RwLock<HashMap<Pid, tlid::Pool<tlid::Checked<Sid>>>>>,
        return_pid_to: Option<std::sync::mpsc::Sender<Pid>>,
    ) -> Self {
        let randomno = thread_rng().gen();
        warn!(?randomno, "new channel,yay ");
        Self {
            randomno,
            stream_id_pool: None,
            local_pid,
            remote_pid: None,
            sid_backup_per_participant,
            streams: Vec::new(),
            send_queue: VecDeque::new(),
            protocol,
            return_pid_to,
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

    pub fn tick_recv(
        &mut self,
        worker_participants: &mut HashMap<Pid, tlid::Pool<tlid::Wrapping<Sid>>>,
        rtrn_tx: &mpsc::Sender<RtrnMsg>,
    ) {
        match &mut self.protocol {
            ChannelProtocols::Tcp(c) => {
                for frame in c.read() {
                    self.handle(frame, worker_participants, rtrn_tx);
                }
            },
            ChannelProtocols::Udp(c) => {
                for frame in c.read() {
                    self.handle(frame, worker_participants, rtrn_tx);
                }
            },
            ChannelProtocols::Mpsc(c) => {
                for frame in c.read() {
                    self.handle(frame, worker_participants, rtrn_tx);
                }
            },
        }
    }

    pub fn tick_send(&mut self) {
        self.tick_streams();
        match &mut self.protocol {
            ChannelProtocols::Tcp(c) => {
                c.write(&mut self.send_queue.drain(..));
            },
            ChannelProtocols::Udp(c) => {
                c.write(&mut self.send_queue.drain(..));
            },
            ChannelProtocols::Mpsc(c) => {
                c.write(&mut self.send_queue.drain(..));
            },
        }
    }

    fn handle(
        &mut self,
        frame: Frame,
        worker_participants: &mut HashMap<Pid, tlid::Pool<tlid::Wrapping<Sid>>>,
        rtrn_tx: &mpsc::Sender<RtrnMsg>,
    ) {
        match frame {
            Frame::Handshake {
                magic_number,
                version,
            } => {
                if magic_number != VELOREN_MAGIC_NUMBER {
                    error!(
                        ?magic_number,
                        "connection with invalid magic_number, closing connection"
                    );
                    self.wrong_shutdown(Self::WRONG_NUMBER);
                }
                if version != VELOREN_NETWORK_VERSION {
                    error!(?version, "tcp connection with wrong network version");
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
            Frame::ParticipantId { pid } => {
                if self.remote_pid.is_some() {
                    error!(?pid, "invalid message, cant change participantId");
                    return;
                }
                self.remote_pid = Some(pid);
                debug!(?pid, "Participant send their ID");
                self.recv_pid = true;
                if self.send_pid {
                    //If participant is unknown to worker, assign some range from global pool
                    if !worker_participants.contains_key(&pid) {
                        let mut global_participants =
                            self.sid_backup_per_participant.write().unwrap();
                        //if this is the first time a participant connects to this Controller
                        if !global_participants.contains_key(&pid) {
                            // I dont no participant, so i can safely assume that they don't know
                            // me. so HERE we gonna fill local network pool
                            global_participants.insert(pid, tlid::Pool::new_full());
                        }
                        //grab a range for controller
                        let global_part_pool = global_participants.get_mut(&pid).unwrap();

                        let mut local_controller_sids =
                            tlid::subpool_wrapping(global_part_pool, DEFAULT_SID_SIZE).unwrap();
                        let remote_controller_sids =
                            tlid::subpool_wrapping(global_part_pool, DEFAULT_SID_SIZE).unwrap();
                        let mut local_worker_sids =
                            tlid::subpool_wrapping(global_part_pool, DEFAULT_SID_SIZE).unwrap();
                        let remote_worker_sids =
                            tlid::subpool_wrapping(global_part_pool, DEFAULT_SID_SIZE).unwrap();

                        let local_controller_range =
                            tlid::RemoveAllocation::new(&mut local_controller_sids);
                        let local_worker_range =
                            tlid::RemoveAllocation::new(&mut local_worker_sids);

                        worker_participants.insert(pid.clone(), local_worker_sids);
                        self.send_queue.push_back(Frame::Configure {
                            sender_controller_sids: local_controller_range,
                            sender_worker_sids: local_worker_range,
                            receiver_controller_sids: remote_controller_sids,
                            receiver_worker_sids: remote_worker_sids,
                        });
                        self.send_config = true;
                        info!(?pid, "this channel is now configured!");
                        if let Err(err) = rtrn_tx.send(RtrnMsg::ConnectedParticipant {
                            controller_sids: local_controller_sids,
                            pid,
                        }) {
                            error!(?err, "couldn't notify, is network already closed ?");
                        }
                    } else {
                        warn!(
                            "a known participant opened an additional channel, UNCHECKED BECAUSE \
                             NO TOKEN WAS IMPLEMENTED IN THE HANDSHAKE!"
                        );
                    }
                } else {
                    self.send_queue.push_back(Frame::ParticipantId {
                        pid: self.local_pid,
                    });
                    self.send_pid = true;
                }
            },
            Frame::Configure {
                sender_controller_sids,
                sender_worker_sids,
                mut receiver_controller_sids,
                mut receiver_worker_sids,
            } => {
                let pid = match self.remote_pid {
                    Some(pid) => pid,
                    None => {
                        error!("Cant configure a Channel without a PID first!");
                        return;
                    },
                };
                self.recv_config = true;
                //Check if worker already knows about this participant
                if !worker_participants.contains_key(&pid) {
                    let mut global_participants = self.sid_backup_per_participant.write().unwrap();
                    if !global_participants.contains_key(&pid) {
                        // I dont no participant, so i can safely assume that they don't know me. so
                        // HERE we gonna fill local network pool
                        global_participants.insert(pid, tlid::Pool::new_full());
                    }
                    //grab a range for controller
                    let global_part_pool = global_participants.get_mut(&pid).unwrap();

                    sender_controller_sids
                        .remove_from(global_part_pool)
                        .unwrap();
                    sender_worker_sids.remove_from(global_part_pool).unwrap();
                    tlid::RemoveAllocation::new(&mut receiver_controller_sids)
                        .remove_from(global_part_pool)
                        .unwrap();
                    tlid::RemoveAllocation::new(&mut receiver_worker_sids)
                        .remove_from(global_part_pool)
                        .unwrap();

                    worker_participants.insert(pid.clone(), receiver_worker_sids);
                    if let Err(err) = rtrn_tx.send(RtrnMsg::ConnectedParticipant {
                        pid,
                        controller_sids: receiver_controller_sids,
                    }) {
                        error!(?err, "couldn't notify, is network already closed ?");
                    }
                    if let Some(send) = &self.return_pid_to {
                        if let Err(err) = send.send(pid) {
                            error!(
                                ?err,
                                "couldn't notify of connected participant, is network already \
                                 closed ?"
                            );
                        }
                    };
                    self.return_pid_to = None;
                } else {
                    warn!(
                        "a known participant opened an additional channel, UNCHECKED BECAUSE NO \
                         TOKEN WAS IMPLEMENTED IN THE HANDSHAKE!"
                    );
                }
                info!("recv config. This channel is now configured!");
            },
            Frame::Shutdown => {
                self.recv_shutdown = true;
                info!("shutting down channel");
                if let Err(err) = rtrn_tx.send(RtrnMsg::Shutdown) {
                    error!(?err, "couldn't notify of shutdown");
                }
            },
            Frame::OpenStream {
                sid,
                prio,
                promises,
            } => {
                if let Some(pid) = self.remote_pid {
                    let (msg_tx, msg_rx) = futures::channel::mpsc::unbounded::<InCommingMessage>();
                    let stream = IntStream::new(sid, prio, promises.clone(), msg_tx);

                    trace!(?self.streams, "-OPEN STREAM- going to modify streams");
                    self.streams.push(stream);
                    trace!(?self.streams, "-OPEN STREAM- did to modify streams");
                    info!("opened a stream");
                    if let Err(err) = rtrn_tx.send(RtrnMsg::OpendStream {
                        pid,
                        sid,
                        prio,
                        msg_rx,
                        promises,
                    }) {
                        error!(?err, "couldn't notify of opened stream");
                    }
                } else {
                    error!("called OpenStream before PartcipantID!");
                }
            },
            Frame::CloseStream { sid } => {
                if let Some(pid) = self.remote_pid {
                    trace!(?self.streams, "-CLOSE STREAM- going to modify streams");
                    self.streams.retain(|stream| stream.sid() != sid);
                    trace!(?self.streams, "-CLOSE STREAM- did to modify streams");
                    info!("closed a stream");
                    if let Err(err) = rtrn_tx.send(RtrnMsg::ClosedStream { pid, sid }) {
                        error!(?err, "couldn't notify of closed stream");
                    }
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
                start: _, //TODO: use start to verify!
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
                        let sid = s.sid();
                        let mut tx = s.msg_tx();
                        for m in s.to_receive.drain(pos..pos + 1) {
                            info!(?sid, ? m.mid, "received message");
                            //TODO: I dislike that block_on here!
                            block_on(async {
                                if let Err(err) = tx.send(m).await {
                                    error!(
                                        ?err,
                                        "cannot notify that message was received, probably stream \
                                         is already closed"
                                    );
                                };
                            });
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
    fn tick_streams(&mut self) {
        //ignoring prio for now
        //TODO: fix prio
        for s in &mut self.streams {
            let mut remove = false;
            let sid = s.sid();
            if let Some(m) = s.to_send.front_mut() {
                let to_send = std::cmp::min(m.buffer.data.len() as u64 - m.cursor, 1400);
                if to_send > 0 {
                    if m.cursor == 0 {
                        let mid = s.mid_pool.next();
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

    fn wrong_shutdown(&mut self, raw: &[u8]) {
        #[cfg(debug_assertions)]
        {
            debug!("sending client instructions before killing");
            self.send_queue.push_back(Frame::Raw(raw.to_vec()));
            self.send_queue.push_back(Frame::Shutdown {});
            self.send_shutdown = true;
        }
    }

    pub(crate) fn open_stream(
        &mut self,
        sid: Sid,
        prio: u8,
        promises: EnumSet<Promise>,
        msg_tx: futures::channel::mpsc::UnboundedSender<InCommingMessage>,
    ) {
        // validate promises
        trace!(?sid, "going to open a new stream");
        let stream = IntStream::new(sid, prio, promises.clone(), msg_tx);
        trace!(?sid, "1");
        self.streams.push(stream);
        trace!(?sid, "2");
        trace!(?self.streams, ?self.randomno, "2b");
        if self.streams.len() >= 0 {
            // breakpoint here
            let a = self.streams.len();
            if a > 1000 {
                //this will never happen but is a blackbox to catch a
                panic!("dasd");
            }
        }
        self.send_queue.push_back(Frame::OpenStream {
            sid,
            prio,
            promises,
        });
    }

    pub(crate) fn close_stream(&mut self, sid: Sid) {
        trace!(?self.streams, "--CLOSE STREAM-- going to modify streams");
        self.streams.retain(|stream| stream.sid() != sid);
        trace!(?self.streams, "--CLOSE STREAM-- did to modify streams");
        self.send_queue.push_back(Frame::CloseStream { sid });
    }

    pub(crate) fn handshake(&mut self) {
        self.send_queue.push_back(Frame::Handshake {
            magic_number: VELOREN_MAGIC_NUMBER.to_string(),
            version: VELOREN_NETWORK_VERSION,
        });
        self.send_handshake = true;
    }

    pub(crate) fn shutdown(&mut self) {
        self.send_queue.push_back(Frame::Shutdown {});
        self.send_shutdown = true;
    }

    pub(crate) fn send(&mut self, outgoing: OutGoingMessage) {
        trace!(?outgoing.sid, "3");
        trace!(?self.streams, ?self.randomno, "3b");

        for s in self.streams.iter_mut() {
            if s.sid() == outgoing.sid {
                s.to_send.push_back(outgoing);
                return;
            }
        }
        trace!(?outgoing.sid, "4");
        let sid = &outgoing.sid;
        error!(?sid, "couldn't send message, didn't found sid")
    }

    pub(crate) fn get_protocol(&self) -> &ChannelProtocols { &self.protocol }
}
