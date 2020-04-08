use crate::{
    api::Stream,
    message::{InCommingMessage, MessageBuffer, OutGoingMessage},
    types::{Cid, Frame, Pid, Prio, Promises, Sid},
};
use async_std::sync::RwLock;
use futures::{
    channel::{mpsc, oneshot},
    future::FutureExt,
    select,
    sink::SinkExt,
    stream::StreamExt,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tracing::*;

#[derive(Debug)]
struct ControlChannels {
    stream_open_receiver: mpsc::UnboundedReceiver<(Prio, Promises, oneshot::Sender<Stream>)>,
    stream_opened_sender: mpsc::UnboundedSender<Stream>,
    transfer_channel_receiver: mpsc::UnboundedReceiver<(Cid, mpsc::UnboundedSender<Frame>)>,
    frame_recv_receiver: mpsc::UnboundedReceiver<Frame>,
    shutdown_api_receiver: mpsc::UnboundedReceiver<Sid>,
    shutdown_api_sender: mpsc::UnboundedSender<Sid>,
    send_outgoing: Arc<Mutex<std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>>>, //api
    frame_send_receiver: mpsc::UnboundedReceiver<(Pid, Sid, Frame)>, //scheduler
    shutdown_receiver: oneshot::Receiver<()>,                        //own
    stream_finished_request_sender: mpsc::UnboundedSender<(Pid, Sid, oneshot::Sender<()>)>,
}

#[derive(Debug)]
pub struct BParticipant {
    remote_pid: Pid,
    offset_sid: Sid,
    channels: RwLock<Vec<(Cid, mpsc::UnboundedSender<Frame>)>>,
    streams: RwLock<
        HashMap<
            Sid,
            (
                Prio,
                Promises,
                mpsc::UnboundedSender<InCommingMessage>,
                Arc<AtomicBool>,
            ),
        >,
    >,
    run_channels: Option<ControlChannels>,
}

impl BParticipant {
    pub(crate) fn new(
        remote_pid: Pid,
        offset_sid: Sid,
        send_outgoing: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
        stream_finished_request_sender: mpsc::UnboundedSender<(Pid, Sid, oneshot::Sender<()>)>,
    ) -> (
        Self,
        mpsc::UnboundedSender<(Prio, Promises, oneshot::Sender<Stream>)>,
        mpsc::UnboundedReceiver<Stream>,
        mpsc::UnboundedSender<(Cid, mpsc::UnboundedSender<Frame>)>,
        mpsc::UnboundedSender<Frame>,
        mpsc::UnboundedSender<(Pid, Sid, Frame)>,
        oneshot::Sender<()>,
    ) {
        let (stream_open_sender, stream_open_receiver) =
            mpsc::unbounded::<(Prio, Promises, oneshot::Sender<Stream>)>();
        let (stream_opened_sender, stream_opened_receiver) = mpsc::unbounded::<Stream>();
        let (transfer_channel_sender, transfer_channel_receiver) =
            mpsc::unbounded::<(Cid, mpsc::UnboundedSender<Frame>)>();
        let (frame_recv_sender, frame_recv_receiver) = mpsc::unbounded::<Frame>();
        let (shutdown_api_sender, shutdown_api_receiver) = mpsc::unbounded();
        let (frame_send_sender, frame_send_receiver) = mpsc::unbounded::<(Pid, Sid, Frame)>();
        let (shutdown_sender, shutdown_receiver) = oneshot::channel();

        let run_channels = Some(ControlChannels {
            stream_open_receiver,
            stream_opened_sender,
            transfer_channel_receiver,
            frame_recv_receiver,
            shutdown_api_receiver,
            shutdown_api_sender,
            send_outgoing: Arc::new(Mutex::new(send_outgoing)),
            frame_send_receiver,
            shutdown_receiver,
            stream_finished_request_sender,
        });

        (
            Self {
                remote_pid,
                offset_sid,
                channels: RwLock::new(vec![]),
                streams: RwLock::new(HashMap::new()),
                run_channels,
            },
            stream_open_sender,
            stream_opened_receiver,
            transfer_channel_sender,
            frame_recv_sender,
            frame_send_sender,
            shutdown_sender,
        )
    }

    pub async fn run(mut self) {
        //those managers that listen on api::Participant need an additional oneshot for
        // shutdown scenario, those handled by scheduler will be closed by it.
        let (shutdown_open_manager_sender, shutdown_open_manager_receiver) = oneshot::channel();
        let (shutdown_stream_close_manager_sender, shutdown_stream_close_manager_receiver) =
            oneshot::channel();

        let run_channels = self.run_channels.take().unwrap();
        futures::join!(
            self.transfer_channel_manager(run_channels.transfer_channel_receiver),
            self.open_manager(
                run_channels.stream_open_receiver,
                run_channels.shutdown_api_sender.clone(),
                run_channels.send_outgoing.clone(),
                shutdown_open_manager_receiver,
            ),
            self.handle_frames(
                run_channels.frame_recv_receiver,
                run_channels.stream_opened_sender,
                run_channels.shutdown_api_sender,
                run_channels.send_outgoing.clone(),
            ),
            self.send_manager(run_channels.frame_send_receiver),
            self.stream_close_manager(
                run_channels.shutdown_api_receiver,
                shutdown_stream_close_manager_receiver,
                run_channels.stream_finished_request_sender,
            ),
            self.shutdown_manager(
                run_channels.shutdown_receiver,
                vec!(
                    shutdown_open_manager_sender,
                    shutdown_stream_close_manager_sender
                )
            ),
        );
    }

    async fn send_frame(&self, frame: Frame) {
        // find out ideal channel here
        //TODO: just take first
        if let Some((_cid, channel)) = self.channels.write().await.get_mut(0) {
            channel.send(frame).await.unwrap();
        } else {
            error!("participant has no channel to communicate on");
        }
    }

    async fn handle_frames(
        &self,
        mut frame_recv_receiver: mpsc::UnboundedReceiver<Frame>,
        mut stream_opened_sender: mpsc::UnboundedSender<Stream>,
        shutdown_api_sender: mpsc::UnboundedSender<Sid>,
        send_outgoing: Arc<Mutex<std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>>>,
    ) {
        trace!("start handle_frames");
        let send_outgoing = { send_outgoing.lock().unwrap().clone() };
        let mut messages = HashMap::new();
        while let Some(frame) = frame_recv_receiver.next().await {
            debug!("handling frame");
            match frame {
                Frame::OpenStream {
                    sid,
                    prio,
                    promises,
                } => {
                    let send_outgoing = send_outgoing.clone();
                    let stream = self
                        .create_stream(sid, prio, promises, send_outgoing, &shutdown_api_sender)
                        .await;
                    stream_opened_sender.send(stream).await.unwrap();
                    trace!("opened frame from remote");
                },
                Frame::CloseStream { sid } => {
                    // Closing is realised by setting a AtomicBool to true, however we also have a
                    // guarantee that send or recv fails if the other one is destroyed
                    // However Stream.send() is not async and their receiver isn't dropped if Steam
                    // is dropped, so i need a way to notify the Stream that it's send messages will
                    // be dropped... from remote, notify local
                    if let Some((_, _, _, closed)) = self.streams.write().await.remove(&sid) {
                        closed.store(true, Ordering::Relaxed);
                    } else {
                        error!(
                            "couldn't find stream to close, either this is a duplicate message, \
                             or the local copy of the Stream got closed simultaniously"
                        );
                    }
                    trace!("closed frame from remote");
                },
                Frame::DataHeader { mid, sid, length } => {
                    let imsg = InCommingMessage {
                        buffer: MessageBuffer { data: Vec::new() },
                        length,
                        mid,
                        sid,
                    };
                    messages.insert(mid, imsg);
                },
                Frame::Data {
                    id,
                    start: _,
                    mut data,
                } => {
                    let finished = if let Some(imsg) = messages.get_mut(&id) {
                        imsg.buffer.data.append(&mut data);
                        imsg.buffer.data.len() as u64 == imsg.length
                    } else {
                        false
                    };
                    if finished {
                        debug!(?id, "finished receiving message");
                        let imsg = messages.remove(&id).unwrap();
                        if let Some((_, _, sender, _)) =
                            self.streams.write().await.get_mut(&imsg.sid)
                        {
                            sender.send(imsg).await.unwrap();
                        } else {
                            error!("dropping message as stream no longer seems to exist");
                        }
                    }
                },
                _ => unreachable!("never reaches frame!"),
            }
        }
        trace!("stop handle_frames");
    }

    async fn send_manager(
        &self,
        mut frame_send_receiver: mpsc::UnboundedReceiver<(Pid, Sid, Frame)>,
    ) {
        trace!("start send_manager");
        while let Some((_, _, frame)) = frame_send_receiver.next().await {
            self.send_frame(frame).await;
        }
        trace!("stop send_manager");
    }

    async fn transfer_channel_manager(
        &self,
        mut transfer_channel_receiver: mpsc::UnboundedReceiver<(Cid, mpsc::UnboundedSender<Frame>)>,
    ) {
        trace!("start transfer_channel_manager");
        while let Some((cid, sender)) = transfer_channel_receiver.next().await {
            debug!(?cid, "got a new channel to listen on");
            self.channels.write().await.push((cid, sender));
        }
        trace!("stop transfer_channel_manager");
    }

    async fn open_manager(
        &self,
        mut stream_open_receiver: mpsc::UnboundedReceiver<(
            Prio,
            Promises,
            oneshot::Sender<Stream>,
        )>,
        shutdown_api_sender: mpsc::UnboundedSender<Sid>,
        send_outgoing: Arc<Mutex<std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>>>,
        shutdown_open_manager_receiver: oneshot::Receiver<()>,
    ) {
        trace!("start open_manager");
        let send_outgoing = {
            //fighting the borrow checker ;)
            send_outgoing.lock().unwrap().clone()
        };
        let mut stream_ids = self.offset_sid;
        let mut shutdown_open_manager_receiver = shutdown_open_manager_receiver.fuse();
        //from api or shutdown signal
        while let Some((prio, promises, sender)) = select! {
            next = stream_open_receiver.next().fuse() => next,
            _ = shutdown_open_manager_receiver => None,
        } {
            debug!(?prio, ?promises, "got request to open a new steam");
            let send_outgoing = send_outgoing.clone();
            let sid = stream_ids;
            let stream = self
                .create_stream(sid, prio, promises, send_outgoing, &shutdown_api_sender)
                .await;
            self.send_frame(Frame::OpenStream {
                sid,
                prio,
                promises,
            })
            .await;
            sender.send(stream).unwrap();
            stream_ids += Sid::from(1);
        }
        trace!("stop open_manager");
    }

    async fn shutdown_manager(
        &self,
        shutdown_receiver: oneshot::Receiver<()>,
        mut to_shutdown: Vec<oneshot::Sender<()>>,
    ) {
        trace!("start shutdown_manager");
        shutdown_receiver.await.unwrap();
        debug!("closing all managers");
        for sender in to_shutdown.drain(..) {
            if sender.send(()).is_err() {
                debug!("manager seems to be closed already, weird, maybe a bug");
            };
        }
        debug!("closing all streams");
        let mut streams = self.streams.write().await;
        for (sid, (_, _, _, closing)) in streams.drain() {
            trace!(?sid, "shutting down Stream");
            closing.store(true, Ordering::Relaxed);
        }
        trace!("stop shutdown_manager");
    }

    async fn stream_close_manager(
        &self,
        mut shutdown_api_receiver: mpsc::UnboundedReceiver<Sid>,
        shutdown_stream_close_manager_receiver: oneshot::Receiver<()>,
        mut stream_finished_request_sender: mpsc::UnboundedSender<(Pid, Sid, oneshot::Sender<()>)>,
    ) {
        trace!("start stream_close_manager");
        let mut shutdown_stream_close_manager_receiver =
            shutdown_stream_close_manager_receiver.fuse();
        //from api or shutdown signal
        while let Some(sid) = select! {
            next = shutdown_api_receiver.next().fuse() => next,
            _ = shutdown_stream_close_manager_receiver => None,
        } {
            trace!(?sid, "got request from api to close steam");
            //TODO: wait here till the last prio was send!
            //The error is, that the close msg as a control message is send directly, while
            // messages are only send after a next prio tick. This means, we
            // close it first, and then send the headers and data packages...
            // ofc the other side then no longer finds the respective stream.
            //however we need to find out when the last message of a stream is send. it
            // would be usefull to get a snapshot here, like, this stream has send out to
            // msgid n, while the prio only has send m. then sleep as long as n < m maybe...
            debug!("IF YOU SEE THIS, FIND A PROPPER FIX FOR CLOSING STREAMS");

            let (sender, receiver) = oneshot::channel();
            trace!(?sid, "wait for stream to be flushed");
            stream_finished_request_sender
                .send((self.remote_pid, sid, sender))
                .await
                .unwrap();
            receiver.await.unwrap();
            trace!(?sid, "stream was successfully flushed");

            self.streams.write().await.remove(&sid);
            //from local, notify remote
            self.send_frame(Frame::CloseStream { sid }).await;
        }
        trace!("stop stream_close_manager");
    }

    async fn create_stream(
        &self,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        send_outgoing: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
        shutdown_api_sender: &mpsc::UnboundedSender<Sid>,
    ) -> Stream {
        let (msg_recv_sender, msg_recv_receiver) = mpsc::unbounded::<InCommingMessage>();
        let closed = Arc::new(AtomicBool::new(false));
        self.streams
            .write()
            .await
            .insert(sid, (prio, promises, msg_recv_sender, closed.clone()));
        Stream::new(
            self.remote_pid,
            sid,
            prio,
            promises,
            send_outgoing,
            msg_recv_receiver,
            closed.clone(),
            shutdown_api_sender.clone(),
        )
    }
}
