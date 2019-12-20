use crate::{message::Message, protocol::Protocol};
use enumset::*;
use mio::{
    self,
    net::{TcpListener, TcpStream},
    Poll, PollOpt, Ready, Token,
};
use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, RwLock},
    time::Duration,
};
use uvth::{ThreadPool, ThreadPoolBuilder};

#[derive(Clone)]
pub enum Address {
    Tcp(std::net::SocketAddr),
    Udp(std::net::SocketAddr),
}

#[derive(EnumSetType, Debug)]
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
    fn OnRemoteConnectionOpen(net: &Network<Self>, con: &Connection)
    where
        Self: std::marker::Sized;
    fn OnRemoteConnectionClose(net: &Network<Self>, con: &Connection)
    where
        Self: std::marker::Sized;
    fn OnRemoteStreamOpen(net: &Network<Self>, st: &Stream)
    where
        Self: std::marker::Sized;
    fn OnRemoteStreamClose(net: &Network<Self>, st: &Stream)
    where
        Self: std::marker::Sized;
}

pub enum TokenObjects {
    TCP_LISTENER(TcpListener),
}

pub struct NetworkData {
    next_token_id: usize,
    tokens: HashMap<Token, TokenObjects>, //TODO: move to Vec<Options> for faster lookup
    poll: Poll,
}

pub struct Network<E: Events> {
    internal_sync: Arc<RwLock<NetworkData>>,
    thread_pool: ThreadPool,
    participant_id: u64,
    _pe: PhantomData<E>,
}

impl NetworkData {
    pub fn new() -> Self {
        NetworkData {
            next_token_id: 0,
            tokens: HashMap::new(),
            poll: Poll::new().unwrap(),
        }
    }
}

impl<E: Events> Network<E> {
    const TCP_LISTEN_TOK: Token = Token(0);

    pub fn new() -> Self {
        let thread_pool = ThreadPoolBuilder::new()
            .name("veloren-network".into())
            .build();
        let internal_sync = Arc::new(RwLock::new(NetworkData::new()));
        let internal_sync_clone = internal_sync.clone();
        thread_pool.execute(|| master_poll_worker(internal_sync_clone));
        Self {
            internal_sync,
            thread_pool,
            participant_id: 42,
            _pe: PhantomData::<E> {},
        }
    }

    pub fn send<'a, M: Message<'a>>(&self, msg: M, stream: &Stream) {}

    pub fn listen(&self, addr: &Address) {
        let addr = addr.clone();
        let internal_sync = self.internal_sync.clone();
        self.thread_pool.execute(move || match addr {
            Address::Tcp(a) => {
                let tcp_listener = TcpListener::bind(&a).unwrap();
                let mut internal_sync = internal_sync.write().unwrap();
                let tok = Token(internal_sync.next_token_id);
                internal_sync.next_token_id += 1;
                internal_sync
                    .poll
                    .register(&tcp_listener, tok, Ready::readable(), PollOpt::edge())
                    .unwrap();
                internal_sync
                    .tokens
                    .insert(tok, TokenObjects::TCP_LISTENER(tcp_listener));
            },
            Address::Udp(_) => unimplemented!("lazy me"),
        });
    }

    pub fn connect(&self, addr: &Address) -> Participant { Participant { addr: addr.clone() } }

    pub fn open(&self, part: Participant, prio: u8, prom: EnumSet<Promise>) -> Stream { Stream {} }

    pub fn close(&self, stream: Stream) {}
}

fn master_poll_worker(internal_sync: Arc<RwLock<NetworkData>>) {
    let mut events = mio::Events::with_capacity(1024);
    loop {
        let internal_sync = internal_sync.write().unwrap();
        if let Err(err) = internal_sync
            .poll
            .poll(&mut events, Some(Duration::from_millis(1)))
        {
            //postbox_tx.send(Err(err.into()))?;
            return;
        }

        for event in &events {
            match internal_sync.tokens.get(&event.token()) {
                Some(e) => {
                    match e {
                        TokenObjects::TCP_LISTENER(listener) => {
                            match listener.accept() {
                                Ok((stream, _)) => {}, /* PostBox::from_tcpstream(stream) */
                                Err(err) => {},        /* Err(err.into()) */
                            }
                        },
                    }
                },
                None => panic!("Unexpected event token '{:?}'", &event.token()),
            };
        }
    }
}

impl Address {
    pub fn getProtocol(&self) -> Protocol {
        match self {
            Address::Tcp(_) => Protocol::Tcp,
            Address::Udp(_) => Protocol::Udp,
        }
    }
}
