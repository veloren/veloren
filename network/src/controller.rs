/*
    Most of the internals take place in it's own worker-thread.
    This folder contains all this outsourced calculation.
    This controller contains the interface to communicate with the thread,
    communication is done via channels.
*/
use crate::{
    metrics::NetworkMetrics,
    types::{CtrlMsg, Pid, RemoteParticipant, RtrnMsg},
    worker::Worker,
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
        metrics: Arc<Option<NetworkMetrics>>,
        remotes: Arc<RwLock<HashMap<Pid, RemoteParticipant>>>,
    ) -> Self {
        let poll = Arc::new(Poll::new().unwrap());

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
            let mut worker = Worker::new(pid, poll, metrics, remotes, token_pool, ctrl_rx, rtrn_tx);
            worker.run();
        });
        Controller { ctrl_tx, rtrn_rx }
    }

    //TODO: split 4->5 MioWorkers and merge 5->4 MioWorkers

    pub(crate) fn get_tx(&self) -> Sender<CtrlMsg> { self.ctrl_tx.clone() }

    pub(crate) fn get_rx(&self) -> &Receiver<RtrnMsg> { &self.rtrn_rx }
}
impl Drop for Controller {
    fn drop(&mut self) { let _ = self.ctrl_tx.send(CtrlMsg::Shutdown); }
}
