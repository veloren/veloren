use crate::{
    internal::RemoteParticipant,
    worker::{
        types::{CtrlMsg, Pid, RtrnMsg, Statistics, TokenObjects},
        Channel, Controller, TcpChannel,
    },
};
use mio::{self, Poll, PollOpt, Ready, Token};
use mio_extras::channel::{Receiver, Sender};
use std::{
    collections::HashMap,
    sync::{mpsc::TryRecvError, Arc, RwLock},
    time::Instant,
};
use tlid;
use tracing::*;
/*
The worker lives in a own thread and only communcates with the outside via a Channel
*/

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

pub(crate) struct Worker {
    pid: Pid,
    poll: Arc<Poll>,
    statistics: Arc<RwLock<Statistics>>,
    remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
    ctrl_rx: Receiver<CtrlMsg>,
    rtrn_tx: Sender<RtrnMsg>,
    mio_tokens: MioTokens,
    time_before_poll: Instant,
    time_after_poll: Instant,
}

impl Worker {
    pub fn new(
        pid: Pid,
        poll: Arc<Poll>,
        statistics: Arc<RwLock<Statistics>>,
        remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
        token_pool: tlid::Pool<tlid::Wrapping<usize>>,
        ctrl_rx: Receiver<CtrlMsg>,
        rtrn_tx: Sender<RtrnMsg>,
    ) -> Self {
        let mio_tokens = MioTokens::new(token_pool);
        Worker {
            pid,
            poll,
            statistics,
            remotes,
            ctrl_rx,
            rtrn_tx,
            mio_tokens,
            time_before_poll: Instant::now(),
            time_after_poll: Instant::now(),
        }
    }

    pub fn run(&mut self) {
        let mut events = mio::Events::with_capacity(1024);
        loop {
            self.time_before_poll = Instant::now();
            if let Err(err) = self.poll.poll(&mut events, None) {
                error!("network poll error: {}", err);
                return;
            }
            self.time_after_poll = Instant::now();
            for event in &events {
                trace!(?event, "event");
                match event.token() {
                    Controller::CTRL_TOK => {
                        if self.handle_ctl() {
                            return;
                        }
                    },
                    _ => self.handle_tok(&event),
                };
            }
            self.handle_statistics();
        }
    }

    fn handle_ctl(&mut self) -> bool {
        let msg = match self.ctrl_rx.try_recv() {
            Ok(msg) => msg,
            Err(TryRecvError::Empty) => {
                return false;
            },
            Err(err) => {
                panic!("Unexpected error '{}'", err);
            },
        };

        match msg {
            CtrlMsg::Shutdown => {
                debug!("Shutting Down");
                for (tok, obj) in self.mio_tokens.tokens.iter_mut() {
                    if let TokenObjects::TcpChannel(channel, _) = obj {
                        channel.shutdown();
                        channel.tick_send();
                    }
                }
                return true;
            },
            CtrlMsg::Register(handle, interest, opts) => {
                let tok = self.mio_tokens.construct();
                match &handle {
                    TokenObjects::TcpListener(h) => {
                        self.poll.register(h, tok, interest, opts).unwrap()
                    },
                    TokenObjects::TcpChannel(channel, _) => self
                        .poll
                        .register(channel.get_handle(), tok, interest, opts)
                        .unwrap(),
                    TokenObjects::UdpChannel(channel, _) => self
                        .poll
                        .register(channel.get_handle(), tok, interest, opts)
                        .unwrap(),
                    TokenObjects::MpscChannel(channel, _) => self
                        .poll
                        .register(channel.get_handle(), tok, interest, opts)
                        .unwrap(),
                }
                debug!(?handle, ?tok, "Registered new handle");
                self.mio_tokens.insert(tok, handle);
            },
            CtrlMsg::OpenStream {
                pid,
                prio,
                promises,
            } => {
                for (tok, obj) in self.mio_tokens.tokens.iter_mut() {
                    if let TokenObjects::TcpChannel(channel, _) = obj {
                        channel.open_stream(prio, promises); //TODO: check participant
                        channel.tick_send();
                    }
                }
                //TODO:
            },
            CtrlMsg::CloseStream { pid, sid } => {
                //TODO:
                for to in self.mio_tokens.tokens.values_mut() {
                    if let TokenObjects::TcpChannel(channel, _) = to {
                        channel.close_stream(sid); //TODO: check participant
                        channel.tick_send();
                    }
                }
            },
            CtrlMsg::Send(outgoing) => {
                //TODO:
                for to in self.mio_tokens.tokens.values_mut() {
                    if let TokenObjects::TcpChannel(channel, _) = to {
                        channel.send(outgoing); //TODO: check participant
                        channel.tick_send();
                        break;
                    }
                }
            },
        };
        false
    }

