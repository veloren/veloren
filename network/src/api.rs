use crate::{
    channel::{Channel, ChannelProtocols},
    controller::Controller,
    message::{self, InCommingMessage, OutGoingMessage},
    metrics::NetworkMetrics,
    tcp::TcpChannel,
    types::{CtrlMsg, Pid, RemoteParticipant, RtrnMsg, Sid, TokenObjects},
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
    sync::{
        mpsc::{self, Receiver, TryRecvError},
        Arc, RwLock,
    },
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
    network_controller: Arc<Vec<Controller>>,
}

pub struct Connection {}

pub struct Stream {
    sid: Sid,
    msg_rx: Receiver<InCommingMessage>,
    ctr_tx: mio_extras::channel::Sender<CtrlMsg>,
}

pub struct Network {
    token_pool: tlid::Pool<tlid::Wrapping<usize>>,
    worker_pool: tlid::Pool<tlid::Wrapping<u64>>,
    controller: Arc<Vec<Controller>>,
    thread_pool: Arc<ThreadPool>,
    participant_id: Pid,
    remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
    metrics: Arc<Option<NetworkMetrics>>,
}

impl Network {
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
        }
    }

    fn get_lowest_worker<'a: 'b, 'b>(list: &'a Arc<Vec<Controller>>) -> &'a Controller { &list[0] }

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
                let (ctrl_tx, ctrl_rx) = mpsc::channel::<Pid>();
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
                    network_controller: self.controller.clone(),
                });
            },
            Address::Udp(_) => unimplemented!("lazy me"),
        }
    }

    //TODO: evaluate if move to Participant
    pub async fn _disconnect(&self, participant: Participant) -> Result<(), NetworkError> {
        panic!("sda");
    }

    pub fn participants(&self) -> Vec<Participant> {
        panic!("sda");
    }

    pub async fn connected(&self) -> Result<Participant, NetworkError> {
        // returns if a Participant connected and is ready
        loop {
            //ARRGGG
            for worker in self.controller.iter() {
                //TODO harden!
                if let Ok(msg) = worker.get_rx().try_recv() {
                    if let RtrnMsg::ConnectedParticipant { pid } = msg {
                        return Ok(Participant {
                            addr: Address::Tcp(std::net::SocketAddr::from(([1, 3, 3, 7], 1337))), /* TODO: FIXME */
                            remote_pid: pid,
                            network_controller: self.controller.clone(),
                        });
                    }
                };
            }
        }
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
    pub async fn open(
        &self,
        prio: u8,
        promises: EnumSet<Promise>,
    ) -> Result<Stream, ParticipantError> {
        let (ctrl_tx, ctrl_rx) = mpsc::channel::<Sid>();
        let (msg_tx, msg_rx) = mpsc::channel::<InCommingMessage>();
        for controller in self.network_controller.iter() {
            let tx = controller.get_tx();
            tx.send(CtrlMsg::OpenStream {
                pid: self.remote_pid,
                prio,
                promises,
                return_sid: ctrl_tx,
                msg_tx,
            })
            .unwrap();

            // I dont like the fact that i need to wait on the worker thread for getting my
            // sid back :/ we could avoid this by introducing a Thread Local Network
            // which owns some sids we can take without waiting
            let sid = ctrl_rx.recv().unwrap();
            info!(?sid, " sucessfully opened stream");
            return Ok(Stream {
                sid,
                msg_rx,
                ctr_tx: tx,
            });
        }
        Err(ParticipantError::ParticipantDisconected)
    }

    pub fn close(&self, stream: Stream) -> Result<(), ParticipantError> { Ok(()) }

    pub async fn opened(&self) -> Result<Stream, ParticipantError> {
        loop {
            //ARRGGG
            for worker in self.network_controller.iter() {
                //TODO harden!
                if let Ok(msg) = worker.get_rx().try_recv() {
                    if let RtrnMsg::OpendStream {
                        pid,
                        sid,
                        prio,
                        msg_rx,
                        promises,
                    } = msg
                    {
                        return Ok(Stream {
                            sid,
                            msg_rx,
                            ctr_tx: worker.get_tx(),
                        });
                    }
                };
            }
        }
    }

    pub async fn _closed(&self) -> Result<Stream, ParticipantError> {
        panic!("sda");
    }
}

impl Stream {
    //TODO: What about SEND instead of Serializeable if it goes via PIPE ?
    //TODO: timeout per message or per stream ? stream or ?

    pub fn send<M: Serialize>(&self, msg: M) -> Result<(), StreamError> {
        let messagebuffer = Arc::new(message::serialize(&msg));
        //transfer message to right worker to right channel to correct stream
        //TODO: why do we need a look here, i want my own local directory which is
        // updated by workes via a channel and needs to be intepreted on a send but it
        // should almost ever be empty except for new channel creations and stream
        // creations!
        self.ctr_tx
            .send(CtrlMsg::Send(OutGoingMessage {
                buffer: messagebuffer.clone(),
                cursor: 0,
                mid: None,
                sid: self.sid,
            }))
            .unwrap();
        Ok(())
    }

    pub async fn recv<M: DeserializeOwned>(&self) -> Result<M, StreamError> {
        match self.msg_rx.recv() {
            Ok(msg) => {
                info!(?msg, "delivering a message");
                Ok(message::deserialize(msg.buffer))
            },
            Err(err) => panic!("Unexpected error '{}'", err),
        }
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
    fn from(err: mio_extras::channel::SendError<T>) -> Self { NetworkError::WorkerDestroyed }
}
