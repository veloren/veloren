#[cfg(feature = "metrics")]
use crate::metrics::{NetworkMetrics, PidCidFrameCache};
use crate::{
    api::{ParticipantError, Stream},
    channel::Channel,
    message::{IncomingMessage, MessageBuffer, OutgoingMessage},
    prios::PrioManager,
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
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tracing::*;

pub(crate) type A2bStreamOpen = (Prio, Promises, oneshot::Sender<Stream>);
pub(crate) type S2bCreateChannel = (Cid, Sid, Protocols, Vec<(Cid, Frame)>, oneshot::Sender<()>);
pub(crate) type S2bShutdownBparticipant = oneshot::Sender<Result<(), ParticipantError>>;
pub(crate) type B2sPrioStatistic = (Pid, u64, u64);

#[derive(Debug)]
struct ChannelInfo {
    cid: Cid,
    cid_string: String, //optimisationmetrics
    b2w_frame_s: mpsc::UnboundedSender<Frame>,
    b2r_read_shutdown: oneshot::Sender<()>,
}

#[derive(Debug)]
struct StreamInfo {
    prio: Prio,
    promises: Promises,
    send_closed: Arc<AtomicBool>,
    b2a_msg_recv_s: mpsc::UnboundedSender<IncomingMessage>,
}

#[derive(Debug)]
struct ControlChannels {
    a2b_stream_open_r: mpsc::UnboundedReceiver<A2bStreamOpen>,
    b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
    s2b_create_channel_r: mpsc::UnboundedReceiver<S2bCreateChannel>,
    a2b_close_stream_r: mpsc::UnboundedReceiver<Sid>,
    a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
    s2b_shutdown_bparticipant_r: oneshot::Receiver<S2bShutdownBparticipant>, /* own */
}

#[derive(Debug)]
struct ShutdownInfo {
    //a2b_stream_open_r: mpsc::UnboundedReceiver<A2bStreamOpen>,
    b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
    error: Option<ParticipantError>,
}

#[derive(Debug)]
pub struct BParticipant {
    remote_pid: Pid,
    remote_pid_string: String, //optimisation
    offset_sid: Sid,
    channels: Arc<RwLock<Vec<ChannelInfo>>>,
    streams: RwLock<HashMap<Sid, StreamInfo>>,
    running_mgr: AtomicUsize,
    run_channels: Option<ControlChannels>,
    #[cfg(feature = "metrics")]
    metrics: Arc<NetworkMetrics>,
    no_channel_error_info: RwLock<(Instant, u64)>,
    shutdown_info: RwLock<ShutdownInfo>,
}

impl BParticipant {
    #[allow(clippy::type_complexity)]
    pub(crate) fn new(
        remote_pid: Pid,
        offset_sid: Sid,
        #[cfg(feature = "metrics")] metrics: Arc<NetworkMetrics>,
    ) -> (
        Self,
        mpsc::UnboundedSender<A2bStreamOpen>,
        mpsc::UnboundedReceiver<Stream>,
        mpsc::UnboundedSender<S2bCreateChannel>,
        oneshot::Sender<S2bShutdownBparticipant>,
    ) {
        let (a2b_steam_open_s, a2b_stream_open_r) = mpsc::unbounded::<A2bStreamOpen>();
        let (b2a_stream_opened_s, b2a_stream_opened_r) = mpsc::unbounded::<Stream>();
        let (a2b_close_stream_s, a2b_close_stream_r) = mpsc::unbounded();
        let (s2b_shutdown_bparticipant_s, s2b_shutdown_bparticipant_r) = oneshot::channel();
        let (s2b_create_channel_s, s2b_create_channel_r) = mpsc::unbounded();

        let shutdown_info = RwLock::new(ShutdownInfo {
            //a2b_stream_open_r: a2b_stream_open_r.clone(),
            b2a_stream_opened_s: b2a_stream_opened_s.clone(),
            error: None,
        });

        let run_channels = Some(ControlChannels {
            a2b_stream_open_r,
            b2a_stream_opened_s,
            s2b_create_channel_r,
            a2b_close_stream_r,
            a2b_close_stream_s,
            s2b_shutdown_bparticipant_r,
        });

        (
            Self {
                remote_pid,
                remote_pid_string: remote_pid.to_string(),
                offset_sid,
                channels: Arc::new(RwLock::new(vec![])),
                streams: RwLock::new(HashMap::new()),
                running_mgr: AtomicUsize::new(0),
                run_channels,
                #[cfg(feature = "metrics")]
                metrics,
                no_channel_error_info: RwLock::new((Instant::now(), 0)),
                shutdown_info,
            },
            a2b_steam_open_s,
            b2a_stream_opened_r,
            s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
        )
    }

    pub async fn run(mut self, b2s_prio_statistic_s: mpsc::UnboundedSender<B2sPrioStatistic>) {
        //those managers that listen on api::Participant need an additional oneshot for
        // shutdown scenario, those handled by scheduler will be closed by it.
        let (shutdown_send_mgr_sender, shutdown_send_mgr_receiver) = oneshot::channel();
        let (shutdown_stream_close_mgr_sender, shutdown_stream_close_mgr_receiver) =
            oneshot::channel();
        let (shutdown_open_mgr_sender, shutdown_open_mgr_receiver) = oneshot::channel();
        let (b2b_prios_flushed_s, b2b_prios_flushed_r) = oneshot::channel();
        let (w2b_frames_s, w2b_frames_r) = mpsc::unbounded::<(Cid, Frame)>();
        let (prios, a2p_msg_s, b2p_notify_empty_stream_s) = PrioManager::new(
            #[cfg(feature = "metrics")]
            self.metrics.clone(),
            self.remote_pid_string.clone(),
        );

        let run_channels = self.run_channels.take().unwrap();
        futures::join!(
            self.open_mgr(
                run_channels.a2b_stream_open_r,
                run_channels.a2b_close_stream_s.clone(),
                a2p_msg_s.clone(),
                shutdown_open_mgr_receiver,
            ),
            self.handle_frames_mgr(
                w2b_frames_r,
                run_channels.b2a_stream_opened_s,
                run_channels.a2b_close_stream_s,
                a2p_msg_s.clone(),
            ),
            self.create_channel_mgr(run_channels.s2b_create_channel_r, w2b_frames_s),
            self.send_mgr(
                prios,
                shutdown_send_mgr_receiver,
                b2b_prios_flushed_s,
                b2s_prio_statistic_s
            ),
            self.stream_close_mgr(
                run_channels.a2b_close_stream_r,
                shutdown_stream_close_mgr_receiver,
                b2p_notify_empty_stream_s,
            ),
            self.participant_shutdown_mgr(
                run_channels.s2b_shutdown_bparticipant_r,
                b2b_prios_flushed_r,
                vec![
                    shutdown_send_mgr_sender,
                    shutdown_open_mgr_sender,
                    shutdown_stream_close_mgr_sender,
                ],
            ),
        );
    }

    async fn send_mgr(
        &self,
        mut prios: PrioManager,
        mut shutdown_send_mgr_receiver: oneshot::Receiver<()>,
        b2b_prios_flushed_s: oneshot::Sender<()>,
        mut b2s_prio_statistic_s: mpsc::UnboundedSender<B2sPrioStatistic>,
    ) {
        //This time equals the MINIMUM Latency in average, so keep it down and //Todo:
        // make it configureable or switch to await E.g. Prio 0 = await, prio 50
        // wait for more messages
        const TICK_TIME: Duration = Duration::from_millis(10);
        const FRAMES_PER_TICK: usize = 10005;
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        let mut closing_up = false;
        trace!("Start send_mgr");
        #[cfg(feature = "metrics")]
        let mut send_cache =
            PidCidFrameCache::new(self.metrics.frames_out_total.clone(), self.remote_pid);
        loop {
            let mut frames = VecDeque::new();
            prios.fill_frames(FRAMES_PER_TICK, &mut frames).await;
            let len = frames.len();
            if len > 0 {
                trace!("Tick {}", len);
            }
            for (_, frame) in frames {
                self.send_frame(
                    frame,
                    #[cfg(feature = "metrics")]
                    &mut send_cache,
                )
                .await;
            }
            b2s_prio_statistic_s
                .send((self.remote_pid, len as u64, /*  */ 0))
                .await
                .unwrap();
            async_std::task::sleep(TICK_TIME).await;
            //shutdown after all msg are send!
            if closing_up && (len == 0) {
                break;
            }
            //this IF below the break IF to give it another chance to close all streams
            // closed
            if !closing_up && shutdown_send_mgr_receiver.try_recv().unwrap().is_some() {
                closing_up = true;
            }
        }
        trace!("Stop send_mgr");
        b2b_prios_flushed_s.send(()).unwrap();
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    //retruns false if sending isn't possible. In that case we have to render the
    // Participant `closed`
    #[must_use = "You need to check if the send was successful and report to client!"]
    async fn send_frame(
        &self,
        frame: Frame,
        #[cfg(feature = "metrics")] frames_out_total_cache: &mut PidCidFrameCache,
    ) -> bool {
        // find out ideal channel here
        //TODO: just take first
        let mut lock = self.channels.write().await;
        if let Some(ci) = lock.get_mut(0) {
            //note: this is technically wrong we should only increase when it suceeded, but
            // this requiered me to clone `frame` which is a to big performance impact for
            // error handling
            #[cfg(feature = "metrics")]
            frames_out_total_cache
                .with_label_values(ci.cid, &frame)
                .inc();
            if let Err(e) = ci.b2w_frame_s.send(frame).await {
                warn!(
                    ?e,
                    "The channel got closed unexpectedly, cleaning it up now."
                );
                let ci = lock.remove(0);
                if let Err(e) = ci.b2r_read_shutdown.send(()) {
                    debug!(
                        ?e,
                        "Error shutdowning channel, which is prob fine as we detected it to no \
                         longer work in the first place"
                    );
                };
                //TODO FIXME tags: takeover channel multiple
                info!(
                    "FIXME: the frame is actually drop. which is fine for now as the participant \
                     will be closed, but not if we do channel-takeover"
                );
                //TEMP FIX: as we dont have channel takeover yet drop the whole bParticipant
                self.close_api(Some(ParticipantError::ProtocolFailedUnrecoverable))
                    .await;
                false
            } else {
                true
            }
        } else {
            let mut guard = self.no_channel_error_info.write().await;
            let now = Instant::now();
            if now.duration_since(guard.0) > Duration::from_secs(1) {
                guard.0 = now;
                let occurrences = guard.1 + 1;
                guard.1 = 0;
                error!(?occurrences, "Participant has no channel to communicate on");
            } else {
                guard.1 += 1;
            }
            false
        }
    }

    async fn handle_frames_mgr(
        &self,
        mut w2b_frames_r: mpsc::UnboundedReceiver<(Cid, Frame)>,
        mut b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
        a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
        a2p_msg_s: crossbeam_channel::Sender<(Prio, Sid, OutgoingMessage)>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start handle_frames_mgr");
        let mut messages = HashMap::new();
        let mut dropped_instant = Instant::now();
        let mut dropped_cnt = 0u64;
        let mut dropped_sid = Sid::new(0);

        while let Some((cid, frame)) = w2b_frames_r.next().await {
            //trace!("handling frame");
            #[cfg(feature = "metrics")]
            {
                let cid_string = cid.to_string();
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&self.remote_pid_string, &cid_string, frame.get_string()])
                    .inc();
            }
            #[cfg(not(feature = "metrics"))]
            let _cid = cid;
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
                    trace!("Opened frame from remote");
                },
                Frame::CloseStream { sid } => {
                    // Closing is realised by setting a AtomicBool to true, however we also have a
                    // guarantee that send or recv fails if the other one is destroyed
                    // However Stream.send() is not async and their receiver isn't dropped if Steam
                    // is dropped, so i need a way to notify the Stream that it's send messages will
                    // be dropped... from remote, notify local
                    trace!(
                        ?sid,
                        "Got remote request to close a stream, without flushing it, local \
                         messages are dropped"
                    );
                    // no wait for flush here, as the remote wouldn't care anyway.
                    if let Some(si) = self.streams.write().await.remove(&sid) {
                        #[cfg(feature = "metrics")]
                        self.metrics
                            .streams_closed_total
                            .with_label_values(&[&self.remote_pid_string])
                            .inc();
                        si.send_closed.store(true, Ordering::Relaxed);
                        si.b2a_msg_recv_s.close_channel();
                        trace!(?sid, "Closed stream from remote");
                    } else {
                        warn!(
                            ?sid,
                            "Couldn't find stream to close, either this is a duplicate message, \
                             or the local copy of the Stream got closed simultaniously"
                        );
                    }
                },
                Frame::DataHeader { mid, sid, length } => {
                    let imsg = IncomingMessage {
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
                            if let Err(e) = si.b2a_msg_recv_s.send(imsg).await {
                                warn!(
                                    ?e,
                                    ?mid,
                                    "Dropping message, as streams seem to be in act of beeing \
                                     dropped right now"
                                );
                            }
                        } else {
                            //aggregate errors
                            let n = Instant::now();
                            if dropped_sid != imsg.sid
                                || n.duration_since(dropped_instant) > Duration::from_secs(1)
                            {
                                warn!(
                                    ?dropped_cnt,
                                    "Dropping multiple messages as stream no longer seems to \
                                     exist because it was dropped probably."
                                );
                                dropped_cnt = 0;
                                dropped_instant = n;
                                dropped_sid = imsg.sid;
                            } else {
                                dropped_cnt += 1;
                            }
                        }
                    }
                },
                Frame::Shutdown => {
                    debug!("Shutdown received from remote side");
                    self.close_api(Some(ParticipantError::ParticipantDisconnected))
                        .await;
                },
                f => unreachable!("Frame should never reache participant!: {:?}", f),
            }
        }
        if dropped_cnt > 0 {
            warn!(
                ?dropped_cnt,
                "Dropping multiple messages as stream no longer seems to exist because it was \
                 dropped probably."
            );
        }
        trace!("Stop handle_frames_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn create_channel_mgr(
        &self,
        s2b_create_channel_r: mpsc::UnboundedReceiver<S2bCreateChannel>,
        w2b_frames_s: mpsc::UnboundedSender<(Cid, Frame)>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start create_channel_mgr");
        s2b_create_channel_r
            .for_each_concurrent(
                None,
                |(cid, _, protocol, leftover_cid_frame, b2s_create_channel_done_s)| {
                    // This channel is now configured, and we are running it in scope of the
                    // participant.
                    let w2b_frames_s = w2b_frames_s.clone();
                    let channels = self.channels.clone();
                    async move {
                        let (channel, b2w_frame_s, b2r_read_shutdown) = Channel::new(cid);
                        channels.write().await.push(ChannelInfo {
                            cid,
                            cid_string: cid.to_string(),
                            b2w_frame_s,
                            b2r_read_shutdown,
                        });
                        b2s_create_channel_done_s.send(()).unwrap();
                        #[cfg(feature = "metrics")]
                        self.metrics
                            .channels_connected_total
                            .with_label_values(&[&self.remote_pid_string])
                            .inc();
                        trace!(?cid, "Running channel in participant");
                        channel
                            .run(protocol, w2b_frames_s, leftover_cid_frame)
                            .await;
                        #[cfg(feature = "metrics")]
                        self.metrics
                            .channels_disconnected_total
                            .with_label_values(&[&self.remote_pid_string])
                            .inc();
                        trace!(?cid, "Channel got closed");
                    }
                },
            )
            .await;
        trace!("Stop create_channel_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn open_mgr(
        &self,
        mut a2b_stream_open_r: mpsc::UnboundedReceiver<A2bStreamOpen>,
        a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
        a2p_msg_s: crossbeam_channel::Sender<(Prio, Sid, OutgoingMessage)>,
        shutdown_open_mgr_receiver: oneshot::Receiver<()>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start open_mgr");
        let mut stream_ids = self.offset_sid;
        #[cfg(feature = "metrics")]
        let mut send_cache =
            PidCidFrameCache::new(self.metrics.frames_out_total.clone(), self.remote_pid);
        let mut shutdown_open_mgr_receiver = shutdown_open_mgr_receiver.fuse();
        //from api or shutdown signal
        while let Some((prio, promises, p2a_return_stream)) = select! {
            next = a2b_stream_open_r.next().fuse() => next,
            _ = shutdown_open_mgr_receiver => None,
        } {
            debug!(?prio, ?promises, "Got request to open a new steam");
            //TODO: a2b_stream_open_r isn't closed on api_close yet. This needs to change.
            //till then just check here if we are closed and in that case do nothing (not
            // even answer)
            if self.shutdown_info.read().await.error.is_some() {
                continue;
            }

            let a2p_msg_s = a2p_msg_s.clone();
            let sid = stream_ids;
            let stream = self
                .create_stream(sid, prio, promises, a2p_msg_s, &a2b_close_stream_s)
                .await;
            if self
                .send_frame(
                    Frame::OpenStream {
                        sid,
                        prio,
                        promises,
                    },
                    #[cfg(feature = "metrics")]
                    &mut send_cache,
                )
                .await
            {
                //On error, we drop this, so it gets closed and client will handle this as an
                // Err any way (:
                p2a_return_stream.send(stream).unwrap();
                stream_ids += Sid::from(1);
            }
        }
        trace!("Stop open_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    /// when activated this function will drop the participant completly and
    /// wait for everything to go right! Then return 1. Shutting down
    /// Streams for API and End user! 2. Wait for all "prio queued" Messages
    /// to be send. 3. Send Stream
    /// If BParticipant kills itself managers stay active till this function is
    /// called by api to get the result status
    async fn participant_shutdown_mgr(
        &self,
        s2b_shutdown_bparticipant_r: oneshot::Receiver<S2bShutdownBparticipant>,
        b2b_prios_flushed_r: oneshot::Receiver<()>,
        mut mgr_to_shutdown: Vec<oneshot::Sender<()>>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start participant_shutdown_mgr");
        let sender = s2b_shutdown_bparticipant_r.await.unwrap();

        self.close_api(None).await;

        debug!("Closing all managers");
        for sender in mgr_to_shutdown.drain(..) {
            if let Err(e) = sender.send(()) {
                warn!(?e, "Manager seems to be closed already, weird, maybe a bug");
            };
        }

        b2b_prios_flushed_r.await.unwrap();
        debug!("Closing all channels, after flushed prios");
        for ci in self.channels.write().await.drain(..) {
            if let Err(e) = ci.b2r_read_shutdown.send(()) {
                debug!(?e, ?ci.cid, "Seems like this read protocol got already dropped by closing the Stream itself, just ignoring the fact");
            };
        }

        //Wait for other bparticipants mgr to close via AtomicUsize
        const SLEEP_TIME: Duration = Duration::from_millis(5);
        const ALLOWED_MANAGER: usize = 1;
        async_std::task::sleep(SLEEP_TIME).await;
        let mut i: u32 = 1;
        while self.running_mgr.load(Ordering::Relaxed) > ALLOWED_MANAGER {
            i += 1;
            if i.rem_euclid(10) == 1 {
                trace!(
                    ?ALLOWED_MANAGER,
                    "Waiting for bparticipant mgr to shut down, remaining {}",
                    self.running_mgr.load(Ordering::Relaxed) - ALLOWED_MANAGER
                );
            }
            async_std::task::sleep(SLEEP_TIME * i).await;
        }
        trace!("All BParticipant mgr (except me) are shut down now");

        #[cfg(feature = "metrics")]
        self.metrics.participants_disconnected_total.inc();
        debug!("BParticipant close done");

        let mut lock = self.shutdown_info.write().await;
        sender
            .send(match lock.error.take() {
                None => Ok(()),
                Some(e) => Err(e),
            })
            .unwrap();

        trace!("Stop participant_shutdown_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn stream_close_mgr(
        &self,
        mut a2b_close_stream_r: mpsc::UnboundedReceiver<Sid>,
        shutdown_stream_close_mgr_receiver: oneshot::Receiver<()>,
        b2p_notify_empty_stream_s: crossbeam_channel::Sender<(Sid, oneshot::Sender<()>)>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start stream_close_mgr");
        #[cfg(feature = "metrics")]
        let mut send_cache =
            PidCidFrameCache::new(self.metrics.frames_out_total.clone(), self.remote_pid);
        let mut shutdown_stream_close_mgr_receiver = shutdown_stream_close_mgr_receiver.fuse();

        //from api or shutdown signal
        while let Some(sid) = select! {
            next = a2b_close_stream_r.next().fuse() => next,
            _ = shutdown_stream_close_mgr_receiver => None,
        } {
            //TODO: make this concurrent!
            //TODO: Performance, closing is slow!
            trace!(?sid, "Got request from api to close steam");
            //This needs to first stop clients from sending any more.
            //Then it will wait for all pending messages (in prio) to be send to the
            // protocol After this happened the stream is closed
            //Only after all messages are send to the prococol, we can send the CloseStream
            // frame! If we would send it before, all followup messages couldn't
            // be handled at the remote side.

            trace!(?sid, "Stopping api to use this stream");
            match self.streams.read().await.get(&sid) {
                Some(si) => {
                    si.send_closed.store(true, Ordering::Relaxed);
                    si.b2a_msg_recv_s.close_channel();
                },
                None => warn!("Couldn't find the stream, might be simulanious close from remote"),
            }

            //TODO: what happens if RIGHT NOW the remote sends a StreamClose and this
            // streams get closed and removed? RACE CONDITION
            trace!(?sid, "Wait for stream to be flushed");
            let (s2b_stream_finished_closed_s, s2b_stream_finished_closed_r) = oneshot::channel();
            b2p_notify_empty_stream_s
                .send((sid, s2b_stream_finished_closed_s))
                .unwrap();
            s2b_stream_finished_closed_r.await.unwrap();

            trace!(?sid, "Stream was successfully flushed");
            #[cfg(feature = "metrics")]
            self.metrics
                .streams_closed_total
                .with_label_values(&[&self.remote_pid_string])
                .inc();
            //only now remove the Stream, that means we can still recv on it.
            self.streams.write().await.remove(&sid);
            self.send_frame(
                Frame::CloseStream { sid },
                #[cfg(feature = "metrics")]
                &mut send_cache,
            )
            .await;
        }
        trace!("Stop stream_close_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn create_stream(
        &self,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        a2p_msg_s: crossbeam_channel::Sender<(Prio, Sid, OutgoingMessage)>,
        a2b_close_stream_s: &mpsc::UnboundedSender<Sid>,
    ) -> Stream {
        let (b2a_msg_recv_s, b2a_msg_recv_r) = mpsc::unbounded::<IncomingMessage>();
        let send_closed = Arc::new(AtomicBool::new(false));
        self.streams.write().await.insert(sid, StreamInfo {
            prio,
            promises,
            send_closed: send_closed.clone(),
            b2a_msg_recv_s,
        });
        #[cfg(feature = "metrics")]
        self.metrics
            .streams_opened_total
            .with_label_values(&[&self.remote_pid_string])
            .inc();
        Stream::new(
            self.remote_pid,
            sid,
            prio,
            promises,
            send_closed,
            a2p_msg_s,
            b2a_msg_recv_r,
            a2b_close_stream_s.clone(),
        )
    }

    /// close streams and set err
    async fn close_api(&self, reason: Option<ParticipantError>) {
        //closing api::Participant is done by closing all channels, exepct for the
        // shutdown channel at this point!
        let mut lock = self.shutdown_info.write().await;
        if let Some(r) = reason {
            lock.error = Some(r);
        }
        lock.b2a_stream_opened_s.close_channel();

        debug!("Closing all streams");
        for (sid, si) in self.streams.write().await.drain() {
            trace!(?sid, "Shutting down Stream");
            si.b2a_msg_recv_s.close_channel();
            si.send_closed.store(true, Ordering::Relaxed);
        }
    }
}
