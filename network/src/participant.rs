use crate::{
    api::Stream,
    channel::Channel,
    message::{InCommingMessage, MessageBuffer, OutGoingMessage},
    metrics::NetworkMetrics,
    protocols::Protocols,
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
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tracing::*;

#[derive(Debug)]
struct ChannelInfo {
    cid: Cid,
    b2w_frame_s: mpsc::UnboundedSender<Frame>,
    b2r_read_shutdown: oneshot::Sender<()>,
}

#[derive(Debug)]
struct StreamInfo {
    prio: Prio,
    promises: Promises,
    b2a_msg_recv_s: mpsc::UnboundedSender<InCommingMessage>,
    closed: Arc<AtomicBool>,
}

#[derive(Debug)]
struct ControlChannels {
    a2b_steam_open_r: mpsc::UnboundedReceiver<(Prio, Promises, oneshot::Sender<Stream>)>,
    b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
    s2b_create_channel_r: mpsc::UnboundedReceiver<(Cid, Sid, Protocols, oneshot::Sender<()>)>,
    a2b_close_stream_r: mpsc::UnboundedReceiver<Sid>,
    a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
    a2p_msg_s: Arc<Mutex<std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>>>, //api stream
    p2b_notify_empty_stream_s: Arc<Mutex<std::sync::mpsc::Sender<(Pid, Sid, oneshot::Sender<()>)>>>,
    s2b_frame_r: mpsc::UnboundedReceiver<(Pid, Sid, Frame)>, //scheduler
    s2b_shutdown_bparticipant_r: oneshot::Receiver<oneshot::Sender<async_std::io::Result<()>>>, /* own */
}

#[derive(Debug)]
pub struct BParticipant {
    remote_pid: Pid,
    offset_sid: Sid,
    channels: Arc<RwLock<Vec<ChannelInfo>>>,
    streams: RwLock<HashMap<Sid, StreamInfo>>,
    running_mgr: AtomicUsize,
    run_channels: Option<ControlChannels>,
    metrics: Arc<NetworkMetrics>,
}

impl BParticipant {
    pub(crate) fn new(
        remote_pid: Pid,
        offset_sid: Sid,
        metrics: Arc<NetworkMetrics>,
        a2p_msg_s: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
        p2b_notify_empty_stream_s: std::sync::mpsc::Sender<(Pid, Sid, oneshot::Sender<()>)>,
    ) -> (
        Self,
        mpsc::UnboundedSender<(Prio, Promises, oneshot::Sender<Stream>)>,
        mpsc::UnboundedReceiver<Stream>,
        mpsc::UnboundedSender<(Cid, Sid, Protocols, oneshot::Sender<()>)>,
        mpsc::UnboundedSender<(Pid, Sid, Frame)>,
        oneshot::Sender<oneshot::Sender<async_std::io::Result<()>>>,
    ) {
        let (a2b_steam_open_s, a2b_steam_open_r) =
            mpsc::unbounded::<(Prio, Promises, oneshot::Sender<Stream>)>();
        let (b2a_stream_opened_s, b2a_stream_opened_r) = mpsc::unbounded::<Stream>();
        let (a2b_close_stream_s, a2b_close_stream_r) = mpsc::unbounded();
        let (s2b_frame_s, s2b_frame_r) = mpsc::unbounded::<(Pid, Sid, Frame)>();
        let (s2b_shutdown_bparticipant_s, s2b_shutdown_bparticipant_r) = oneshot::channel();
        let (s2b_create_channel_s, s2b_create_channel_r) =
            mpsc::unbounded::<(Cid, Sid, Protocols, oneshot::Sender<()>)>();

        let run_channels = Some(ControlChannels {
            a2b_steam_open_r,
            b2a_stream_opened_s,
            s2b_create_channel_r,
            a2b_close_stream_r,
            a2b_close_stream_s,
            a2p_msg_s: Arc::new(Mutex::new(a2p_msg_s)),
            p2b_notify_empty_stream_s: Arc::new(Mutex::new(p2b_notify_empty_stream_s)),
            s2b_frame_r,
            s2b_shutdown_bparticipant_r,
        });

        (
            Self {
                remote_pid,
                offset_sid,
                channels: Arc::new(RwLock::new(vec![])),
                streams: RwLock::new(HashMap::new()),
                running_mgr: AtomicUsize::new(0),
                run_channels,
                metrics,
            },
            a2b_steam_open_s,
            b2a_stream_opened_r,
            s2b_create_channel_s,
            s2b_frame_s,
            s2b_shutdown_bparticipant_s,
        )
    }

    pub async fn run(mut self) {
        //those managers that listen on api::Participant need an additional oneshot for
        // shutdown scenario, those handled by scheduler will be closed by it.
        let (shutdown_open_mgr_sender, shutdown_open_mgr_receiver) = oneshot::channel();
        let (shutdown_stream_close_mgr_sender, shutdown_stream_close_mgr_receiver) =
            oneshot::channel();
        let (w2b_frames_s, w2b_frames_r) = mpsc::unbounded::<(Cid, Frame)>();

        let run_channels = self.run_channels.take().unwrap();
        futures::join!(
            self.open_mgr(
                run_channels.a2b_steam_open_r,
                run_channels.a2b_close_stream_s.clone(),
                run_channels.a2p_msg_s.clone(),
                shutdown_open_mgr_receiver,
            ),
            self.handle_frames_mgr(
                w2b_frames_r,
                run_channels.b2a_stream_opened_s,
                run_channels.a2b_close_stream_s,
                run_channels.a2p_msg_s.clone(),
            ),
            self.create_channel_mgr(run_channels.s2b_create_channel_r, w2b_frames_s,),
            self.send_mgr(run_channels.s2b_frame_r),
            self.stream_close_mgr(
                run_channels.a2b_close_stream_r,
                shutdown_stream_close_mgr_receiver,
                run_channels.p2b_notify_empty_stream_s,
            ),
            self.participant_shutdown_mgr(
                run_channels.s2b_shutdown_bparticipant_r,
                vec!(shutdown_open_mgr_sender, shutdown_stream_close_mgr_sender)
            ),
        );
    }

    async fn send_frame(&self, frame: Frame) {
        // find out ideal channel here
        //TODO: just take first
        if let Some(ci) = self.channels.write().await.get_mut(0) {
            self.metrics
                .frames_out_total
                .with_label_values(&[
                    &self.remote_pid.to_string(),
                    &ci.cid.to_string(),
                    frame.get_string(),
                ])
                .inc();
            ci.b2w_frame_s.send(frame).await.unwrap();
        } else {
            error!("participant has no channel to communicate on");
        }
    }

    async fn handle_frames_mgr(
        &self,
        mut w2b_frames_r: mpsc::UnboundedReceiver<(Cid, Frame)>,
        mut b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
        a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
        a2p_msg_s: Arc<Mutex<std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>>>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("start handle_frames");
        let a2p_msg_s = { a2p_msg_s.lock().unwrap().clone() };
        let mut messages = HashMap::new();
        let pid_string = &self.remote_pid.to_string();
        while let Some((cid, frame)) = w2b_frames_r.next().await {
            let cid_string = cid.to_string();
            //trace!("handling frame");
            self.metrics
                .frames_in_total
                .with_label_values(&[&pid_string, &cid_string, frame.get_string()])
                .inc();
            match frame {
                Frame::OpenStream {
                    sid,
                    prio,
                    promises,
                } => {
                    let a2p_msg_s = a2p_msg_s.clone();
                    let stream = self
                        .create_stream(sid, prio, promises, a2p_msg_s, &a2b_close_stream_s)
                        .await;
                    b2a_stream_opened_s.send(stream).await.unwrap();
                    trace!("opened frame from remote");
                },
                Frame::CloseStream { sid } => {
                    // Closing is realised by setting a AtomicBool to true, however we also have a
                    // guarantee that send or recv fails if the other one is destroyed
                    // However Stream.send() is not async and their receiver isn't dropped if Steam
                    // is dropped, so i need a way to notify the Stream that it's send messages will
                    // be dropped... from remote, notify local
                    if let Some(si) = self.streams.write().await.remove(&sid) {
                        self.metrics
                            .streams_closed_total
                            .with_label_values(&[&pid_string])
                            .inc();
                        si.closed.store(true, Ordering::Relaxed);
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
                    mid,
                    start: _,
                    mut data,
                } => {
                    let finished = if let Some(imsg) = messages.get_mut(&mid) {
                        imsg.buffer.data.append(&mut data);
                        imsg.buffer.data.len() as u64 == imsg.length
                    } else {
                        false
                    };
                    if finished {
                        //debug!(?mid, "finished receiving message");
                        let imsg = messages.remove(&mid).unwrap();
                        if let Some(si) = self.streams.write().await.get_mut(&imsg.sid) {
                            si.b2a_msg_recv_s.send(imsg).await.unwrap();
                        } else {
                            error!("dropping message as stream no longer seems to exist");
                        }
                    }
                },
                _ => unreachable!("never reaches frame!"),
            }
        }
        trace!("stop handle_frames");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn create_channel_mgr(
        &self,
        s2b_create_channel_r: mpsc::UnboundedReceiver<(Cid, Sid, Protocols, oneshot::Sender<()>)>,
        w2b_frames_s: mpsc::UnboundedSender<(Cid, Frame)>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("start create_channel_mgr");
        s2b_create_channel_r
            .for_each_concurrent(None, |(cid, sid, protocol, b2s_create_channel_done_s)| {
                // This channel is now configured, and we are running it in scope of the
                // participant.
                let w2b_frames_s = w2b_frames_s.clone();
                let channels = self.channels.clone();
                async move {
                    let (channel, b2w_frame_s, b2r_read_shutdown) =
                        Channel::new(cid, self.remote_pid, self.metrics.clone());
                    channels.write().await.push(ChannelInfo {
                        cid,
                        b2w_frame_s,
                        b2r_read_shutdown,
                    });
                    b2s_create_channel_done_s.send(()).unwrap();
                    self.metrics
                        .channels_connected_total
                        .with_label_values(&[&self.remote_pid.to_string()])
                        .inc();
                    trace!(?cid, "running channel in participant");
                    channel.run(protocol, w2b_frames_s).await;
                    self.metrics
                        .channels_disconnected_total
                        .with_label_values(&[&self.remote_pid.to_string()])
                        .inc();
                    trace!(?cid, "channel got closed");
                }
            })
            .await;
        trace!("stop create_channel_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn send_mgr(&self, mut s2b_frame_r: mpsc::UnboundedReceiver<(Pid, Sid, Frame)>) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("start send_mgr");
        while let Some((_, sid, frame)) = s2b_frame_r.next().await {
            self.send_frame(frame).await;
        }
        trace!("stop send_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn open_mgr(
        &self,
        mut a2b_steam_open_r: mpsc::UnboundedReceiver<(Prio, Promises, oneshot::Sender<Stream>)>,
        a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
        a2p_msg_s: Arc<Mutex<std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>>>,
        shutdown_open_mgr_receiver: oneshot::Receiver<()>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("start open_mgr");
        let send_outgoing = {
            //fighting the borrow checker ;)
            a2p_msg_s.lock().unwrap().clone()
        };
        let mut stream_ids = self.offset_sid;
        let mut shutdown_open_mgr_receiver = shutdown_open_mgr_receiver.fuse();
        //from api or shutdown signal
        while let Some((prio, promises, p2a_return_stream)) = select! {
            next = a2b_steam_open_r.next().fuse() => next,
            _ = shutdown_open_mgr_receiver => None,
        } {
            debug!(?prio, ?promises, "got request to open a new steam");
            let send_outgoing = send_outgoing.clone();
            let sid = stream_ids;
            let stream = self
                .create_stream(sid, prio, promises, send_outgoing, &a2b_close_stream_s)
                .await;
            self.send_frame(Frame::OpenStream {
                sid,
                prio,
                promises,
            })
            .await;
            p2a_return_stream.send(stream).unwrap();
            stream_ids += Sid::from(1);
        }
        trace!("stop open_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    /// when activated this function will drop the participant completly and
    /// wait for everything to go right! Then return 1. Shutting down
    /// Streams for API and End user! 2. Wait for all "prio queued" Messages
    /// to be send. 3. Send Stream
    async fn participant_shutdown_mgr(
        &self,
        s2b_shutdown_bparticipant_r: oneshot::Receiver<oneshot::Sender<async_std::io::Result<()>>>,
        mut to_shutdown: Vec<oneshot::Sender<()>>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("start participant_shutdown_mgr");
        let sender = s2b_shutdown_bparticipant_r.await.unwrap();
        debug!("closing all managers");
        for sender in to_shutdown.drain(..) {
            if sender.send(()).is_err() {
                warn!("manager seems to be closed already, weird, maybe a bug");
            };
        }
        debug!("closing all streams");
        let mut streams = self.streams.write().await;
        for (sid, si) in streams.drain() {
            trace!(?sid, "shutting down Stream");
            si.closed.store(true, Ordering::Relaxed);
        }
        debug!("closing all channels");
        for ci in self.channels.write().await.drain(..) {
            ci.b2r_read_shutdown.send(()).unwrap();
        }
        //Wait for other bparticipants mgr to close via AtomicUsize
        const SLEEP_TIME: std::time::Duration = std::time::Duration::from_millis(5);
        async_std::task::sleep(SLEEP_TIME).await;
        let mut i: u32 = 1;
        while self.running_mgr.load(Ordering::Relaxed) > 1 {
            i += 1;
            if i.rem_euclid(10) == 1 {
                trace!(
                    "waiting for bparticipant mgr to shut down, remaining {}",
                    self.running_mgr.load(Ordering::Relaxed) - 1
                );
            }
            async_std::task::sleep(SLEEP_TIME * i).await;
        }
        trace!("all bparticipant mgr (except me) are shut down now");
        self.metrics.participants_disconnected_total.inc();
        sender.send(Ok(())).unwrap();
        trace!("stop participant_shutdown_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn stream_close_mgr(
        &self,
        mut a2b_close_stream_r: mpsc::UnboundedReceiver<Sid>,
        shutdown_stream_close_mgr_receiver: oneshot::Receiver<()>,
        mut p2b_notify_empty_stream_s: Arc<
            Mutex<std::sync::mpsc::Sender<(Pid, Sid, oneshot::Sender<()>)>>,
        >,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("start stream_close_mgr");
        let mut shutdown_stream_close_mgr_receiver = shutdown_stream_close_mgr_receiver.fuse();

        //from api or shutdown signal
        while let Some(sid) = select! {
            next = a2b_close_stream_r.next().fuse() => next,
            _ = shutdown_stream_close_mgr_receiver => None,
        } {
            //TODO: make this concurrent!
            //TODO: Performance, closing is slow!
            trace!(?sid, "got request from api to close steam");
            //This needs to first stop clients from sending any more.
            //Then it will wait for all pending messages (in prio) to be send to the
            // protocol After this happened the stream is closed
            //Only after all messages are send to the prococol, we can send the CloseStream
            // frame! If we would send it before, all followup messages couldn't
            // be handled at the remote side.

            trace!(?sid, "stopping api to use this stream");
            self.streams
                .read()
                .await
                .get(&sid)
                .unwrap()
                .closed
                .store(true, Ordering::Relaxed);

            trace!(?sid, "wait for stream to be flushed");
            let (s2b_stream_finished_closed_s, s2b_stream_finished_closed_r) = oneshot::channel();
            p2b_notify_empty_stream_s
                .lock()
                .unwrap()
                .send((self.remote_pid, sid, s2b_stream_finished_closed_s))
                .unwrap();
            s2b_stream_finished_closed_r.await.unwrap();

            trace!(?sid, "stream was successfully flushed");
            self.metrics
                .streams_closed_total
                .with_label_values(&[&self.remote_pid.to_string()])
                .inc();
            //only now remove the Stream, that means we can still recv on it.
            self.streams.write().await.remove(&sid);
            self.send_frame(Frame::CloseStream { sid }).await;
        }
        trace!("stop stream_close_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn create_stream(
        &self,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        a2p_msg_s: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
        a2b_close_stream_s: &mpsc::UnboundedSender<Sid>,
    ) -> Stream {
        let (b2a_msg_recv_s, b2a_msg_recv_r) = mpsc::unbounded::<InCommingMessage>();
        let closed = Arc::new(AtomicBool::new(false));
        self.streams.write().await.insert(sid, StreamInfo {
            prio,
            promises,
            b2a_msg_recv_s,
            closed: closed.clone(),
        });
        self.metrics
            .streams_opened_total
            .with_label_values(&[&self.remote_pid.to_string()])
            .inc();
        Stream::new(
            self.remote_pid,
            sid,
            prio,
            promises,
            a2p_msg_s,
            b2a_msg_recv_r,
            closed.clone(),
            a2b_close_stream_s.clone(),
        )
    }
}
