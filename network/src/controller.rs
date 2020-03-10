/*
    Most of the internals take place in it's own worker-thread.
    This folder contains all this outsourced calculation.
    This controller contains the interface to communicate with the thread,
    communication is done via channels.
*/
use crate::{
    api::Stream,
    metrics::NetworkMetrics,
    types::{CtrlMsg, Pid, RtrnMsg, Sid},
    worker::Worker,
};
use mio::{self, Poll, PollOpt, Ready, Token};
use mio_extras::channel;
use std::{
    collections::HashMap,
    sync::{mpsc, Arc, RwLock, RwLockReadGuard},
};
use tlid;
use tracing::*;
use uvth::ThreadPool;

pub struct ControllerParticipant {
    pub sid_pool: RwLock<tlid::Pool<tlid::Wrapping<Sid>>>,
    //TODO: move this in a future aware variant! via futures Channels
    stream_open_tx: mpsc::Sender<Stream>,
    pub stream_open_rx: mpsc::Receiver<Stream>,
    pub stream_close_txs: RwLock<HashMap<Sid, mpsc::Sender<()>>>,
}

/*
    The MioWorker runs in it's own thread,
    it has a given set of Channels to work with.
    It is monitored, and when it's thread is fully loaded it can be splitted up into 2 MioWorkers
*/
pub struct Controller {
    ctrl_tx: channel::Sender<CtrlMsg>,
    rtrn_rx: mpsc::Receiver<RtrnMsg>,

    participant_connect_tx: mpsc::Sender<Pid>,
    participant_connect_rx: mpsc::Receiver<Pid>,

    participants: RwLock<HashMap<Pid, ControllerParticipant>>,
}

impl Controller {
    pub const CTRL_TOK: Token = Token(0);

    pub fn new(
        wid: u64,
        pid: uuid::Uuid,
        thread_pool: Arc<ThreadPool>,
        mut token_pool: tlid::Pool<tlid::Wrapping<usize>>,
        metrics: Arc<Option<NetworkMetrics>>,
        sid_backup_per_participant: Arc<RwLock<HashMap<Pid, tlid::Pool<tlid::Checked<Sid>>>>>,
    ) -> Self {
        let poll = Arc::new(Poll::new().unwrap());

        let (ctrl_tx, ctrl_rx) = channel::channel();
        let (rtrn_tx, rtrn_rx) = mpsc::channel();
        let (participant_connect_tx, participant_connect_rx) = mpsc::channel();
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
                poll,
                metrics,
                sid_backup_per_participant,
                token_pool,
                ctrl_rx,
                rtrn_tx,
            );
            worker.run();
        });
        let participants = RwLock::new(HashMap::new());
        Controller {
            ctrl_tx,
            rtrn_rx,
            participant_connect_rx,
            participant_connect_tx,
            participants,
        }
    }

    //TODO: split 4->5 MioWorkers and merge 5->4 MioWorkers

    pub(crate) fn get_tx(&self) -> channel::Sender<CtrlMsg> { self.ctrl_tx.clone() }

    pub(crate) fn get_participant_connect_rx(&self) -> &mpsc::Receiver<Pid> {
        &self.participant_connect_rx
    }

    pub(crate) fn tick(&self) {
        for msg in self.rtrn_rx.try_iter() {
            match msg {
                /*TODO: WAIT, THIS ASSUMES CONNECTED PARTICIPANT IS ONLY EVER TRIGGERED ONCE PER CONTROLLER
                that means, that it can happen multiple time for the same participant on multiple controller,
                and even multiple channel on one worker shouldn't trigger it*/
                RtrnMsg::ConnectedParticipant {
                    pid,
                    controller_sids,
                } => {
                    let mut parts = self.participants.write().unwrap();
                    debug!(
                        ?pid,
                        "A new participant connected to this channel, we assign it the sid pool"
                    );
                    let (stream_open_tx, stream_open_rx) = mpsc::channel();
                    let part = ControllerParticipant {
                        sid_pool: RwLock::new(controller_sids),
                        stream_open_tx,
                        stream_open_rx,
                        stream_close_txs: RwLock::new(HashMap::new()),
                    };
                    parts.insert(pid.clone(), part);
                    self.participant_connect_tx.send(pid).unwrap();
                },
                RtrnMsg::OpendStream {
                    pid,
                    sid,
                    prio: _,
                    msg_rx,
                    promises: _,
                } => {
                    trace!(
                        ?pid,
                        ?sid,
                        "A new stream was opened on this channel, we assign it the participant"
                    );
                    let parts = self.participants.read().unwrap();
                    if let Some(p) = parts.get(&pid) {
                        let (stream_close_tx, stream_close_rx) = mpsc::channel();
                        p.stream_close_txs
                            .write()
                            .unwrap()
                            .insert(sid, stream_close_tx);
                        p.stream_open_tx
                            .send(Stream::new(
                                sid,
                                pid,
                                stream_close_rx,
                                msg_rx,
                                self.ctrl_tx.clone(),
                            ))
                            .unwrap();
                    }
                },
                RtrnMsg::ClosedStream { pid, sid } => {
                    trace!(?pid, ?sid, "Stream got closeed, will route message");
                    let parts = self.participants.read().unwrap();
                    if let Some(p) = parts.get(&pid) {
                        if let Some(tx) = p.stream_close_txs.read().unwrap().get(&sid) {
                            tx.send(()).unwrap();
                            trace!(?pid, ?sid, "routed message");
                        }
                    }
                },
                _ => {},
            }
        }
    }

    pub(crate) fn participants(&self) -> RwLockReadGuard<HashMap<Pid, ControllerParticipant>> {
        self.participants.read().unwrap()
    }
}
impl Drop for Controller {
    fn drop(&mut self) { let _ = self.ctrl_tx.send(CtrlMsg::Shutdown); }
}
