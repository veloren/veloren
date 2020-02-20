use crate::{
    internal::RemoteParticipant,
    message::{self, OutGoingMessage},
    worker::{
        channel::ChannelProtocols,
        metrics::NetworkMetrics,
        types::{CtrlMsg, Pid, RtrnMsg, Sid, TokenObjects},
        Channel, Controller, TcpChannel,
    },
};
use enumset::*;
use mio::{
    self,
    net::{TcpListener, TcpStream},
    PollOpt, Ready,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{mpsc::TryRecvError, Arc, RwLock},
};
use tlid;
use tracing::*;
use uuid::Uuid;
use uvth::ThreadPool;

#[derive(Clone, Debug)]
pub enum Address {
    Tcp(std::net::SocketAddr),
    Udp(std::net::SocketAddr),
}

#[derive(Serialize, Deserialize, EnumSetType, Debug)]
#[enumset(serialize_repr = "u8")]
pub enum Promise {
    InOrder,
    NoCorrupt,
    GuaranteedDelivery,
    Encrypted,
}

pub struct Participant {
    addr: Address,
    remote_pid: Pid,
}

pub struct Connection {}

pub struct Stream {
    sid: Sid,
}

pub trait Events {
    fn on_remote_connection_open(net: &Network<Self>, con: &Connection)
    where
        Self: std::marker::Sized;
    fn on_remote_connection_close(net: &Network<Self>, con: &Connection)
    where
        Self: std::marker::Sized;
    fn on_remote_stream_open(net: &Network<Self>, st: &Stream)
    where
        Self: std::marker::Sized;
    fn on_remote_stream_close(net: &Network<Self>, st: &Stream)
    where
        Self: std::marker::Sized;
}

pub struct Network<E: Events> {
    token_pool: tlid::Pool<tlid::Wrapping<usize>>,
    worker_pool: tlid::Pool<tlid::Wrapping<u64>>,
    controller: Arc<Vec<Controller>>,
    thread_pool: Arc<ThreadPool>,
    participant_id: Pid,
    remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
    metrics: Arc<Option<NetworkMetrics>>,
    _pe: PhantomData<E>,
}

impl<E: Events> Network<E> {
    pub fn new(participant_id: Uuid, thread_pool: Arc<ThreadPool>) -> Self {
        let mut token_pool = tlid::Pool::new_full();
        let mut worker_pool = tlid::Pool::new_full();
        let remotes = Arc::new(RwLock::new(HashMap::new()));
        for _ in 0..participant_id.as_u128().rem_euclid(64) {
            worker_pool.next();
            //random offset from 0 for tests where multiple networks are
            // created and we do not want to polute the traces with
            // network pid everytime
        }
        let metrics = Arc::new(None);
        let controller = Arc::new(vec![Controller::new(
            worker_pool.next(),
            participant_id,
            thread_pool.clone(),
            token_pool.subpool(1000000).unwrap(),
            metrics.clone(),
            remotes.clone(),
        )]);
        Self {
            token_pool,
            worker_pool,
            controller,
            thread_pool,
            participant_id,
            remotes,
            metrics,
            _pe: PhantomData::<E> {},
        }
    }

