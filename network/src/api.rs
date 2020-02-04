use crate::{
    internal::RemoteParticipant,
    message::{self, OutGoingMessage},
    worker::{
        channel::Channel,
        tcp::TcpChannel,
        types::{CtrlMsg, Pid, RtrnMsg, Sid, TokenObjects},
        Controller,
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
        let controller = Arc::new(vec![Controller::new(
            worker_pool.next(),
            participant_id,
            thread_pool.clone(),
            token_pool.subpool(1000000).unwrap(),
            remotes.clone(),
        )]);
        Self {
            token_pool,
            worker_pool,
            controller,
            thread_pool,
            participant_id,
            remotes,
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
            worker.get_tx().send(CtrlMsg::Send(OutGoingMessage {
                buffer: messagebuffer.clone(),
                cursor: 0,
                mid: None,
                sid: stream.sid,
            }));
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

    pub fn listen(&self, addr: &Address) {
        let worker = Self::get_lowest_worker(&self.controller);
        let pipe = worker.get_tx();
        let address = addr.clone();
        self.thread_pool.execute(move || {
            let span = span!(Level::INFO, "listen", ?address);
            let _enter = span.enter();
            match address {
                Address::Tcp(a) => {
                    info!("listening");
                    let tcp_listener = TcpListener::bind(&a).unwrap();
                    pipe.send(CtrlMsg::Register(
                        TokenObjects::TcpListener(tcp_listener),
                        Ready::readable(),
                        PollOpt::edge(),
                    ))
                    .unwrap();
                },
                Address::Udp(_) => unimplemented!("lazy me"),
            }
        });
    }

    pub fn connect(&self, addr: &Address) -> Participant {
        let worker = Self::get_lowest_worker(&self.controller);
        let pipe = worker.get_tx();
        let address = addr.clone();
        let pid = self.participant_id;
        let remotes = self.remotes.clone();
        self.thread_pool.execute(move || {
            let mut span = span!(Level::INFO, "connect", ?address);
            let _enter = span.enter();
            match address {
                Address::Tcp(a) => {
                    info!("connecting");
                    let tcp_stream = match TcpStream::connect(&a) {
                        Err(err) => {
                            error!("could not open connection: {}", err);
                            return;
                        },
                        Ok(s) => s,
                    };
                    let mut channel = TcpChannel::new(tcp_stream, pid, remotes);
                    pipe.send(CtrlMsg::Register(
                        TokenObjects::TcpChannel(channel),
                        Ready::readable() | Ready::writable(),
                        PollOpt::edge(),
                    ))
                    .unwrap();
                },
                Address::Udp(_) => unimplemented!("lazy me"),
            }
        });
        Participant { addr: addr.clone() }
    }

    pub fn open(&self, part: Participant, prio: u8, promises: EnumSet<Promise>) -> Stream {
        for worker in self.controller.iter() {
            worker.get_tx().send(CtrlMsg::OpenStream {
                pid: uuid::Uuid::new_v4(),
                prio,
                promises,
            });
        }
        Stream { sid: 0 }
    }

    pub fn close(&self, stream: Stream) {}
}
