use crate::{
    message::{self, InCommingMessage, OutGoingMessage},
    scheduler::Scheduler,
    types::{Mid, Pid, Prio, Promises, Requestor::User, Sid},
};
use async_std::{io, sync::RwLock, task};
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

pub struct Participant {
    local_pid: Pid,
    remote_pid: Pid,
    stream_open_sender: RwLock<mpsc::UnboundedSender<(Prio, Promises, oneshot::Sender<Stream>)>>,
    stream_opened_receiver: RwLock<mpsc::UnboundedReceiver<Stream>>,
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
    closed: Arc<AtomicBool>,
    shutdown_sender: Option<mpsc::UnboundedSender<Sid>>,
}

#[derive(Debug)]
pub enum NetworkError {
    NetworkClosed,
    ListenFailed(std::io::Error),
}

#[derive(Debug, PartialEq)]
pub enum ParticipantError {
    ParticipantClosed,
}

#[derive(Debug, PartialEq)]
pub enum StreamError {
    StreamClosed,
}

pub struct Network {
    local_pid: Pid,
    participants: RwLock<HashMap<Pid, Arc<Participant>>>,
    listen_sender:
        RwLock<mpsc::UnboundedSender<(Address, oneshot::Sender<async_std::io::Result<()>>)>>,
    connect_sender:
        RwLock<mpsc::UnboundedSender<(Address, oneshot::Sender<io::Result<Participant>>)>>,
    connected_receiver: RwLock<mpsc::UnboundedReceiver<Participant>>,
    shutdown_sender: Option<oneshot::Sender<()>>,
}

