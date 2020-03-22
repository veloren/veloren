use crate::{
    api::Stream,
    frames::Frame,
    message::{InCommingMessage, MessageBuffer, OutGoingMessage},
    types::{Cid, Pid, Prio, Promises, Sid},
};
use async_std::sync::RwLock;
use futures::{
    channel::{mpsc, oneshot},
    sink::SinkExt,
    stream::StreamExt,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
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
                oneshot::Sender<()>,
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
    ) -> (
        Self,
        mpsc::UnboundedSender<(Prio, Promises, oneshot::Sender<Stream>)>,
        mpsc::UnboundedReceiver<Stream>,
        mpsc::UnboundedSender<(Cid, mpsc::UnboundedSender<Frame>)>,
        mpsc::UnboundedSender<Frame>,
        mpsc::UnboundedSender<(Pid, Sid, Frame)>,
    ) {
        let (stream_open_sender, stream_open_receiver) =
            mpsc::unbounded::<(Prio, Promises, oneshot::Sender<Stream>)>();
        let (stream_opened_sender, stream_opened_receiver) = mpsc::unbounded::<Stream>();
        let (transfer_channel_sender, transfer_channel_receiver) =
            mpsc::unbounded::<(Cid, mpsc::UnboundedSender<Frame>)>();
        let (frame_recv_sender, frame_recv_receiver) = mpsc::unbounded::<Frame>();
        //let (shutdown1_sender, shutdown1_receiver) = oneshot::channel();
        let (shutdown_api_sender, shutdown_api_receiver) = mpsc::unbounded();
        let (frame_send_sender, frame_send_receiver) = mpsc::unbounded::<(Pid, Sid, Frame)>();

        let run_channels = Some(ControlChannels {
            stream_open_receiver,
            stream_opened_sender,
            transfer_channel_receiver,
            frame_recv_receiver,
            //shutdown_sender: shutdown1_sender,
            shutdown_api_receiver,
            shutdown_api_sender,
            send_outgoing: Arc::new(Mutex::new(send_outgoing)),
            frame_send_receiver,
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
            //shutdown1_receiver,
        )
    }

    pub async fn run(mut self) {
        let run_channels = self.run_channels.take().unwrap();
        futures::join!(
            self.transfer_channel_manager(run_channels.transfer_channel_receiver),
            self.open_manager(
                run_channels.stream_open_receiver,
                run_channels.shutdown_api_sender.clone(),
                run_channels.send_outgoing.clone(),
            ),
            self.handle_frames(
                run_channels.frame_recv_receiver,
                run_channels.stream_opened_sender,
                run_channels.shutdown_api_sender,
                run_channels.send_outgoing.clone(),
            ),
            self.send_manager(run_channels.frame_send_receiver),
            self.shutdown_manager(run_channels.shutdown_api_receiver,),
        );
    }

    async fn send_frame(&self, frame: Frame) {
        // find out ideal channel
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
                    if let Some((_, _, _, sender)) = self.streams.write().await.remove(&sid) {
                        sender.send(()).unwrap();
                    } else {
                        error!("unreachable, coudln't send close stream event!");
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
    ) {
        trace!("start open_manager");
        let send_outgoing = {
            //fighting the borrow checker ;)
            send_outgoing.lock().unwrap().clone()
        };
        let mut stream_ids = self.offset_sid;
        while let Some((prio, promises, sender)) = stream_open_receiver.next().await {
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
            stream_ids += 1;
        }
        trace!("stop open_manager");
    }

    async fn shutdown_manager(&self, mut shutdown_api_receiver: mpsc::UnboundedReceiver<Sid>) {
        trace!("start shutdown_manager");
        while let Some(sid) = shutdown_api_receiver.next().await {
            trace!(?sid, "got request to close steam");
            self.streams.write().await.remove(&sid);
            self.send_frame(Frame::CloseStream { sid }).await;
        }
        trace!("stop shutdown_manager");
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
        let (shutdown1_sender, shutdown1_receiver) = oneshot::channel();
        self.streams
            .write()
            .await
            .insert(sid, (prio, promises, msg_recv_sender, shutdown1_sender));
        Stream::new(
            self.remote_pid,
            sid,
            prio,
            promises,
            send_outgoing,
            msg_recv_receiver,
            shutdown1_receiver,
            shutdown_api_sender.clone(),
        )
    }
}
