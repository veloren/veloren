use crate::{
    channel::{Channel, ChannelProtocols},
    controller::Controller,
    message::{self, InCommingMessage, OutGoingMessage},
    metrics::NetworkMetrics,
    mpsc::MpscChannel,
    tcp::TcpChannel,
    types::{CtrlMsg, Pid, Sid, TokenObjects},
};
use enumset::*;
use futures::stream::StreamExt;
use mio::{
    self,
    net::{TcpListener, TcpStream},
    PollOpt, Ready,
};
use mio_extras;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, mpsc, Arc, Mutex, RwLock},
};
use tlid;
use tracing::*;
use uuid::Uuid;
use uvth::ThreadPool;

#[derive(Clone, Debug)]
pub enum Address {
    Tcp(std::net::SocketAddr),
    Udp(std::net::SocketAddr),
    Mpsc(u64),
}

#[derive(Serialize, Deserialize, EnumSetType, Debug)]
#[enumset(serialize_repr = "u8")]
pub enum Promise {
    InOrder,
    NoCorrupt,
    GuaranteedDelivery,
    Encrypted,
}

#[derive(Clone)]
pub struct Participant {
    remote_pid: Pid,
    network_controller: Arc<Vec<Controller>>,
}

pub struct Stream {
    sid: Sid,
    remote_pid: Pid,
    closed: AtomicBool,
    closed_rx: mpsc::Receiver<()>,
    msg_rx: futures::channel::mpsc::UnboundedReceiver<InCommingMessage>,
    ctr_tx: mio_extras::channel::Sender<CtrlMsg>,
}

pub struct Network {
    _token_pool: tlid::Pool<tlid::Wrapping<usize>>,
    _worker_pool: tlid::Pool<tlid::Wrapping<u64>>,
    controller: Arc<Vec<Controller>>,
    _thread_pool: Arc<ThreadPool>,
    participant_id: Pid,
    sid_backup_per_participant: Arc<RwLock<HashMap<Pid, tlid::Pool<tlid::Checked<Sid>>>>>,
    participants: RwLock<Vec<Participant>>,
    _metrics: Arc<Option<NetworkMetrics>>,
}

impl Network {
    pub fn new(participant_id: Uuid, thread_pool: Arc<ThreadPool>) -> Self {
        let mut token_pool = tlid::Pool::new_full();
        let mut worker_pool = tlid::Pool::new_full();
        let sid_backup_per_participant = Arc::new(RwLock::new(HashMap::new()));
        for _ in 0..participant_id.as_u128().rem_euclid(64) {
            worker_pool.next();
            //random offset from 0 for tests where multiple networks are
            // created and we do not want to polute the traces with
            // network pid everywhere
        }
        let metrics = Arc::new(None);
        let controller = Arc::new(vec![Controller::new(
            worker_pool.next(),
            participant_id,
            thread_pool.clone(),
            token_pool.subpool(1000000).unwrap(),
            metrics.clone(),
            sid_backup_per_participant.clone(),
        )]);
        let participants = RwLock::new(vec![]);
        Self {
            _token_pool: token_pool,
            _worker_pool: worker_pool,
            controller,
            _thread_pool: thread_pool,
            participant_id,
            sid_backup_per_participant,
            participants,
            _metrics: metrics,
        }
    }

