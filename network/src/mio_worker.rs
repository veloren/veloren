use crate::tcp_channel::TcpChannel;
use mio::{self, net::TcpListener, Poll, PollOpt, Ready, Token};
use rand::{self, seq::IteratorRandom};
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
    time::{Duration, Instant},
};
use tracing::{debug, error, info, span, trace, warn, Level};
use uvth::ThreadPool;

#[derive(Debug)]
pub(crate) enum TokenObjects {
    TcpListener(TcpListener),
    TcpChannel(TcpChannel),
}

pub(crate) struct MioTokens {
    next_token_id: usize,
    pub tokens: HashMap<Token, TokenObjects>, //TODO: move to Vec<Options> for faster lookup
}

impl MioTokens {
    pub fn new() -> Self {
        MioTokens {
            next_token_id: 10,
            tokens: HashMap::new(),
        }
    }

    pub fn construct(&mut self) -> Token {
        let tok = Token(self.next_token_id);
        self.next_token_id += 1;
        tok
    }

    pub fn insert(&mut self, tok: Token, obj: TokenObjects) {
        trace!(?tok, ?obj, "added new token");
        self.tokens.insert(tok, obj);
    }
}

// MioStatistics should be copied in order to not hold locks for long
#[derive(Clone, Default)]
pub struct MioStatistics {
    nano_wait: u128,
    nano_busy: u128,
}

/*
    The MioWorker runs in it's own thread,
    it has a given set of Channels to work with.
    It is monitored, and when it's thread is fully loaded it can be splitted up into 2 MioWorkers
*/
pub struct MioWorker {
    worker_tag: u64, /* only relevant for logs */
    poll: Arc<Poll>,
    mio_tokens: Arc<RwLock<MioTokens>>,
    mio_statistics: Arc<RwLock<MioStatistics>>,
    shutdown: Arc<AtomicBool>,
}

impl MioWorker {
    const CTRL_TOK: Token = Token(0);

    pub fn new(worker_tag: u64, thread_pool: Arc<ThreadPool>) -> Self {
        let poll = Arc::new(Poll::new().unwrap());
        let poll_clone = poll.clone();
        let mio_tokens = Arc::new(RwLock::new(MioTokens::new()));
        let mio_tokens_clone = mio_tokens.clone();
        let mio_statistics = Arc::new(RwLock::new(MioStatistics::default()));
        let mio_statistics_clone = mio_statistics.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let mw = MioWorker {
            worker_tag,
            poll,
            mio_tokens,
            mio_statistics,
            shutdown,
        };
        thread_pool.execute(move || {
            mio_worker(
                worker_tag,
                poll_clone,
                mio_tokens_clone,
                mio_statistics_clone,
                shutdown_clone,
            )
        });
        mw
    }

    pub fn get_load_ratio(&self) -> f32 {
        let statistics = self.mio_statistics.read().unwrap();
        statistics.nano_busy as f32 / (statistics.nano_busy + statistics.nano_wait + 1) as f32
    }

    //TODO: split 4->5 MioWorkers and merge 5->4 MioWorkers
    pub fn split(&self, worker_id: u64, thread_pool: Arc<ThreadPool>) -> Self {
        //fork off a second MioWorker and split load
        let second = MioWorker::new(worker_id, thread_pool);
        {
            let mut first_tokens = self.mio_tokens.write().unwrap();
            let mut second_tokens = second.mio_tokens.write().unwrap();
            let cnt = first_tokens.tokens.len() / 2;

            for (key, val) in first_tokens
                .tokens
                .drain()
                .choose_multiple(&mut rand::thread_rng(), cnt / 2)
            {
                second_tokens.tokens.insert(key, val);
            }
            info!(
                "split MioWorker with {} tokens. New MioWorker has now {} tokens",
                cnt,
                second_tokens.tokens.len()
            );
        }
        second
    }

    pub fn merge(&self, other: MioWorker) {
        //fork off a second MioWorker and split load
        let mut first_tokens = self.mio_tokens.write().unwrap();
        let mut second_tokens = other.mio_tokens.write().unwrap();
        let cnt = first_tokens.tokens.len();

        for (key, val) in second_tokens.tokens.drain() {
            first_tokens.tokens.insert(key, val);
        }
        info!(
            "merge MioWorker with {} tokens. New MioWorker has now {} tokens",
            cnt,
            first_tokens.tokens.len()
        );
    }

    pub(crate) fn register(&self, handle: TokenObjects, interest: Ready, opts: PollOpt) {
        let mut tokens = self.mio_tokens.write().unwrap();
        let tok = tokens.construct();
        match &handle {
            TokenObjects::TcpListener(h) => self.poll.register(h, tok, interest, opts).unwrap(),
            TokenObjects::TcpChannel(channel) => self
                .poll
                .register(&channel.stream, tok, interest, opts)
                .unwrap(),
        }
        trace!(?handle, ?tok, "registered");
        tokens.insert(tok, handle);
    }
}

