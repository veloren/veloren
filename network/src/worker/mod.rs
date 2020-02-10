/*
    Most of the internals take place in it's own worker-thread.
    This folder contains all this outsourced calculation.
    This mod.rs contains the interface to communicate with the thread,
    communication is done via channels.
*/
pub mod channel;
pub mod mpsc;
pub mod tcp;
pub mod types;
pub mod udp;
pub mod worker;

pub(crate) use channel::Channel;
pub(crate) use mpsc::MpscChannel;
pub(crate) use tcp::TcpChannel;
pub(crate) use udp::UdpChannel;

use crate::{
    internal::RemoteParticipant,
    worker::{
        types::{CtrlMsg, Pid, RtrnMsg, Statistics},
        worker::Worker,
    },
};
use mio::{self, Poll, PollOpt, Ready, Token};
use mio_extras::channel::{channel, Receiver, Sender};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tlid;
use tracing::*;
use uvth::ThreadPool;

/*
    The MioWorker runs in it's own thread,
    it has a given set of Channels to work with.
    It is monitored, and when it's thread is fully loaded it can be splitted up into 2 MioWorkers
*/
pub struct Controller {
    poll: Arc<Poll>,
    statistics: Arc<RwLock<Statistics>>,
    ctrl_tx: Sender<CtrlMsg>,
    rtrn_rx: Receiver<RtrnMsg>,
}

impl Controller {
    pub const CTRL_TOK: Token = Token(0);

    pub fn new(
        wid: u64,
        pid: uuid::Uuid,
        thread_pool: Arc<ThreadPool>,
        mut token_pool: tlid::Pool<tlid::Wrapping<usize>>,
        remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
    ) -> Self {
        let poll = Arc::new(Poll::new().unwrap());
        let poll_clone = poll.clone();
        let statistics = Arc::new(RwLock::new(Statistics::default()));
        let statistics_clone = statistics.clone();

        let (ctrl_tx, ctrl_rx) = channel();
        let (rtrn_tx, rtrn_rx) = channel();
        poll.register(&ctrl_rx, Self::CTRL_TOK, Ready::readable(), PollOpt::edge())
            .unwrap();
        // reserve 10 tokens in case they start with 0, //TODO: cleaner method
        for _ in 0..10 {
            token_pool.next();
        }

        thread_pool.execute(move || {
            let w = wid;
            let span = span!(Level::INFO, "worker", ?w);
            let _enter = span.enter();
            let mut worker = Worker::new(
                pid,
                poll_clone,
                statistics_clone,
                remotes,
                token_pool,
                ctrl_rx,
                rtrn_tx,
            );
            worker.run();
        });
        Controller {
            poll,
            statistics,
            ctrl_tx,
            rtrn_rx,
        }
    }

    pub fn get_load_ratio(&self) -> f32 {
        let statistics = self.statistics.read().unwrap();
        statistics.nano_busy as f32 / (statistics.nano_busy + statistics.nano_wait + 1) as f32
    }

    //TODO: split 4->5 MioWorkers and merge 5->4 MioWorkers

    pub(crate) fn get_tx(&self) -> Sender<CtrlMsg> { self.ctrl_tx.clone() }

    pub(crate) fn get_rx(&self) -> &Receiver<RtrnMsg> { &self.rtrn_rx }
}
impl Drop for Controller {
    fn drop(&mut self) { let _ = self.ctrl_tx.send(CtrlMsg::Shutdown); }
}