    fn get_lowest_worker<'a: 'b, 'b>(list: &'a Arc<Vec<Controller>>) -> &'a Controller { &list[0] }

    pub fn listen(&self, address: &Address) -> Result<(), NetworkError> {
        let span = span!(Level::TRACE, "listen", ?address);
        let worker = Self::get_lowest_worker(&self.controller);
        let _enter = span.enter();
        match address {
            Address::Tcp(a) => {
                let tcp_listener = TcpListener::bind(&a)?;
                info!("listening");
                worker.get_tx().send(CtrlMsg::Register(
                    TokenObjects::TcpListener(tcp_listener),
                    Ready::readable(),
                    PollOpt::edge(),
                ))?;
            },
            Address::Udp(_) => unimplemented!(
                "UDP is currently not supportet problem is in internal worker - channel view. I \
                 except to have every Channel it#s own socket, but UDP shares a Socket with \
                 everyone on it. So there needs to be a instance that detects new connections \
                 inside worker and then creates a new channel for them, while handling needs to \
                 be done in UDP layer... however i am to lazy to build it yet."
            ),
            Address::Mpsc(a) => {
                let (listen_tx, listen_rx) = mio_extras::channel::channel();
                let (connect_tx, conntect_rx) = mio_extras::channel::channel();
                let mut registry = (*crate::mpsc::MPSC_REGISTRY).write().unwrap();
                registry.insert(*a, Mutex::new((listen_tx, conntect_rx)));
                info!("listening");
                let mpsc_channel = MpscChannel::new(connect_tx, listen_rx);
                let mut channel = Channel::new(
                    self.participant_id,
                    ChannelProtocols::Mpsc(mpsc_channel),
                    self.sid_backup_per_participant.clone(),
                    None,
                );
                channel.handshake();
                channel.tick_send();
                worker.get_tx().send(CtrlMsg::Register(
                    TokenObjects::Channel(channel),
                    Ready::readable() | Ready::writable(),
                    PollOpt::edge(),
                ))?;
            },
        };
        Ok(())
    }

    pub async fn connect(&self, address: &Address) -> Result<Participant, NetworkError> {
        let worker = Self::get_lowest_worker(&self.controller);
        let sid_backup_per_participant = self.sid_backup_per_participant.clone();
        let span = span!(Level::INFO, "connect", ?address);
        let _enter = span.enter();
        match address {
            Address::Tcp(a) => {
                info!("connecting");
                let tcp_stream = TcpStream::connect(&a)?;
                let tcp_channel = TcpChannel::new(tcp_stream);
                let (ctrl_tx, ctrl_rx) = mpsc::channel::<Pid>();
                let channel = Channel::new(
                    self.participant_id,
                    ChannelProtocols::Tcp(tcp_channel),
                    sid_backup_per_participant,
                    Some(ctrl_tx),
                );
                worker.get_tx().send(CtrlMsg::Register(
                    TokenObjects::Channel(channel),
                    Ready::readable() | Ready::writable(),
                    PollOpt::edge(),
                ))?;
                let remote_pid = ctrl_rx.recv().unwrap();
                info!(?remote_pid, " sucessfully connected to");
                let part = Participant {
                    remote_pid,
                    network_controller: self.controller.clone(),
                };
                self.participants.write().unwrap().push(part.clone());
                return Ok(part);
            },
            Address::Udp(_) => unimplemented!("lazy me"),
            Address::Mpsc(a) => {
                let mut registry = (*crate::mpsc::MPSC_REGISTRY).write().unwrap();
                let (listen_tx, conntect_rx) = match registry.remove(a) {
                    Some(x) => x.into_inner().unwrap(),
                    None => {
                        error!("could not connect to mpsc");
                        return Err(NetworkError::NetworkDestroyed);
                    },
                };
                info!("connect to mpsc");
                let mpsc_channel = MpscChannel::new(listen_tx, conntect_rx);
                let (ctrl_tx, ctrl_rx) = mpsc::channel::<Pid>();
                let channel = Channel::new(
                    self.participant_id,
                    ChannelProtocols::Mpsc(mpsc_channel),
                    self.sid_backup_per_participant.clone(),
                    Some(ctrl_tx),
                );
                worker.get_tx().send(CtrlMsg::Register(
                    TokenObjects::Channel(channel),
                    Ready::readable() | Ready::writable(),
                    PollOpt::edge(),
                ))?;

                let remote_pid = ctrl_rx.recv().unwrap();
                info!(?remote_pid, " sucessfully connected to");
                let part = Participant {
                    remote_pid,
                    network_controller: self.controller.clone(),
                };
                self.participants.write().unwrap().push(part.clone());
                return Ok(part);
            },
        }
    }

    pub fn disconnect(&self, _participant: Participant) -> Result<(), NetworkError> {
        //todo: close all channels to a participant!
        unimplemented!("sda");
    }

    pub fn participants(&self) -> std::sync::RwLockReadGuard<Vec<Participant>> {
        self.participants.read().unwrap()
    }

    pub async fn connected(&self) -> Result<Participant, NetworkError> {
        // returns if a Participant connected and is ready
        loop {
            //ARRGGG
            for worker in self.controller.iter() {
                //TODO harden!
                worker.tick();
                if let Ok(remote_pid) = worker.get_participant_connect_rx().try_recv() {
                    let part = Participant {
                        remote_pid,
                        network_controller: self.controller.clone(),
                    };
                    self.participants.write().unwrap().push(part.clone());
                    return Ok(part);
                };
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    pub fn multisend<M: Serialize>(
        &self,
        streams: Vec<Stream>,
        msg: M,
    ) -> Result<(), NetworkError> {
        let messagebuffer = Arc::new(message::serialize(&msg));
        //TODO: why do we need a look here, i want my own local directory which is
        // updated by workes via a channel and needs to be intepreted on a send but it
        // should almost ever be empty except for new channel creations and stream
        // creations!
        for stream in streams {
            stream
                .ctr_tx
                .send(CtrlMsg::Send(OutGoingMessage {
                    buffer: messagebuffer.clone(),
                    cursor: 0,
                    mid: None,
                    sid: stream.sid,
                }))
                .unwrap();
        }
        Ok(())
    }
}

impl Participant {
    pub fn open(&self, prio: u8, promises: EnumSet<Promise>) -> Result<Stream, ParticipantError> {
        let (msg_tx, msg_rx) = futures::channel::mpsc::unbounded::<InCommingMessage>();
        for controller in self.network_controller.iter() {
            //trigger tick:
            controller.tick();
            let parts = controller.participants();
            let (stream_close_tx, stream_close_rx) = mpsc::channel();
            let sid = match parts.get(&self.remote_pid) {
                Some(p) => {
                    let sid = p.sid_pool.write().unwrap().next();
                    //prepare the closing of the new stream already
                    p.stream_close_txs
                        .write()
                        .unwrap()
                        .insert(sid, stream_close_tx);
                    sid
                },
                None => return Err(ParticipantError::ParticipantDisconected), /* TODO: participant was never connected in the first case maybe... */
            };
            let tx = controller.get_tx();
            tx.send(CtrlMsg::OpenStream {
                pid: self.remote_pid,
                sid,
                prio,
                promises,
                msg_tx,
            })
            .unwrap();
            info!(?sid, " sucessfully opened stream");
            return Ok(Stream::new(
                sid,
                self.remote_pid,
                stream_close_rx,
                msg_rx,
                tx,
            ));
        }
        Err(ParticipantError::ParticipantDisconected)
    }

    pub async fn opened(&self) -> Result<Stream, ParticipantError> {
        //TODO: make this async native!
        loop {
            // Going to all workers in a network, but only receive on specific channels!
            for worker in self.network_controller.iter() {
                worker.tick();
                let parts = worker.participants();
                if let Some(p) = parts.get(&self.remote_pid) {
                    if let Ok(stream) = p.stream_open_rx.try_recv() {
                        //need a try, as i depend on the tick, it's the same thread...
                        debug!("delivering a stream");
                        return Ok(stream);
                    };
                }
            }
        }
    }
}

impl Stream {
    //TODO: What about SEND instead of Serializeable if it goes via PIPE ?
    //TODO: timeout per message or per stream ? stream or ? like for Position Data,
    // if not transmitted within 1 second, throw away...
    pub(crate) fn new(
        sid: Sid,
        remote_pid: Pid,
        closed_rx: mpsc::Receiver<()>,
        msg_rx: futures::channel::mpsc::UnboundedReceiver<InCommingMessage>,
        ctr_tx: mio_extras::channel::Sender<CtrlMsg>,
    ) -> Self {
        Self {
            sid,
            remote_pid,
            closed: AtomicBool::new(false),
            closed_rx,
            msg_rx,
            ctr_tx,
        }
    }

    pub fn send<M: Serialize>(&self, msg: M) -> Result<(), StreamError> {
        if self.is_closed() {
            return Err(StreamError::StreamClosed);
        }
        let messagebuffer = Arc::new(message::serialize(&msg));
        self.ctr_tx
            .send(CtrlMsg::Send(OutGoingMessage {
                buffer: messagebuffer,
                cursor: 0,
                mid: None,
                sid: self.sid,
            }))
            .unwrap();
        Ok(())
    }

    pub async fn recv<M: DeserializeOwned>(&mut self) -> Result<M, StreamError> {
        if self.is_closed() {
            return Err(StreamError::StreamClosed);
        }
        match self.msg_rx.next().await {
            Some(msg) => {
                info!(?msg, "delivering a message");
                Ok(message::deserialize(msg.buffer))
            },
            None => panic!(
                "Unexpected error, probably stream was destroyed... maybe i dont know yet, no \
                 idea of async stuff"
            ),
        }
    }

    pub fn close(mut self) -> Result<(), StreamError> { self.intclose() }

    fn is_closed(&self) -> bool {
        use core::sync::atomic::Ordering;
        if self.closed.load(Ordering::Relaxed) {
            true
        } else {
            if let Ok(()) = self.closed_rx.try_recv() {
                self.closed.store(true, Ordering::SeqCst); //TODO: Is this the right Ordering?
                true
            } else {
                false
            }
        }
    }

    fn intclose(&mut self) -> Result<(), StreamError> {
        use core::sync::atomic::Ordering;
        if self.is_closed() {
            return Err(StreamError::StreamClosed);
        }
        self.ctr_tx
            .send(CtrlMsg::CloseStream {
                pid: self.remote_pid,
                sid: self.sid,
            })
            .unwrap();
        self.closed.store(true, Ordering::SeqCst); //TODO: Is this the right Ordering?
        Ok(())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        let _ = self.intclose().map_err(
            |e| error!(?self.sid, ?e, "could not properly shutdown stream, which got out of scope"),
        );
    }
}

#[derive(Debug)]
pub enum NetworkError {
    NetworkDestroyed,
    WorkerDestroyed,
    IoError(std::io::Error),
}

#[derive(Debug, PartialEq)]
pub enum ParticipantError {
    ParticipantDisconected,
}

#[derive(Debug, PartialEq)]
pub enum StreamError {
    StreamClosed,
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self { NetworkError::IoError(err) }
}

impl<T> From<mio_extras::channel::SendError<T>> for NetworkError {
    fn from(_err: mio_extras::channel::SendError<T>) -> Self { NetworkError::WorkerDestroyed }
}
