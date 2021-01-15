#[cfg(feature = "metrics")]
use crate::metrics::{MultiCidFrameCache, NetworkMetrics};
use crate::{
    api::{ParticipantError, Stream},
    channel::Channel,
    message::{IncomingMessage, MessageBuffer, OutgoingMessage},
    prios::PrioManager,
    protocols::Protocols,
    types::{Cid, Frame, Pid, Prio, Promises, Sid},
};
use futures_util::{FutureExt, StreamExt};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    runtime::Runtime,
    select,
    sync::{mpsc, oneshot, Mutex, RwLock},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::*;
use tracing_futures::Instrument;

pub(crate) type A2bStreamOpen = (Prio, Promises, oneshot::Sender<Stream>);
pub(crate) type C2pFrame = (Cid, Result<Frame, ()>);
pub(crate) type S2bCreateChannel = (Cid, Sid, Protocols, Vec<C2pFrame>, oneshot::Sender<()>);
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
    b2a_msg_recv_s: Mutex<async_channel::Sender<IncomingMessage>>,
}

#[derive(Debug)]
struct ControlChannels {
    a2b_stream_open_r: mpsc::UnboundedReceiver<A2bStreamOpen>,
    b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
    b2b_close_stream_opened_sender_r: oneshot::Receiver<()>,
    s2b_create_channel_r: mpsc::UnboundedReceiver<S2bCreateChannel>,
    a2b_close_stream_r: mpsc::UnboundedReceiver<Sid>,
    a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
    s2b_shutdown_bparticipant_r: oneshot::Receiver<S2bShutdownBparticipant>, /* own */
}

#[derive(Debug)]
struct ShutdownInfo {
    //a2b_stream_open_r: mpsc::UnboundedReceiver<A2bStreamOpen>,
    b2b_close_stream_opened_sender_s: Option<oneshot::Sender<()>>,
    error: Option<ParticipantError>,
}

#[derive(Debug)]
pub struct BParticipant {
    remote_pid: Pid,
    remote_pid_string: String, //optimisation
    offset_sid: Sid,
    runtime: Arc<Runtime>,
    channels: Arc<RwLock<HashMap<Cid, Mutex<ChannelInfo>>>>,
    streams: RwLock<HashMap<Sid, StreamInfo>>,
    running_mgr: AtomicUsize,
    run_channels: Option<ControlChannels>,
    #[cfg(feature = "metrics")]
    metrics: Arc<NetworkMetrics>,
    no_channel_error_info: RwLock<(Instant, u64)>,
    shutdown_info: RwLock<ShutdownInfo>,
}

impl BParticipant {
    const BANDWIDTH: u64 = 25_000_000;
    const FRAMES_PER_TICK: u64 = Self::BANDWIDTH * Self::TICK_TIME_MS / 1000 / 1400 /*TCP FRAME*/;
    const TICK_TIME: Duration = Duration::from_millis(Self::TICK_TIME_MS);
    //in bit/s
    const TICK_TIME_MS: u64 = 10;

