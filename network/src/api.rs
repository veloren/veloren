use crate::{
    internal::Channel,
    message::{self, Message},
    mio_worker::{CtrlMsg, MioWorker, TokenObjects},
    tcp_channel::TcpChannel,
};
use enumset::*;
use mio::{
    self,
    net::{TcpListener, TcpStream},
    PollOpt, Ready,
};
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, sync::Arc};
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

pub struct Stream {}

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
    mio_workers: Arc<Vec<MioWorker>>,
    thread_pool: Arc<ThreadPool>,
    participant_id: Uuid,
    _pe: PhantomData<E>,
}

impl<E: Events> Network<E> {
    pub fn new(participant_id: Uuid, thread_pool: Arc<ThreadPool>) -> Self {
        let mut token_pool = tlid::Pool::new_full();
        let mio_workers = Arc::new(vec![MioWorker::new(
            (participant_id.as_u128().rem_euclid(1024)) as u64,
            participant_id,
            thread_pool.clone(),
            token_pool.subpool(1000000).unwrap(),
        )]);
        Self {
            token_pool,
            mio_workers,
            thread_pool,
            participant_id,
            _pe: PhantomData::<E> {},
        }
    }

    fn get_lowest_worker<'a: 'b, 'b>(list: &'a Arc<Vec<MioWorker>>) -> &'a MioWorker { &list[0] }

    pub fn send<'a, M: Message<'a>>(&self, msg: M, stream: &Stream) {
        let messagebuffer = message::serialize(&msg);
    }

    pub fn listen(&self, addr: &Address) {
        let worker = Self::get_lowest_worker(&self.mio_workers);
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
        let worker = Self::get_lowest_worker(&self.mio_workers);
        let pipe = worker.get_tx();
        let address = addr.clone();
        let pid = self.participant_id;
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
                    let mut channel = TcpChannel::new(tcp_stream);
                    channel.handshake();
                    channel.participant_id(pid);
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
        for worker in self.mio_workers.iter() {
            worker.get_tx().send(CtrlMsg::OpenStream {
                pid: uuid::Uuid::new_v4(),
                prio,
                promises,
            });
        }
        Stream {}
    }

    pub fn close(&self, stream: Stream) {}
}