    fn handle_tok(&mut self, event: &mio::Event) {
        let obj = match self.mio_tokens.tokens.get_mut(&event.token()) {
            Some(obj) => obj,
            None => panic!("Unexpected event token '{:?}'", &event.token()),
        };

        match obj {
            TokenObjects::TcpListener(listener) => match listener.accept() {
                Ok((remote_stream, _)) => {
                    info!(?remote_stream, "remote connected");

                    let tok = self.mio_tokens.construct();
                    self.poll
                        .register(
                            &remote_stream,
                            tok,
                            Ready::readable() | Ready::writable(),
                            PollOpt::edge(),
                        )
                        .unwrap();
                    trace!(?remote_stream, ?tok, "registered");
                    let tcp_channel = TcpChannel::new(remote_stream);
                    let mut channel = Channel::new(self.pid, tcp_channel, self.remotes.clone());
                    channel.handshake();
                    channel.tick_send();

                    self.mio_tokens
                        .tokens
                        .insert(tok, TokenObjects::TcpChannel(channel, None));
                },
                Err(err) => {
                    error!(?err, "error during remote connected");
                },
            },
            TokenObjects::TcpChannel(channel, _) => {
                if event.readiness().is_readable() {
                    let handle = channel.get_handle();
                    trace!(?handle, "stream readable");
                    channel.tick_recv(&self.rtrn_tx);
                }
                if event.readiness().is_writable() {
                    let handle = channel.get_handle();
                    trace!(?handle, "stream writeable");
                    channel.tick_send();
                }
            },
            TokenObjects::UdpChannel(channel, _) => {
                if event.readiness().is_readable() {
                    let handle = channel.get_handle();
                    trace!(?handle, "stream readable");
                    channel.tick_recv(&self.rtrn_tx);
                }
                if event.readiness().is_writable() {
                    let handle = channel.get_handle();
                    trace!(?handle, "stream writeable");
                    channel.tick_send();
                }
            },
            TokenObjects::MpscChannel(channel, _) => {
                if event.readiness().is_readable() {
                    let handle = channel.get_handle();
                    channel.tick_recv(&self.rtrn_tx);
                }
                if event.readiness().is_writable() {
                    let handle = channel.get_handle();
                    channel.tick_send();
                }
            },
        };
    }

    fn handle_statistics(&mut self) {
        let time_after_work = Instant::now();
        let mut statistics = match self.statistics.try_write() {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    ?e,
                    "statistics dropped because they are currently accecssed"
                );
                return;
            },
        };

        const KEEP_FACTOR: f64 = 0.995;
        //in order to weight new data stronger than older we fade them out with a
        // factor < 1. for 0.995 under full load (500 ticks a 1ms) we keep 8% of the old
        // value this means, that we start to see load comming up after
        // 500ms, but not for small spikes - as reordering for smaller spikes would be
        // to slow
        let first = self.time_after_poll.duration_since(self.time_before_poll);
        let second = time_after_work.duration_since(self.time_after_poll);
        statistics.nano_wait =
            (statistics.nano_wait as f64 * KEEP_FACTOR) as u128 + first.as_nanos();
        statistics.nano_busy =
            (statistics.nano_busy as f64 * KEEP_FACTOR) as u128 + second.as_nanos();

        trace!(
            "current Load {}",
            statistics.nano_busy as f32 / (statistics.nano_busy + statistics.nano_wait + 1) as f32
        );
    }
}