impl Drop for MioWorker {
    fn drop(&mut self) { self.shutdown.store(true, Ordering::Relaxed); }
}

fn mio_worker(
    worker_tag: u64,
    poll: Arc<Poll>,
    mio_tokens: Arc<RwLock<MioTokens>>,
    mio_statistics: Arc<RwLock<MioStatistics>>,
    shutdown: Arc<AtomicBool>,
) {
    let mut events = mio::Events::with_capacity(1024);
    let span = span!(Level::INFO, "mio worker", ?worker_tag);
    let _enter = span.enter();
    while !shutdown.load(Ordering::Relaxed) {
        let time_before_poll = Instant::now();
        if let Err(err) = poll.poll(&mut events, Some(Duration::from_millis(1000))) {
            error!("network poll error: {}", err);
            return;
        }
        let time_after_poll = Instant::now();

        if !events.is_empty() {
            let mut mio_tokens = mio_tokens.write().unwrap();
            for event in &events {
                match mio_tokens.tokens.get_mut(&event.token()) {
                    Some(e) => {
                        trace!(?event, "event");
                        match e {
                            TokenObjects::TcpListener(listener) => match listener.accept() {
                                Ok((mut remote_stream, _)) => {
                                    info!(?remote_stream, "remote connected");
                                    remote_stream.write_all("Hello Client".as_bytes()).unwrap();
                                    remote_stream.flush().unwrap();

                                    let tok = mio_tokens.construct();
                                    poll.register(
                                        &remote_stream,
                                        tok,
                                        Ready::readable() | Ready::writable(),
                                        PollOpt::edge(),
                                    )
                                    .unwrap();
                                    trace!(?remote_stream, ?tok, "registered");
                                    mio_tokens.tokens.insert(
                                        tok,
                                        TokenObjects::TcpChannel(TcpChannel::new(remote_stream)),
                                    );
                                },
                                Err(err) => {
                                    error!(?err, "error during remote connected");
                                },
                            },
                            TokenObjects::TcpChannel(channel) => {
                                if event.readiness().is_readable() {
                                    trace!(?channel.stream, "stream readable");
                                    //TODO: read values here and put to message assembly
                                    let mut buf: [u8; 1500] = [0; 1500];
                                    match channel.stream.read(&mut buf) {
                                        Ok(n) => {
                                            warn!("incomming message with len: {}", n);
                                            channel
                                                .to_receive
                                                .write()
                                                .unwrap()
                                                .push_back(buf.to_vec());
                                        },
                                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                            debug!("would block");
                                        },
                                        Err(e) => {
                                            panic!("{}", e);
                                        },
                                    };
                                }
                                if event.readiness().is_writable() {
                                    debug!(?channel.stream, "stream writeable");
                                    let mut to_send = channel.to_send.write().unwrap();
                                    if let Some(mut data) = to_send.pop_front() {
                                        let total = data.len();
                                        match channel.stream.write(&data) {
                                            Ok(n) if n == total => {},
                                            Ok(n) => {
                                                debug!("could only send part");
                                                let data = data.drain(n..).collect(); //TODO: validate n.. is correct
                                                to_send.push_front(data);
                                            },
                                            Err(e)
                                                if e.kind() == std::io::ErrorKind::WouldBlock =>
                                            {
                                                debug!("would block");
                                            }
                                            Err(e) => {
                                                panic!("{}", e);
                                            },
                                        };
                                    };
                                }
                            },
                            _ => unimplemented!("still lazy me"),
                        }
                    },
                    None => panic!("Unexpected event token '{:?}'", &event.token()),
                };
            }
        }
        let time_after_work = Instant::now();
        match mio_statistics.try_write() {
            Ok(mut mio_statistics) => {
                const OLD_KEEP_FACTOR: f64 = 0.995;
                //in order to weight new data stronger than older we fade them out with a
                // factor < 1. for 0.995 under full load (500 ticks a 1ms) we keep 8% of the old
                // value this means, that we start to see load comming up after
                // 500ms, but not for small spikes - as reordering for smaller spikes would be
                // to slow
                mio_statistics.nano_wait = (mio_statistics.nano_wait as f64 * OLD_KEEP_FACTOR)
                    as u128
                    + time_after_poll.duration_since(time_before_poll).as_nanos();
                mio_statistics.nano_busy = (mio_statistics.nano_busy as f64 * OLD_KEEP_FACTOR)
                    as u128
                    + time_after_work.duration_since(time_after_poll).as_nanos();

                trace!(
                    "current Load {}",
                    mio_statistics.nano_busy as f32
                        / (mio_statistics.nano_busy + mio_statistics.nano_wait + 1) as f32
                );
            },
            Err(e) => warn!("statistics dropped because they are currently accecssed"),
        }
    }
}