    #[allow(clippy::type_complexity)]
    pub(crate) fn new(
        remote_pid: Pid,
        offset_sid: Sid,
        runtime: Arc<Runtime>,
        #[cfg(feature = "metrics")] metrics: Arc<NetworkMetrics>,
    ) -> (
        Self,
        mpsc::UnboundedSender<A2bStreamOpen>,
        mpsc::UnboundedReceiver<Stream>,
        mpsc::UnboundedSender<S2bCreateChannel>,
        oneshot::Sender<S2bShutdownBparticipant>,
    ) {
        let (a2b_steam_open_s, a2b_stream_open_r) = mpsc::unbounded_channel::<A2bStreamOpen>();
        let (b2a_stream_opened_s, b2a_stream_opened_r) = mpsc::unbounded_channel::<Stream>();
        let (b2b_close_stream_opened_sender_s, b2b_close_stream_opened_sender_r) =
            oneshot::channel();
        let (a2b_close_stream_s, a2b_close_stream_r) = mpsc::unbounded_channel();
        let (s2b_shutdown_bparticipant_s, s2b_shutdown_bparticipant_r) = oneshot::channel();
        let (s2b_create_channel_s, s2b_create_channel_r) = mpsc::unbounded_channel();

        let shutdown_info = RwLock::new(ShutdownInfo {
            //a2b_stream_open_r: a2b_stream_open_r.clone(),
            b2b_close_stream_opened_sender_s: Some(b2b_close_stream_opened_sender_s),
            error: None,
        });

        let run_channels = Some(ControlChannels {
            a2b_stream_open_r,
            b2a_stream_opened_s,
            b2b_close_stream_opened_sender_r,
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
                runtime,
                channels: Arc::new(RwLock::new(HashMap::new())),
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
        let (w2b_frames_s, w2b_frames_r) = mpsc::unbounded_channel::<C2pFrame>();
        let (prios, a2p_msg_s, b2p_notify_empty_stream_s) = PrioManager::new(
            #[cfg(feature = "metrics")]
            Arc::clone(&self.metrics),
            self.remote_pid_string.clone(),
        );

        let run_channels = self.run_channels.take().unwrap();
        tokio::join!(
            self.open_mgr(
                run_channels.a2b_stream_open_r,
                run_channels.a2b_close_stream_s.clone(),
                a2p_msg_s.clone(),
                shutdown_open_mgr_receiver,
            ),
            self.handle_frames_mgr(
                w2b_frames_r,
                run_channels.b2a_stream_opened_s,
                run_channels.b2b_close_stream_opened_sender_r,
                run_channels.a2b_close_stream_s,
                a2p_msg_s.clone(),
            ),
            self.create_channel_mgr(run_channels.s2b_create_channel_r, w2b_frames_s),
            self.send_mgr(prios, shutdown_send_mgr_receiver, b2s_prio_statistic_s),
            self.stream_close_mgr(
                run_channels.a2b_close_stream_r,
                shutdown_stream_close_mgr_receiver,
                b2p_notify_empty_stream_s,
            ),
            self.participant_shutdown_mgr(
                run_channels.s2b_shutdown_bparticipant_r,
                shutdown_open_mgr_sender,
                shutdown_stream_close_mgr_sender,
                shutdown_send_mgr_sender,
            ),
        );
    }

    async fn send_mgr(
        &self,
        mut prios: PrioManager,
        mut shutdown_send_mgr_receiver: oneshot::Receiver<oneshot::Sender<()>>,
        b2s_prio_statistic_s: mpsc::UnboundedSender<B2sPrioStatistic>,
    ) {
        //This time equals the MINIMUM Latency in average, so keep it down and //Todo:
        // make it configurable or switch to await E.g. Prio 0 = await, prio 50
        // wait for more messages
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        let mut b2b_prios_flushed_s = None; //closing up
        trace!("Start send_mgr");
        #[cfg(feature = "metrics")]
        let mut send_cache = MultiCidFrameCache::new(self.metrics.frames_out_total.clone());
        let mut i: u64 = 0;
        loop {
            let mut frames = VecDeque::new();
            prios
                .fill_frames(Self::FRAMES_PER_TICK as usize, &mut frames)
                .await;
            let len = frames.len();
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
                .unwrap();
            tokio::time::sleep(Self::TICK_TIME).await;
            i += 1;
            if i.rem_euclid(1000) == 0 {
                trace!("Did 1000 ticks");
            }
            //shutdown after all msg are send!
            // Make sure this is called after the API is closed, and all streams are known
            // to be droped to the priomgr
            if b2b_prios_flushed_s.is_some() && (len == 0) {
                break;
            }
            if b2b_prios_flushed_s.is_none() {
                if let Ok(prios_flushed_s) = shutdown_send_mgr_receiver.try_recv() {
                    b2b_prios_flushed_s = Some(prios_flushed_s);
                }
            }
        }
        trace!("Stop send_mgr");
        b2b_prios_flushed_s
            .expect("b2b_prios_flushed_s not set")
            .send(())
            .unwrap();
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    //returns false if sending isn't possible. In that case we have to render the
    // Participant `closed`
    #[must_use = "You need to check if the send was successful and report to client!"]
    async fn send_frame(
        &self,
        frame: Frame,
        #[cfg(feature = "metrics")] frames_out_total_cache: &mut MultiCidFrameCache,
    ) -> bool {
        let mut drop_cid = None;
        // TODO: find out ideal channel here

        let res = if let Some(ci) = self.channels.read().await.values().next() {
            let ci = ci.lock().await;
            //we are increasing metrics without checking the result to please
            // borrow_checker. otherwise we would need to close `frame` what we
            // dont want!
            #[cfg(feature = "metrics")]
            frames_out_total_cache
                .with_label_values(ci.cid, &frame)
                .inc();
            if let Err(e) = ci.b2w_frame_s.send(frame) {
                let cid = ci.cid;
                info!(?e, ?cid, "channel no longer available");
                drop_cid = Some(cid);
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
                let lastframe = frame;
                error!(
                    ?occurrences,
                    ?lastframe,
                    "Participant has no channel to communicate on"
                );
            } else {
                guard.1 += 1;
            }
            false
        };
        if let Some(cid) = drop_cid {
            if let Some(ci) = self.channels.write().await.remove(&cid) {
                let ci = ci.into_inner();
                trace!(?cid, "stopping read protocol");
                if let Err(e) = ci.b2r_read_shutdown.send(()) {
                    trace!(?cid, ?e, "seems like was already shut down");
                }
            }
            //TODO FIXME tags: takeover channel multiple
            info!(
                "FIXME: the frame is actually drop. which is fine for now as the participant will \
                 be closed, but not if we do channel-takeover"
            );
            //TEMP FIX: as we dont have channel takeover yet drop the whole bParticipant
            self.close_write_api(Some(ParticipantError::ProtocolFailedUnrecoverable))
                .await;
        };
        res
    }

    async fn handle_frames_mgr(
        &self,
        mut w2b_frames_r: mpsc::UnboundedReceiver<C2pFrame>,
        b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
        b2b_close_stream_opened_sender_r: oneshot::Receiver<()>,
        a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
        a2p_msg_s: crossbeam_channel::Sender<(Prio, Sid, OutgoingMessage)>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start handle_frames_mgr");
        let mut messages = HashMap::new();
        #[cfg(feature = "metrics")]
        let mut send_cache = MultiCidFrameCache::new(self.metrics.frames_out_total.clone());
        let mut dropped_instant = Instant::now();
        let mut dropped_cnt = 0u64;
        let mut dropped_sid = Sid::new(0);
        let mut b2a_stream_opened_s = Some(b2a_stream_opened_s);
        let mut b2b_close_stream_opened_sender_r = b2b_close_stream_opened_sender_r.fuse();

        while let Some((cid, result_frame)) = select!(
            next = w2b_frames_r.recv().fuse() => next,
            _ = &mut b2b_close_stream_opened_sender_r => {
                b2a_stream_opened_s = None;
                None
            },
        ) {
            //trace!(?result_frame, "handling frame");
            let frame = match result_frame {
                Ok(frame) => frame,
                Err(()) => {
                    // The read protocol stopped, i need to make sure that write gets stopped, can
                    // drop channel as it's dead anyway
                    debug!("read protocol was closed. Stopping channel");
                    self.channels.write().await.remove(&cid);
                    continue;
                },
            };
            #[cfg(feature = "metrics")]
            {
                let cid_string = cid.to_string();
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&cid_string, frame.get_string()])
                    .inc();
            }
            match frame {
                Frame::OpenStream {
                    sid,
                    prio,
                    promises,
                } => {
                    trace!(?sid, ?prio, ?promises, "Opened frame from remote");
                    let a2p_msg_s = a2p_msg_s.clone();
                    let stream = self
                        .create_stream(sid, prio, promises, a2p_msg_s, &a2b_close_stream_s)
                        .await;
                    match &b2a_stream_opened_s {
                        None => debug!("dropping openStream as Channel is already closing"),
                        Some(s) => {
                            if let Err(e) = s.send(stream) {
                                warn!(
                                    ?e,
                                    ?sid,
                                    "couldn't notify api::Participant that a stream got opened. \
                                     Is the participant already dropped?"
                                );
                            }
                        },
                    }
                },
                Frame::CloseStream { sid } => {
                    // no need to keep flushing as the remote no longer knows about this stream
                    // anyway
                    self.delete_stream(
                        sid,
                        None,
                        true,
                        #[cfg(feature = "metrics")]
                        &mut send_cache,
                    )
                    .await;
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
                        //trace!(?mid, "finished receiving message");
                        let imsg = messages.remove(&mid).unwrap();
                        if let Some(si) = self.streams.read().await.get(&imsg.sid) {
                            if let Err(e) = si.b2a_msg_recv_s.lock().await.send(imsg).await {
                                warn!(
                                    ?e,
                                    ?mid,
                                    "Dropping message, as streams seem to be in act of being \
                                     dropped right now"
                                );
                            }
                        } else {
                            //aggregate errors
                            let n = Instant::now();
                            if dropped_cnt > 0
                                && (dropped_sid != imsg.sid
                                    || n.duration_since(dropped_instant) > Duration::from_secs(1))
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
                f => {
                    unreachable!(
                        "Frame should never reach participant!: {:?}, cid: {}",
                        f, cid
                    );
                },
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
        w2b_frames_s: mpsc::UnboundedSender<C2pFrame>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start create_channel_mgr");
        let s2b_create_channel_r = UnboundedReceiverStream::new(s2b_create_channel_r);
        s2b_create_channel_r
            .for_each_concurrent(
                None,
                |(cid, _, protocol, leftover_cid_frame, b2s_create_channel_done_s)| {
                    // This channel is now configured, and we are running it in scope of the
                    // participant.
                    let w2b_frames_s = w2b_frames_s.clone();
                    let channels = Arc::clone(&self.channels);
                    async move {
                        let (channel, b2w_frame_s, b2r_read_shutdown) = Channel::new(cid);
                        let mut lock = channels.write().await;
                        #[cfg(feature = "metrics")]
                        let mut channel_no = lock.len();
                        #[cfg(not(feature = "metrics"))]
                        let channel_no = lock.len();
                        lock.insert(
                            cid,
                            Mutex::new(ChannelInfo {
                                cid,
                                cid_string: cid.to_string(),
                                b2w_frame_s,
                                b2r_read_shutdown,
                            }),
                        );
                        drop(lock);
                        b2s_create_channel_done_s.send(()).unwrap();
                        #[cfg(feature = "metrics")]
                        {
                            self.metrics
                                .channels_connected_total
                                .with_label_values(&[&self.remote_pid_string])
                                .inc();
                            if channel_no > 5 {
                                debug!(?channel_no, "metrics will overwrite channel #5");
                                channel_no = 5;
                            }
                            self.metrics
                                .participants_channel_ids
                                .with_label_values(&[
                                    &self.remote_pid_string,
                                    &channel_no.to_string(),
                                ])
                                .set(cid as i64);
                        }
                        trace!(?cid, ?channel_no, "Running channel in participant");
                        channel
                            .run(protocol, w2b_frames_s, leftover_cid_frame)
                            .instrument(tracing::info_span!("", ?cid))
                            .await;
                        #[cfg(feature = "metrics")]
                        self.metrics
                            .channels_disconnected_total
                            .with_label_values(&[&self.remote_pid_string])
                            .inc();
                        info!(?cid, "Channel got closed");
                        //maybe channel got already dropped, we don't know.
                        channels.write().await.remove(&cid);
                        trace!(?cid, "Channel cleanup completed");
                        //TEMP FIX: as we dont have channel takeover yet drop the whole
                        // bParticipant
                        self.close_write_api(None).await;
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
        let mut send_cache = MultiCidFrameCache::new(self.metrics.frames_out_total.clone());
        let mut shutdown_open_mgr_receiver = shutdown_open_mgr_receiver.fuse();
        //from api or shutdown signal
        while let Some((prio, promises, p2a_return_stream)) = select! {
            next = a2b_stream_open_r.recv().fuse() => next,
            _ = &mut shutdown_open_mgr_receiver => None,
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

    /// when activated this function will drop the participant completely and
    /// wait for everything to go right! Then return 1. Shutting down
    /// Streams for API and End user! 2. Wait for all "prio queued" Messages
    /// to be send. 3. Send Stream
    /// If BParticipant kills itself managers stay active till this function is
    /// called by api to get the result status
    async fn participant_shutdown_mgr(
        &self,
        s2b_shutdown_bparticipant_r: oneshot::Receiver<S2bShutdownBparticipant>,
        shutdown_open_mgr_sender: oneshot::Sender<()>,
        shutdown_stream_close_mgr_sender: oneshot::Sender<oneshot::Sender<()>>,
        shutdown_send_mgr_sender: oneshot::Sender<oneshot::Sender<()>>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start participant_shutdown_mgr");
        let sender = s2b_shutdown_bparticipant_r.await.unwrap();

        #[cfg(feature = "metrics")]
        let mut send_cache = MultiCidFrameCache::new(self.metrics.frames_out_total.clone());

        self.close_api(None).await;

        debug!("Closing all managers");
        shutdown_open_mgr_sender
            .send(())
            .expect("open_mgr must have crashed before");
        let (b2b_stream_close_shutdown_confirmed_s, b2b_stream_close_shutdown_confirmed_r) =
            oneshot::channel();
        shutdown_stream_close_mgr_sender
            .send(b2b_stream_close_shutdown_confirmed_s)
            .expect("stream_close_mgr must have crashed before");
        // We need to wait for the stream_close_mgr BEFORE send_mgr, as the
        // stream_close_mgr needs to wait on the API to drop `Stream` and be triggered
        // It will then sleep for streams to be flushed in PRIO, and send_mgr is
        // responsible for ticking PRIO WHILE this happens, so we cant close it before!
        b2b_stream_close_shutdown_confirmed_r.await.unwrap();

        //closing send_mgr now:
        let (b2b_prios_flushed_s, b2b_prios_flushed_r) = oneshot::channel();
        shutdown_send_mgr_sender
            .send(b2b_prios_flushed_s)
            .expect("stream_close_mgr must have crashed before");
        b2b_prios_flushed_r.await.unwrap();

        if Some(ParticipantError::ParticipantDisconnected) != self.shutdown_info.read().await.error
        {
            debug!("Sending shutdown frame after flushed all prios");
            if !self
                .send_frame(
                    Frame::Shutdown,
                    #[cfg(feature = "metrics")]
                    &mut send_cache,
                )
                .await
            {
                warn!("couldn't send shutdown frame, are channels already closed?");
            }
        }

        debug!("Closing all channels, after flushed prios");
        for (cid, ci) in self.channels.write().await.drain() {
            let ci = ci.into_inner();
            if let Err(e) = ci.b2r_read_shutdown.send(()) {
                debug!(
                    ?e,
                    ?cid,
                    "Seems like this read protocol got already dropped by closing the Stream \
                     itself, ignoring"
                );
            };
        }

        //Wait for other bparticipants mgr to close via AtomicUsize
        const SLEEP_TIME: Duration = Duration::from_millis(5);
        const ALLOWED_MANAGER: usize = 1;
        tokio::time::sleep(SLEEP_TIME).await;
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
            tokio::time::sleep(SLEEP_TIME * i).await;
        }
        trace!("All BParticipant mgr (except me) are shut down now");

        #[cfg(feature = "metrics")]
        self.metrics.participants_disconnected_total.inc();
        debug!("BParticipant close done");

        let mut lock = self.shutdown_info.write().await;
        sender
            .send(match lock.error.take() {
                None => Ok(()),
                Some(ParticipantError::ProtocolFailedUnrecoverable) => {
                    Err(ParticipantError::ProtocolFailedUnrecoverable)
                },
                Some(ParticipantError::ParticipantDisconnected) => Ok(()),
            })
            .unwrap();

        trace!("Stop participant_shutdown_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn stream_close_mgr(
        &self,
        mut a2b_close_stream_r: mpsc::UnboundedReceiver<Sid>,
        shutdown_stream_close_mgr_receiver: oneshot::Receiver<oneshot::Sender<()>>,
        b2p_notify_empty_stream_s: crossbeam_channel::Sender<(Sid, oneshot::Sender<()>)>,
    ) {
        self.running_mgr.fetch_add(1, Ordering::Relaxed);
        trace!("Start stream_close_mgr");
        #[cfg(feature = "metrics")]
        let mut send_cache = MultiCidFrameCache::new(self.metrics.frames_out_total.clone());
        let mut shutdown_stream_close_mgr_receiver = shutdown_stream_close_mgr_receiver.fuse();
        let mut b2b_stream_close_shutdown_confirmed_s = None;

        //from api or shutdown signal
        while let Some(sid) = select! {
            next = a2b_close_stream_r.recv().fuse() => next,
            sender = &mut shutdown_stream_close_mgr_receiver => {
                b2b_stream_close_shutdown_confirmed_s = Some(sender.unwrap());
                None
            }
        } {
            //TODO: make this concurrent!
            //TODO: Performance, closing is slow!
            self.delete_stream(
                sid,
                Some(b2p_notify_empty_stream_s.clone()),
                false,
                #[cfg(feature = "metrics")]
                &mut send_cache,
            )
            .await;
        }
        trace!("deleting all leftover streams");
        let sids = self
            .streams
            .read()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for sid in sids {
            //flushing is still important, e.g. when Participant::drop is called (but
            // Stream:drop isn't)!
            self.delete_stream(
                sid,
                Some(b2p_notify_empty_stream_s.clone()),
                false,
                #[cfg(feature = "metrics")]
                &mut send_cache,
            )
            .await;
        }
        if b2b_stream_close_shutdown_confirmed_s.is_none() {
            b2b_stream_close_shutdown_confirmed_s =
                Some(shutdown_stream_close_mgr_receiver.await.unwrap());
        }
        b2b_stream_close_shutdown_confirmed_s
            .unwrap()
            .send(())
            .unwrap();
        trace!("Stop stream_close_mgr");
        self.running_mgr.fetch_sub(1, Ordering::Relaxed);
    }

    async fn delete_stream(
        &self,
        sid: Sid,
        b2p_notify_empty_stream_s: Option<crossbeam_channel::Sender<(Sid, oneshot::Sender<()>)>>,
        from_remote: bool,
        #[cfg(feature = "metrics")] frames_out_total_cache: &mut MultiCidFrameCache,
    ) {
        //This needs to first stop clients from sending any more.
        //Then it will wait for all pending messages (in prio) to be send to the
        // protocol After this happened the stream is closed
        //Only after all messages are send to the protocol, we can send the CloseStream
        // frame! If we would send it before, all followup messages couldn't
        // be handled at the remote side.
        async {
            trace!("Stopping api to use this stream");
            match self.streams.read().await.get(&sid) {
                Some(si) => {
                    si.send_closed.store(true, Ordering::Relaxed);
                    si.b2a_msg_recv_s.lock().await.close();
                },
                None => trace!(
                    "Couldn't find the stream, might be simultaneous close from local/remote"
                ),
            }

            if !from_remote {
                trace!("Wait for stream to be flushed");
                let (s2b_stream_finished_closed_s, s2b_stream_finished_closed_r) =
                    oneshot::channel();
                b2p_notify_empty_stream_s
                    .expect("needs to be set when from_remote is false")
                    .send((sid, s2b_stream_finished_closed_s))
                    .unwrap();
                s2b_stream_finished_closed_r.await.unwrap();

                trace!("Stream was successfully flushed");
            }

            #[cfg(feature = "metrics")]
            self.metrics
                .streams_closed_total
                .with_label_values(&[&self.remote_pid_string])
                .inc();
            //only now remove the Stream, that means we can still recv on it.
            self.streams.write().await.remove(&sid);

            if !from_remote {
                self.send_frame(
                    Frame::CloseStream { sid },
                    #[cfg(feature = "metrics")]
                    frames_out_total_cache,
                )
                .await;
            }
        }
        .instrument(tracing::info_span!("close", ?sid, ?from_remote))
        .await;
    }

    async fn create_stream(
        &self,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        a2p_msg_s: crossbeam_channel::Sender<(Prio, Sid, OutgoingMessage)>,
        a2b_close_stream_s: &mpsc::UnboundedSender<Sid>,
    ) -> Stream {
        let (b2a_msg_recv_s, b2a_msg_recv_r) = async_channel::unbounded::<IncomingMessage>();
        let send_closed = Arc::new(AtomicBool::new(false));
        self.streams.write().await.insert(sid, StreamInfo {
            prio,
            promises,
            send_closed: Arc::clone(&send_closed),
            b2a_msg_recv_s: Mutex::new(b2a_msg_recv_s),
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

    async fn close_write_api(&self, reason: Option<ParticipantError>) {
        trace!(?reason, "close_api");
        let mut lock = self.shutdown_info.write().await;
        if let Some(r) = reason {
            lock.error = Some(r);
        }
        lock.b2b_close_stream_opened_sender_s
            .take()
            .map(|s| s.send(()));

        debug!("Closing all streams for write");
        for (sid, si) in self.streams.read().await.iter() {
            trace!(?sid, "Shutting down Stream for write");
            si.send_closed.store(true, Ordering::Relaxed);
        }
    }

    ///closing api::Participant is done by closing all channels, expect for the
    /// shutdown channel at this point!
    async fn close_api(&self, reason: Option<ParticipantError>) {
        self.close_write_api(reason).await;
        debug!("Closing all streams");
        for (sid, si) in self.streams.read().await.iter() {
            trace!(?sid, "Shutting down Stream");
            si.b2a_msg_recv_s.lock().await.close();
        }
    }
}