    fn get_lowest_worker<'a: 'b, 'b>(list: &'a Arc<Vec<Controller>>) -> &'a Controller { &list[0] }

    pub fn send<M: Serialize>(&self, msg: M, stream: &Stream) {
        let messagebuffer = Arc::new(message::serialize(&msg));
        //transfer message to right worker to right channel to correct stream
        //TODO: why do we need a look here, i want my own local directory which is
        // updated by workes via a channel and needs to be intepreted on a send but it
        // should almost ever be empty except for new channel creations and stream
        // creations!
        for worker in self.controller.iter() {
            worker
                .get_tx()
                .send(CtrlMsg::Send(OutGoingMessage {
                    buffer: messagebuffer.clone(),
                    cursor: 0,
                    mid: None,
                    sid: stream.sid,
                }))
                .unwrap();
        }
    }

    pub fn recv<M: DeserializeOwned>(&self, stream: &Stream) -> Option<M> {
        for worker in self.controller.iter() {
            let msg = match worker.get_rx().try_recv() {
                Ok(msg) => msg,
                Err(TryRecvError::Empty) => {
                    return None;
                },
                Err(err) => {
                    panic!("Unexpected error '{}'", err);
                },
            };

            match msg {
                RtrnMsg::Receive(m) => {
                    info!("delivering a message");
                    return Some(message::deserialize(m.buffer));
                },
                _ => unimplemented!("woopsie"),
            }
        }
        None
    }

    pub fn open(&self, part: &Participant, prio: u8, promises: EnumSet<Promise>) -> Stream {
        let (ctrl_tx, ctrl_rx) = std::sync::mpsc::channel::<Sid>();
        for controller in self.controller.iter() {
            controller
                .get_tx()
                .send(CtrlMsg::OpenStream {
                    pid: part.remote_pid,
                    prio,
                    promises,
                    return_sid: ctrl_tx,
                })
                .unwrap();
            break;
        }
        // I dont like the fact that i need to wait on the worker thread for getting my
        // sid back :/ we could avoid this by introducing a Thread Local Network
        // which owns some sids we can take without waiting
        let sid = ctrl_rx.recv().unwrap();
        info!(?sid, " sucessfully opened stream");
        Stream { sid }
    }

    pub fn close(&self, stream: Stream) {}

    pub async fn listen(&self, address: &Address) -> Result<(), NetworkError> {
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
            Address::Udp(_) => unimplemented!("lazy me"),
        };
        Ok(())
    }

    pub async fn connect(&self, address: &Address) -> Result<Participant, NetworkError> {
        let worker = Self::get_lowest_worker(&self.controller);
        let pid = self.participant_id;
        let remotes = self.remotes.clone();
        let mut span = span!(Level::INFO, "connect", ?address);
        let _enter = span.enter();
        match address {
            Address::Tcp(a) => {
                info!("connecting");
                let tcp_stream = TcpStream::connect(&a)?;
                let tcp_channel = TcpChannel::new(tcp_stream);
                let (ctrl_tx, ctrl_rx) = std::sync::mpsc::channel::<Pid>();
                let mut channel = Channel::new(
                    pid,
                    ChannelProtocols::Tcp(tcp_channel),
                    remotes,
                    Some(ctrl_tx),
                );
                worker.get_tx().send(CtrlMsg::Register(
                    TokenObjects::Channel(channel),
                    Ready::readable() | Ready::writable(),
                    PollOpt::edge(),
                ))?;
                let remote_pid = ctrl_rx.recv().unwrap();
                info!(?remote_pid, " sucessfully connected to");
                return Ok(Participant {
                    addr: address.clone(),
                    remote_pid,
                });
            },
            Address::Udp(_) => unimplemented!("lazy me"),
        }
        Err(NetworkError::Todo_Error_For_Wrong_Connection)
    }

    //TODO: evaluate if move to Participant
    pub async fn _disconnect(&self, participant: Participant) -> Result<(), NetworkError> {
        panic!("sda");
    }

    pub fn participants(&self) -> Vec<Participant> {
        panic!("sda");
    }

    pub async fn _connected(&self) -> Result<Participant, NetworkError> {
        // returns if a Participant connected and is ready
        panic!("sda");
    }

    pub async fn _disconnected(&self) -> Result<Participant, NetworkError> {
        // returns if a Participant connected and is ready
        panic!("sda");
    }

    pub async fn multisend<M: Serialize>(
        &self,
        streams: Vec<Stream>,
        msg: M,
    ) -> Result<(), NetworkError> {
        panic!("sda");
    }
}

impl Participant {
    pub async fn _open(
        &self,
        prio: u8,
        promises: EnumSet<Promise>,
    ) -> Result<Stream, ParticipantError> {
        panic!("sda");
    }

    pub async fn _close(&self, stream: Stream) -> Result<(), ParticipantError> {
        panic!("sda");
    }

    pub async fn _opened(&self) -> Result<Stream, ParticipantError> {
        panic!("sda");
    }

    pub async fn _closed(&self) -> Result<Stream, ParticipantError> {
        panic!("sda");
    }
}

impl Stream {
    //TODO: What about SEND instead of Serializeable if it goes via PIPE ?
    //TODO: timeout per message or per stream ? stream or ?

    pub async fn _send<M: Serialize>(&self, msg: M) -> Result<(), StreamError> {
        panic!("sda");
    }

    pub async fn _recv<M: DeserializeOwned>(&self) -> Result<M, StreamError> {
        panic!("sda");
    }
}

#[derive(Debug)]
pub enum NetworkError {
    NetworkDestroyed,
    WorkerDestroyed,
    Todo_Error_For_Wrong_Connection,
    IoError(std::io::Error),
}

#[derive(Debug)]
pub enum ParticipantError {
    ParticipantDisconected,
}

#[derive(Debug)]
pub enum StreamError {
    StreamClosed,
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self { NetworkError::IoError(err) }
}

impl<T> From<mio_extras::channel::SendError<T>> for NetworkError {
    fn from(err: mio_extras::channel::SendError<T>) -> Self { NetworkError::WorkerDestroyed }
}
