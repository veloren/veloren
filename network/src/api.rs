use crate::{
    message::{self, InCommingMessage, OutGoingMessage},
    scheduler::Scheduler,
    types::{Mid, Pid, Prio, Promises, Sid},
};
use async_std::{sync::RwLock, task};
use futures::{
    channel::{mpsc, oneshot},
    sink::SinkExt,
    stream::StreamExt,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tracing::*;
use tracing_futures::Instrument;
use uvth::ThreadPool;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Address {
    Tcp(std::net::SocketAddr),
    Udp(std::net::SocketAddr),
    Mpsc(u64),
}

#[derive(Debug)]
pub struct Participant {
    local_pid: Pid,
    remote_pid: Pid,
    stream_open_sender: RwLock<mpsc::UnboundedSender<(Prio, Promises, oneshot::Sender<Stream>)>>,
    stream_opened_receiver: RwLock<mpsc::UnboundedReceiver<Stream>>,
    shutdown_receiver: RwLock<oneshot::Receiver<()>>,
    closed: AtomicBool,
    disconnect_sender: Option<mpsc::UnboundedSender<Pid>>,
}

#[derive(Debug)]
pub struct Stream {
    pid: Pid,
    sid: Sid,
    mid: Mid,
    prio: Prio,
    promises: Promises,
    msg_send_sender: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
    msg_recv_receiver: mpsc::UnboundedReceiver<InCommingMessage>,
    shutdown_receiver: oneshot::Receiver<()>,
    closed: AtomicBool,
    shutdown_sender: Option<mpsc::UnboundedSender<Sid>>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct NetworkError {}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct ParticipantError {}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StreamError {}

pub struct Network {
    local_pid: Pid,
    participants: RwLock<HashMap<Pid, Arc<Participant>>>,
    listen_sender: RwLock<mpsc::UnboundedSender<Address>>,
    connect_sender: RwLock<mpsc::UnboundedSender<(Address, oneshot::Sender<Participant>)>>,
    connected_receiver: RwLock<mpsc::UnboundedReceiver<Participant>>,
    shutdown_sender: Option<oneshot::Sender<()>>,
}

impl Network {
    pub fn new(participant_id: Pid, thread_pool: &ThreadPool) -> Self {
        //let participants = RwLock::new(vec![]);
        let p = participant_id;
        debug!(?p, "starting Network");
        let (scheduler, listen_sender, connect_sender, connected_receiver, shutdown_sender) =
            Scheduler::new(participant_id);
        thread_pool.execute(move || {
            let _handle = task::block_on(
                scheduler
                    .run()
                    .instrument(tracing::info_span!("scheduler", ?p)),
            );
        });
        Self {
            local_pid: participant_id,
            participants: RwLock::new(HashMap::new()),
            listen_sender: RwLock::new(listen_sender),
            connect_sender: RwLock::new(connect_sender),
            connected_receiver: RwLock::new(connected_receiver),
            shutdown_sender: Some(shutdown_sender),
        }
    }

    pub fn listen(&self, address: Address) -> Result<(), NetworkError> {
        task::block_on(async { self.listen_sender.write().await.send(address).await }).unwrap();
        Ok(())
    }

    pub async fn connect(&self, address: Address) -> Result<Arc<Participant>, NetworkError> {
        let (pid_sender, pid_receiver) = oneshot::channel::<Participant>();
        self.connect_sender
            .write()
            .await
            .send((address, pid_sender))
            .await
            .unwrap();
        match pid_receiver.await {
            Ok(participant) => {
                let pid = participant.remote_pid;
                debug!(?pid, "received Participant from remote");
                let participant = Arc::new(participant);
                self.participants
                    .write()
                    .await
                    .insert(participant.remote_pid, participant.clone());
                Ok(participant)
            },
            Err(_) => Err(NetworkError {}),
        }
    }

    pub async fn connected(&self) -> Result<Arc<Participant>, NetworkError> {
        match self.connected_receiver.write().await.next().await {
            Some(participant) => {
                let participant = Arc::new(participant);
                self.participants
                    .write()
                    .await
                    .insert(participant.remote_pid, participant.clone());
                Ok(participant)
            },
            None => Err(NetworkError {}),
        }
    }

    pub async fn disconnect(&self, participant: Arc<Participant>) -> Result<(), NetworkError> {
        // Remove, Close and try_unwrap error when unwrap fails!
        let participant = self
            .participants
            .write()
            .await
            .remove(&participant.remote_pid)
            .unwrap();
        participant.closed.store(true, Ordering::Relaxed);

        if Arc::try_unwrap(participant).is_err() {
            warn!(
                "you are disconnecting and still keeping a reference to this participant, this is \
                 a bad idea. Participant will only be dropped when you drop your last reference"
            );
        };
        Ok(())
    }
}

//TODO: HANDLE SHUTDOWN_RECEIVER

impl Participant {
    pub(crate) fn new(
        local_pid: Pid,
        remote_pid: Pid,
        stream_open_sender: mpsc::UnboundedSender<(Prio, Promises, oneshot::Sender<Stream>)>,
        stream_opened_receiver: mpsc::UnboundedReceiver<Stream>,
        shutdown_receiver: oneshot::Receiver<()>,
        disconnect_sender: mpsc::UnboundedSender<Pid>,
    ) -> Self {
        Self {
            local_pid,
            remote_pid,
            stream_open_sender: RwLock::new(stream_open_sender),
            stream_opened_receiver: RwLock::new(stream_opened_receiver),
            shutdown_receiver: RwLock::new(shutdown_receiver),
            closed: AtomicBool::new(false),
            disconnect_sender: Some(disconnect_sender),
        }
    }

    pub async fn open(&self, prio: u8, promises: Promises) -> Result<Stream, ParticipantError> {
        let (sid_sender, sid_receiver) = oneshot::channel();
        self.stream_open_sender
            .write()
            .await
            .send((prio, promises, sid_sender))
            .await
            .unwrap();
        match sid_receiver.await {
            Ok(stream) => {
                let sid = stream.sid;
                debug!(?sid, "opened stream");
                Ok(stream)
            },
            Err(_) => Err(ParticipantError {}),
        }
    }

    pub async fn opened(&self) -> Result<Stream, ParticipantError> {
        match self.stream_opened_receiver.write().await.next().await {
            Some(stream) => Ok(stream),
            None => Err(ParticipantError {}),
        }
    }
}

impl Stream {
    pub(crate) fn new(
        pid: Pid,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        msg_send_sender: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
        msg_recv_receiver: mpsc::UnboundedReceiver<InCommingMessage>,
        shutdown_receiver: oneshot::Receiver<()>,
        shutdown_sender: mpsc::UnboundedSender<Sid>,
    ) -> Self {
        Self {
            pid,
            sid,
            mid: 0,
            prio,
            promises,
            msg_send_sender,
            msg_recv_receiver,
            shutdown_receiver,
            closed: AtomicBool::new(false),
            shutdown_sender: Some(shutdown_sender),
        }
    }

    pub async fn send<M: Serialize>(&mut self, msg: M) -> Result<(), StreamError> {
        let messagebuffer = Arc::new(message::serialize(&msg));
        self.msg_send_sender
            .send((self.prio, self.pid, self.sid, OutGoingMessage {
                buffer: messagebuffer,
                cursor: 0,
                mid: self.mid,
                sid: self.sid,
            }))
            .unwrap();
        self.mid += 1;
        Ok(())
    }

    pub async fn recv<M: DeserializeOwned>(&mut self) -> Result<M, StreamError> {
        match self.msg_recv_receiver.next().await {
            Some(msg) => {
                info!(?msg, "delivering a message");
                Ok(message::deserialize(msg.buffer))
            },
            None => panic!(
                "Unexpected error, probably stream was destroyed... maybe i dont know yet, no \
                 idea of async stuff"
            ),
        }
    }
    //Todo: ERROR: TODO: implement me and the disconnecting!
}

impl Drop for Network {
    fn drop(&mut self) {
        let p = self.local_pid;
        debug!(?p, "shutting down Network");
        self.shutdown_sender.take().unwrap().send(()).unwrap();
    }
}

impl Drop for Participant {
    fn drop(&mut self) {
        if !self.closed.load(Ordering::Relaxed) {
            let p = self.remote_pid;
            debug!(?p, "shutting down Participant");
            task::block_on(async {
                self.disconnect_sender
                    .take()
                    .unwrap()
                    .send(self.remote_pid)
                    .await
                    .unwrap()
            });
        }
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if !self.closed.load(Ordering::Relaxed) {
            let s = self.sid;
            debug!(?s, "shutting down Stream");
            task::block_on(async {
                self.shutdown_sender
                    .take()
                    .unwrap()
                    .send(self.sid)
                    .await
                    .unwrap()
            });
        }
    }
}
