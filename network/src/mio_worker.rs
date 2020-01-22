use crate::{api::Promise, internal::Channel, message::OutGoingMessage, tcp_channel::TcpChannel};
use enumset::EnumSet;
use mio::{self, net::TcpListener, Poll, PollOpt, Ready, Token};
use mio_extras::channel::{channel, Receiver, Sender};
use std::{
    collections::HashMap,
    sync::{mpsc::TryRecvError, Arc, RwLock},
    time::Instant,
};
use tlid;
use tracing::{debug, error, info, span, trace, warn, Level};
use uvth::ThreadPool;

#[derive(Debug)]
pub(crate) enum TokenObjects {
    TcpListener(TcpListener),
    TcpChannel(TcpChannel),
}

pub(crate) struct MioTokens {
    pool: tlid::Pool<tlid::Wrapping<usize>>,
    pub tokens: HashMap<Token, TokenObjects>, //TODO: move to Vec<Options> for faster lookup
}

impl MioTokens {
    pub fn new(pool: tlid::Pool<tlid::Wrapping<usize>>) -> Self {
        MioTokens {
            pool,
            tokens: HashMap::new(),
        }
    }

    pub fn construct(&mut self) -> Token { Token(self.pool.next()) }

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

pub(crate) enum CtrlMsg {
    Shutdown,
    Register(TokenObjects, Ready, PollOpt),
    OpenStream {
        pid: uuid::Uuid,
        prio: u8,
        promises: EnumSet<Promise>,
    },
    CloseStream {
        pid: uuid::Uuid,
        sid: u32,
    },
    Send(OutGoingMessage),
}

/*
    The MioWorker runs in it's own thread,
    it has a given set of Channels to work with.
    It is monitored, and when it's thread is fully loaded it can be splitted up into 2 MioWorkers
*/
pub struct MioWorker {
    tag: u64, /* only relevant for logs */
    pid: uuid::Uuid,
    poll: Arc<Poll>,
    mio_statistics: Arc<RwLock<MioStatistics>>,
    ctrl_tx: Sender<CtrlMsg>,
}

impl MioWorker {
    pub const CTRL_TOK: Token = Token(0);

    pub fn new(
        tag: u64,
        pid: uuid::Uuid,
        thread_pool: Arc<ThreadPool>,
        mut token_pool: tlid::Pool<tlid::Wrapping<usize>>,
    ) -> Self {
        let poll = Arc::new(Poll::new().unwrap());
        let poll_clone = poll.clone();
        let mio_statistics = Arc::new(RwLock::new(MioStatistics::default()));
        let mio_statistics_clone = mio_statistics.clone();

        let (ctrl_tx, ctrl_rx) = channel();
        poll.register(&ctrl_rx, Self::CTRL_TOK, Ready::readable(), PollOpt::edge())
            .unwrap();
        // reserve 10 tokens in case they start with 0, //TODO: cleaner method
        for _ in 0..10 {
            token_pool.next();
        }

        let mw = MioWorker {
            tag,
            pid,
            poll,
            mio_statistics,
            ctrl_tx,
        };
        thread_pool.execute(move || {
            mio_worker(
                tag,
                pid,
                poll_clone,
                mio_statistics_clone,
                token_pool,
                ctrl_rx,
            )
        });
        mw
    }

    pub fn get_load_ratio(&self) -> f32 {
        let statistics = self.mio_statistics.read().unwrap();
        statistics.nano_busy as f32 / (statistics.nano_busy + statistics.nano_wait + 1) as f32
    }

    //TODO: split 4->5 MioWorkers and merge 5->4 MioWorkers