impl Network {
    pub fn new(participant_id: Pid, thread_pool: &ThreadPool) -> Self {
        //let participants = RwLock::new(vec![]);
        let p = participant_id;
        debug!(?p, ?User, "starting Network");
        let (scheduler, listen_sender, connect_sender, connected_receiver, shutdown_sender) =
            Scheduler::new(participant_id);
        thread_pool.execute(move || {
            trace!(?p, ?User, "starting sheduler in own thread");
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

    pub async fn listen(&self, address: Address) -> Result<(), NetworkError> {
        let (result_sender, result_receiver) = oneshot::channel::<async_std::io::Result<()>>();
        debug!(?address, ?User, "listening on address");
        self.listen_sender
            .write()
            .await
            .send((address, result_sender))
            .await?;
        match result_receiver.await? {
            //waiting guarantees that we either listened sucessfully or get an error like port in
            // use
            Ok(()) => Ok(()),
            Err(e) => Err(NetworkError::ListenFailed(e)),
        }
    }

    pub async fn connect(&self, address: Address) -> Result<Arc<Participant>, NetworkError> {
        let (pid_sender, pid_receiver) = oneshot::channel::<io::Result<Participant>>();
        debug!(?address, ?User, "connect to address");
        self.connect_sender
            .write()
            .await
            .send((address, pid_sender))
            .await?;
        let participant = pid_receiver.await??;
        let pid = participant.remote_pid;
        debug!(
            ?pid,
            ?User,
            "received Participant id from remote and return to user"
        );
        let participant = Arc::new(participant);
        self.participants
            .write()
            .await
            .insert(participant.remote_pid, participant.clone());
        Ok(participant)
    }

    pub async fn connected(&self) -> Result<Arc<Participant>, NetworkError> {
        let participant = self.connected_receiver.write().await.next().await?;
        let participant = Arc::new(participant);
        self.participants
            .write()
            .await
            .insert(participant.remote_pid, participant.clone());
        Ok(participant)
    }

    pub async fn disconnect(&self, participant: Arc<Participant>) -> Result<(), NetworkError> {
        // Remove, Close and try_unwrap error when unwrap fails!
        let pid = participant.remote_pid;
        debug!(?pid, "removing participant from network");
        self.participants.write().await.remove(&pid)?;
        participant.closed.store(true, Ordering::Relaxed);

        if Arc::try_unwrap(participant).is_err() {
            warn!(
                "you are disconnecting and still keeping a reference to this participant, this is \
                 a bad idea. Participant will only be dropped when you drop your last reference"
            );
        };
        Ok(())
    }

    pub async fn participants(&self) -> HashMap<Pid, Arc<Participant>> {
        self.participants.read().await.clone()
    }
}

impl Participant {
    pub(crate) fn new(
        local_pid: Pid,
        remote_pid: Pid,
        stream_open_sender: mpsc::UnboundedSender<(Prio, Promises, oneshot::Sender<Stream>)>,
        stream_opened_receiver: mpsc::UnboundedReceiver<Stream>,
        disconnect_sender: mpsc::UnboundedSender<Pid>,
    ) -> Self {
        Self {
            local_pid,
            remote_pid,
            stream_open_sender: RwLock::new(stream_open_sender),
            stream_opened_receiver: RwLock::new(stream_opened_receiver),
            closed: AtomicBool::new(false),
            disconnect_sender: Some(disconnect_sender),
        }
    }

    pub async fn open(&self, prio: u8, promises: Promises) -> Result<Stream, ParticipantError> {
        //use this lock for now to make sure that only one open at a time is made,
        // TODO: not sure if we can paralise that, check in future
        let mut stream_open_sender = self.stream_open_sender.write().await;
        if self.closed.load(Ordering::Relaxed) {
            warn!(?self.remote_pid, "participant is closed but another open is tried on it");
            return Err(ParticipantError::ParticipantClosed);
        }
        let (sid_sender, sid_receiver) = oneshot::channel();
        if stream_open_sender
            .send((prio, promises, sid_sender))
            .await
            .is_err()
        {
            debug!(?self.remote_pid, ?User, "stream_open_sender failed, closing participant");
            self.closed.store(true, Ordering::Relaxed);
            return Err(ParticipantError::ParticipantClosed);
        }
        match sid_receiver.await {
            Ok(stream) => {
                let sid = stream.sid;
                debug!(?sid, ?self.remote_pid, ?User, "opened stream");
                Ok(stream)
            },
            Err(_) => {
                debug!(?self.remote_pid, ?User, "sid_receiver failed, closing participant");
                self.closed.store(true, Ordering::Relaxed);
                Err(ParticipantError::ParticipantClosed)
            },
        }
    }

    pub async fn opened(&self) -> Result<Stream, ParticipantError> {
        //use this lock for now to make sure that only one open at a time is made,
        // TODO: not sure if we can paralise that, check in future
        let mut stream_opened_receiver = self.stream_opened_receiver.write().await;
        if self.closed.load(Ordering::Relaxed) {
            warn!(?self.remote_pid, "participant is closed but another open is tried on it");
            return Err(ParticipantError::ParticipantClosed);
        }
        match stream_opened_receiver.next().await {
            Some(stream) => {
                let sid = stream.sid;
                debug!(?sid, ?self.remote_pid, "receive opened stream");
                Ok(stream)
            },
            None => {
                debug!(?self.remote_pid, "stream_opened_receiver failed, closing participant");
                self.closed.store(true, Ordering::Relaxed);
                Err(ParticipantError::ParticipantClosed)
            },
        }
    }

    pub fn remote_pid(&self) -> Pid { self.remote_pid }
}

impl Stream {
    pub(crate) fn new(
        pid: Pid,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        msg_send_sender: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
        msg_recv_receiver: mpsc::UnboundedReceiver<InCommingMessage>,
        closed: Arc<AtomicBool>,
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
            closed,
            shutdown_sender: Some(shutdown_sender),
        }
    }

    pub fn send<M: Serialize>(&mut self, msg: M) -> Result<(), StreamError> {
        let messagebuffer = Arc::new(message::serialize(&msg));
        if self.closed.load(Ordering::Relaxed) {
            return Err(StreamError::StreamClosed);
        }
        self.msg_send_sender
            .send((self.prio, self.pid, self.sid, OutGoingMessage {
                buffer: messagebuffer,
                cursor: 0,
                mid: self.mid,
                sid: self.sid,
            }))?;
        self.mid += 1;
        Ok(())
    }

    pub async fn recv<M: DeserializeOwned>(&mut self) -> Result<M, StreamError> {
        //no need to access self.closed here, as when this stream is closed the Channel
        // is closed which will trigger a None
        let msg = self.msg_recv_receiver.next().await?;
        info!(?msg, "delivering a message");
        Ok(message::deserialize(msg.buffer))
    }
    //Todo: ERROR: TODO: implement me and the disconnecting!
}

impl Drop for Network {
    fn drop(&mut self) {
        let pid = self.local_pid;
        debug!(?pid, "shutting down Network");
        self.shutdown_sender
            .take()
            .unwrap()
            .send(())
            .expect("scheduler is closed, but nobody other should be able to close it");
    }
}

impl Drop for Participant {
    fn drop(&mut self) {
        // ignore closed, as we need to send it even though we disconnected the
        // participant from network
        let pid = self.remote_pid;
        debug!(?pid, "shutting down Participant");
        task::block_on(async {
            self.disconnect_sender
                .take()
                .unwrap()
                .send(self.remote_pid)
                .await
                .expect("something is wrong in internal scheduler coding")
        });
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        // a send if closed is unecessary but doesnt hurt, we must not crash here
        if !self.closed.load(Ordering::Relaxed) {
            let sid = self.sid;
            let pid = self.pid;
            debug!(?pid, ?sid, "shutting down Stream");
            if task::block_on(self.shutdown_sender.take().unwrap().send(self.sid)).is_err() {
                warn!(
                    "Other side got already dropped, probably due to timing, other side will \
                     handle this gracefully"
                );
            };
        }
    }
}

impl std::fmt::Debug for Participant {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.closed.load(Ordering::Relaxed) {
            "[CLOSED]"
        } else {
            "[OPEN]"
        };
        write!(
            f,
            "Participant {{{} local_pid: {:?}, remote_pid: {:?} }}",
            status, &self.local_pid, &self.remote_pid,
        )
    }
}

impl<T> From<std::sync::mpsc::SendError<T>> for StreamError {
    fn from(_err: std::sync::mpsc::SendError<T>) -> Self { StreamError::StreamClosed }
}

impl<T> From<std::sync::mpsc::SendError<T>> for ParticipantError {
    fn from(_err: std::sync::mpsc::SendError<T>) -> Self { ParticipantError::ParticipantClosed }
}

impl<T> From<std::sync::mpsc::SendError<T>> for NetworkError {
    fn from(_err: std::sync::mpsc::SendError<T>) -> Self { NetworkError::NetworkClosed }
}

impl From<async_std::io::Error> for NetworkError {
    fn from(err: async_std::io::Error) -> Self { NetworkError::ListenFailed(err) }
}

impl From<std::option::NoneError> for StreamError {
    fn from(_err: std::option::NoneError) -> Self { StreamError::StreamClosed }
}

impl From<std::option::NoneError> for ParticipantError {
    fn from(_err: std::option::NoneError) -> Self { ParticipantError::ParticipantClosed }
}

impl From<std::option::NoneError> for NetworkError {
    fn from(_err: std::option::NoneError) -> Self { NetworkError::NetworkClosed }
}

impl From<mpsc::SendError> for ParticipantError {
    fn from(_err: mpsc::SendError) -> Self { ParticipantError::ParticipantClosed }
}

impl From<mpsc::SendError> for NetworkError {
    fn from(_err: mpsc::SendError) -> Self { NetworkError::NetworkClosed }
}

impl From<oneshot::Canceled> for ParticipantError {
    fn from(_err: oneshot::Canceled) -> Self { ParticipantError::ParticipantClosed }
}

impl From<oneshot::Canceled> for NetworkError {
    fn from(_err: oneshot::Canceled) -> Self { NetworkError::NetworkClosed }
}