    pub(crate) fn get_tx(&self) -> Sender<CtrlMsg> { self.ctrl_tx.clone() }
}

impl Drop for MioWorker {
    fn drop(&mut self) { let _ = self.ctrl_tx.send(CtrlMsg::Shutdown); }
}

fn mio_worker(
    tag: u64,
    pid: uuid::Uuid,
    poll: Arc<Poll>,
    mio_statistics: Arc<RwLock<MioStatistics>>,
    mut token_pool: tlid::Pool<tlid::Wrapping<usize>>,
    ctrl_rx: Receiver<CtrlMsg>,
) {
    let mut mio_tokens = MioTokens::new(token_pool);
    let mut events = mio::Events::with_capacity(1024);
    let mut buf: [u8; 65000] = [0; 65000];
    let span = span!(Level::INFO, "mio worker", ?tag);
    let _enter = span.enter();
    loop {
        let time_before_poll = Instant::now();
        if let Err(err) = poll.poll(&mut events, None) {
            error!("network poll error: {}", err);
            return;
        }
        let time_after_poll = Instant::now();
        for event in &events {
            match event.token() {
                MioWorker::CTRL_TOK => {
                    if handle_ctl(&ctrl_rx, &mut mio_tokens, &poll, &mut buf, time_after_poll) {
                        return;
                    }
                },
                _ => handle_tok(
                    pid,
                    event,
                    &mut mio_tokens,
                    &poll,
                    &mut buf,
                    time_after_poll,
                ),
            };
        }
        handle_statistics(&mio_statistics, time_before_poll, time_after_poll);
    }
}

fn handle_ctl(
    ctrl_rx: &Receiver<CtrlMsg>,
    mio_tokens: &mut MioTokens,
    poll: &Arc<Poll>,
    buf: &mut [u8; 65000],
    time_after_poll: Instant,
) -> bool {
    match ctrl_rx.try_recv() {
        Ok(CtrlMsg::Shutdown) => {
            debug!("Shutting Down");
            return true;
        },
        Ok(CtrlMsg::Register(handle, interest, opts)) => {
            let tok = mio_tokens.construct();
            match &handle {
                TokenObjects::TcpListener(h) => poll.register(h, tok, interest, opts).unwrap(),
                TokenObjects::TcpChannel(channel) => poll
                    .register(&channel.tcpstream, tok, interest, opts)
                    .unwrap(),
            }
            debug!(?handle, ?tok, "Registered new handle");
            mio_tokens.insert(tok, handle);
        },
        Ok(CtrlMsg::OpenStream {
            pid,
            prio,
            promises,
        }) => {
            for (tok, obj) in mio_tokens.tokens.iter_mut() {
                if let TokenObjects::TcpChannel(channel) = obj {
                    channel.open_stream(prio, promises); //TODO: check participant
                    channel.write(buf, time_after_poll);
                }
            }
            //TODO:
        },
        Ok(CtrlMsg::CloseStream { pid, sid }) => {
            //TODO:
            for to in mio_tokens.tokens.values_mut() {
                if let TokenObjects::TcpChannel(channel) = to {
                    channel.close_stream(sid); //TODO: check participant
                    channel.write(buf, time_after_poll);
                }
            }
        },
        Ok(_) => unimplemented!("dad"),
        Err(TryRecvError::Empty) => {},
        Err(err) => {
            //postbox_tx.send(Err(err.into()))?;
            return true;
        },
    }
    false
}

fn handle_tok(
    pid: uuid::Uuid,
    event: mio::Event,
    mio_tokens: &mut MioTokens,
    poll: &Arc<Poll>,
    buf: &mut [u8; 65000],
    time_after_poll: Instant,
) {
    match mio_tokens.tokens.get_mut(&event.token()) {
        Some(e) => {
            trace!(?event, "event");
            match e {
                TokenObjects::TcpListener(listener) => match listener.accept() {
                    Ok((mut remote_stream, _)) => {
                        info!(?remote_stream, "remote connected");

                        let tok = mio_tokens.construct();
                        poll.register(
                            &remote_stream,
                            tok,
                            Ready::readable() | Ready::writable(),
                            PollOpt::edge(),
                        )
                        .unwrap();
                        trace!(?remote_stream, ?tok, "registered");
                        let mut channel = TcpChannel::new(remote_stream);
                        channel.handshake();
                        channel.participant_id(pid);

                        mio_tokens
                            .tokens
                            .insert(tok, TokenObjects::TcpChannel(channel));
                    },
                    Err(err) => {
                        error!(?err, "error during remote connected");
                    },
                },
                TokenObjects::TcpChannel(channel) => {
                    if event.readiness().is_readable() {
                        trace!(?channel.tcpstream, "stream readable");
                        channel.read(buf, time_after_poll);
                    }
                    if event.readiness().is_writable() {
                        trace!(?channel.tcpstream, "stream writeable");
                        channel.write(buf, time_after_poll);
                    }
                },
            }
        },
        None => panic!("Unexpected event token '{:?}'", &event.token()),
    };
}

fn handle_statistics(
    mio_statistics: &Arc<RwLock<MioStatistics>>,
    time_before_poll: Instant,
    time_after_poll: Instant,
) {
    let time_after_work = Instant::now();
    match mio_statistics.try_write() {
        Ok(mut mio_statistics) => {
            const OLD_KEEP_FACTOR: f64 = 0.995;
            //in order to weight new data stronger than older we fade them out with a
            // factor < 1. for 0.995 under full load (500 ticks a 1ms) we keep 8% of the old
            // value this means, that we start to see load comming up after
            // 500ms, but not for small spikes - as reordering for smaller spikes would be
            // to slow
            mio_statistics.nano_wait = (mio_statistics.nano_wait as f64 * OLD_KEEP_FACTOR) as u128
                + time_after_poll.duration_since(time_before_poll).as_nanos();
            mio_statistics.nano_busy = (mio_statistics.nano_busy as f64 * OLD_KEEP_FACTOR) as u128
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
