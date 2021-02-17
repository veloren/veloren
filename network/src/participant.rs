use crate::{
    api::{ParticipantError, Stream},
    channel::{Protocols, RecvProtocols, SendProtocols},
    metrics::NetworkMetrics,
};
use bytes::Bytes;
use futures_util::{FutureExt, StreamExt};
use network_protocol::{
    Bandwidth, Cid, Pid, Prio, Promises, ProtocolEvent, RecvProtocol, SendProtocol, Sid,
};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicI32, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    select,
    sync::{mpsc, oneshot, Mutex, RwLock},
    task::JoinHandle,
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::*;

pub(crate) type A2bStreamOpen = (Prio, Promises, Bandwidth, oneshot::Sender<Stream>);
pub(crate) type S2bCreateChannel = (Cid, Sid, Protocols, oneshot::Sender<()>);
pub(crate) type S2bShutdownBparticipant = (Duration, oneshot::Sender<Result<(), ParticipantError>>);
pub(crate) type B2sPrioStatistic = (Pid, u64, u64);

#[derive(Debug)]
struct ChannelInfo {
    cid: Cid,
    cid_string: String, //optimisationmetrics
}

#[derive(Debug)]
struct StreamInfo {
    prio: Prio,
    promises: Promises,
    send_closed: Arc<AtomicBool>,
    b2a_msg_recv_s: Mutex<async_channel::Sender<Bytes>>,
}

#[derive(Debug)]
struct ControlChannels {
    a2b_open_stream_r: mpsc::UnboundedReceiver<A2bStreamOpen>,
    b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
    s2b_create_channel_r: mpsc::UnboundedReceiver<S2bCreateChannel>,
    s2b_shutdown_bparticipant_r: oneshot::Receiver<S2bShutdownBparticipant>, /* own */
}

#[derive(Debug)]
struct ShutdownInfo {
    b2b_close_stream_opened_sender_s: Option<oneshot::Sender<()>>,
    error: Option<ParticipantError>,
}

#[derive(Debug)]
pub struct BParticipant {
    local_pid: Pid, //tracing
    remote_pid: Pid,
    remote_pid_string: String, //optimisation
    offset_sid: Sid,
    channels: Arc<RwLock<HashMap<Cid, Mutex<ChannelInfo>>>>,
    streams: RwLock<HashMap<Sid, StreamInfo>>,
    run_channels: Option<ControlChannels>,
    shutdown_barrier: AtomicI32,
    metrics: Arc<NetworkMetrics>,
    no_channel_error_info: RwLock<(Instant, u64)>,
}

impl BParticipant {
    // We use integer instead of Barrier to not block mgr from freeing at the end
    const BARR_CHANNEL: i32 = 1;
    const BARR_RECV: i32 = 4;
    const BARR_SEND: i32 = 2;
    const TICK_TIME: Duration = Duration::from_millis(Self::TICK_TIME_MS);
    const TICK_TIME_MS: u64 = 10;

    #[allow(clippy::type_complexity)]
    pub(crate) fn new(
        local_pid: Pid,
        remote_pid: Pid,
        offset_sid: Sid,
        metrics: Arc<NetworkMetrics>,
    ) -> (
        Self,
        mpsc::UnboundedSender<A2bStreamOpen>,
        mpsc::UnboundedReceiver<Stream>,
        mpsc::UnboundedSender<S2bCreateChannel>,
        oneshot::Sender<S2bShutdownBparticipant>,
    ) {
        let (a2b_open_stream_s, a2b_open_stream_r) = mpsc::unbounded_channel::<A2bStreamOpen>();
        let (b2a_stream_opened_s, b2a_stream_opened_r) = mpsc::unbounded_channel::<Stream>();
        let (s2b_shutdown_bparticipant_s, s2b_shutdown_bparticipant_r) = oneshot::channel();
        let (s2b_create_channel_s, s2b_create_channel_r) = mpsc::unbounded_channel();

        let run_channels = Some(ControlChannels {
            a2b_open_stream_r,
            b2a_stream_opened_s,
            s2b_create_channel_r,
            s2b_shutdown_bparticipant_r,
        });

        (
            Self {
                local_pid,
                remote_pid,
                remote_pid_string: remote_pid.to_string(),
                offset_sid,
                channels: Arc::new(RwLock::new(HashMap::new())),
                streams: RwLock::new(HashMap::new()),
                shutdown_barrier: AtomicI32::new(
                    Self::BARR_CHANNEL + Self::BARR_SEND + Self::BARR_RECV,
                ),
                run_channels,
                metrics,
                no_channel_error_info: RwLock::new((Instant::now(), 0)),
            },
            a2b_open_stream_s,
            b2a_stream_opened_r,
            s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
        )
    }

    pub async fn run(mut self, b2s_prio_statistic_s: mpsc::UnboundedSender<B2sPrioStatistic>) {
        let (b2b_add_send_protocol_s, b2b_add_send_protocol_r) =
            mpsc::unbounded_channel::<(Cid, SendProtocols)>();
        let (b2b_add_recv_protocol_s, b2b_add_recv_protocol_r) =
            mpsc::unbounded_channel::<(Cid, RecvProtocols)>();
        let (b2b_close_send_protocol_s, b2b_close_send_protocol_r) =
            async_channel::unbounded::<Cid>();
        let (b2b_force_close_recv_protocol_s, b2b_force_close_recv_protocol_r) =
            async_channel::unbounded::<Cid>();
        let (b2b_notify_send_of_recv_s, b2b_notify_send_of_recv_r) =
            crossbeam_channel::unbounded::<ProtocolEvent>();

        let (a2b_close_stream_s, a2b_close_stream_r) = mpsc::unbounded_channel::<Sid>();
        const STREAM_BOUND: usize = 10_000;
        let (a2b_msg_s, a2b_msg_r) = crossbeam_channel::bounded::<(Sid, Bytes)>(STREAM_BOUND);

        let run_channels = self.run_channels.take().unwrap();
        trace!("start all managers");
        tokio::join!(
            self.send_mgr(
                run_channels.a2b_open_stream_r,
                a2b_close_stream_r,
                a2b_msg_r,
                b2b_add_send_protocol_r,
                b2b_close_send_protocol_r,
                b2b_notify_send_of_recv_r,
                b2s_prio_statistic_s,
                a2b_msg_s.clone(),          //self
                a2b_close_stream_s.clone(), //self
            )
            .instrument(tracing::info_span!("send")),
            self.recv_mgr(
                run_channels.b2a_stream_opened_s,
                b2b_add_recv_protocol_r,
                b2b_force_close_recv_protocol_r,
                b2b_close_send_protocol_s.clone(),
                b2b_notify_send_of_recv_s,
                a2b_msg_s.clone(),          //self
                a2b_close_stream_s.clone(), //self
            )
            .instrument(tracing::info_span!("recv")),
            self.create_channel_mgr(
                run_channels.s2b_create_channel_r,
                b2b_add_send_protocol_s,
                b2b_add_recv_protocol_s,
            ),
            self.participant_shutdown_mgr(
                run_channels.s2b_shutdown_bparticipant_r,
                b2b_close_send_protocol_s.clone(),
                b2b_force_close_recv_protocol_s,
            ),
        );
    }

    //TODO: local stream_cid: HashMap<Sid, Cid> to know the respective protocol
    #[allow(clippy::too_many_arguments)]
    async fn send_mgr(
        &self,
        mut a2b_open_stream_r: mpsc::UnboundedReceiver<A2bStreamOpen>,
        mut a2b_close_stream_r: mpsc::UnboundedReceiver<Sid>,
        a2b_msg_r: crossbeam_channel::Receiver<(Sid, Bytes)>,
        mut b2b_add_protocol_r: mpsc::UnboundedReceiver<(Cid, SendProtocols)>,
        b2b_close_send_protocol_r: async_channel::Receiver<Cid>,
        b2b_notify_send_of_recv_r: crossbeam_channel::Receiver<ProtocolEvent>,
        _b2s_prio_statistic_s: mpsc::UnboundedSender<B2sPrioStatistic>,
        a2b_msg_s: crossbeam_channel::Sender<(Sid, Bytes)>,
        a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
    ) {
        let mut send_protocols: HashMap<Cid, SendProtocols> = HashMap::new();
        let mut interval = tokio::time::interval(Self::TICK_TIME);
        let mut last_instant = Instant::now();
        let mut stream_ids = self.offset_sid;
        let mut fake_mid = 0; //TODO: move MID to protocol, should be inc per stream ? or ?
        trace!("workaround, actively wait for first protocol");
        b2b_add_protocol_r
            .recv()
            .await
            .map(|(c, p)| send_protocols.insert(c, p));
        loop {
            let (open, close, _, addp, remp) = select!(
                Some(n) = a2b_open_stream_r.recv().fuse() => (Some(n), None, None, None, None),
                Some(n) = a2b_close_stream_r.recv().fuse() => (None, Some(n), None, None, None),
                _ = interval.tick() => (None, None, Some(()), None, None),
                Some(n) = b2b_add_protocol_r.recv().fuse() => (None, None, None, Some(n), None),
                Ok(n) = b2b_close_send_protocol_r.recv().fuse() => (None, None, None, None, Some(n)),
            );

            addp.map(|(cid, p)| {
                debug!(?cid, "add protocol");
                send_protocols.insert(cid, p)
            });

            let (cid, active) = match send_protocols.iter_mut().next() {
                Some((cid, a)) => (*cid, a),
                None => {
                    warn!("no channel");
                    continue;
                },
            };

            let active_err = async {
                if let Some((prio, promises, guaranteed_bandwidth, return_s)) = open {
                    let sid = stream_ids;
                    trace!(?sid, "open stream");
                    stream_ids += Sid::from(1);
                    let stream = self
                        .create_stream(
                            sid,
                            prio,
                            promises,
                            guaranteed_bandwidth,
                            &a2b_msg_s,
                            &a2b_close_stream_s,
                        )
                        .await;

                    let event = ProtocolEvent::OpenStream {
                        sid,
                        prio,
                        promises,
                        guaranteed_bandwidth,
                    };

                    return_s.send(stream).unwrap();
                    active.send(event).await?;
                }

                // process recv content first
                let mut closeevents = b2b_notify_send_of_recv_r
                    .try_iter()
                    .map(|e| {
                        if matches!(e, ProtocolEvent::OpenStream { .. }) {
                            active.notify_from_recv(e);
                            None
                        } else {
                            Some(e)
                        }
                    })
                    .collect::<Vec<_>>();

                // get all messages and assign it to a channel
                for (sid, buffer) in a2b_msg_r.try_iter() {
                    fake_mid += 1;
                    active
                        .send(ProtocolEvent::Message {
                            data: buffer,
                            mid: fake_mid,
                            sid,
                        })
                        .await?
                }

                // process recv content afterwards
                let _ = closeevents.drain(..).map(|e| {
                    if let Some(e) = e {
                        active.notify_from_recv(e);
                    }
                });

                if let Some(sid) = close {
                    trace!(?stream_ids, "delete stream");
                    self.delete_stream(sid).await;
                    // Fire&Forget the protocol will take care to verify that this Frame is delayed
                    // till the last msg was received!
                    active.send(ProtocolEvent::CloseStream { sid }).await?;
                }

                let send_time = Instant::now();
                let diff = send_time.duration_since(last_instant);
                last_instant = send_time;
                active.flush(1_000_000_000, diff).await?; //this actually blocks, so we cant set streams while it.
                let r: Result<(), network_protocol::ProtocolError> = Ok(());
                r
            }
            .await;
            if let Err(e) = active_err {
                info!(?cid, ?e, "protocol failed, shutting down channel");
                // remote recv will now fail, which will trigger remote send which will trigger
                // recv
                send_protocols.remove(&cid).unwrap();
                self.metrics.channels_disconnected(&self.remote_pid_string);
            }

            if let Some(cid) = remp {
                debug!(?cid, "remove protocol");
                match send_protocols.remove(&cid) {
                    Some(mut prot) => {
                        self.metrics.channels_disconnected(&self.remote_pid_string);
                        trace!("blocking flush");
                        let _ = prot.flush(u64::MAX, Duration::from_secs(1)).await;
                        trace!("shutdown prot");
                        let _ = prot.send(ProtocolEvent::Shutdown).await;
                    },
                    None => trace!("tried to remove protocol twice"),
                };
                if send_protocols.is_empty() {
                    break;
                }
            }
        }
        trace!("Stop send_mgr");
        self.shutdown_barrier
            .fetch_sub(Self::BARR_SEND, Ordering::Relaxed);
    }

    #[allow(clippy::too_many_arguments)]
    async fn recv_mgr(
        &self,
        b2a_stream_opened_s: mpsc::UnboundedSender<Stream>,
        mut b2b_add_protocol_r: mpsc::UnboundedReceiver<(Cid, RecvProtocols)>,
        b2b_force_close_recv_protocol_r: async_channel::Receiver<Cid>,
        b2b_close_send_protocol_s: async_channel::Sender<Cid>,
        b2b_notify_send_of_recv_s: crossbeam_channel::Sender<ProtocolEvent>,
        a2b_msg_s: crossbeam_channel::Sender<(Sid, Bytes)>,
        a2b_close_stream_s: mpsc::UnboundedSender<Sid>,
    ) {
        let mut recv_protocols: HashMap<Cid, JoinHandle<()>> = HashMap::new();
        // we should be able to directly await futures imo
        let (hacky_recv_s, mut hacky_recv_r) = mpsc::unbounded_channel();

        let retrigger = |cid: Cid, mut p: RecvProtocols, map: &mut HashMap<_, _>| {
            let hacky_recv_s = hacky_recv_s.clone();
            let handle = tokio::spawn(async move {
                let cid = cid;
                let r = p.recv().await;
                let _ = hacky_recv_s.send((cid, r, p)); // ignoring failed
            });
            map.insert(cid, handle);
        };

        let remove_c = |recv_protocols: &mut HashMap<Cid, JoinHandle<()>>, cid: &Cid| {
            match recv_protocols.remove(&cid) {
                Some(h) => {
                    h.abort();
                    debug!(?cid, "remove protocol");
                },
                None => trace!("tried to remove protocol twice"),
            };
            recv_protocols.is_empty()
        };

        loop {
            let (event, addp, remp) = select!(
                Some(n) = hacky_recv_r.recv().fuse() => (Some(n), None, None),
                Some(n) = b2b_add_protocol_r.recv().fuse() => (None, Some(n), None),
                Ok(n) = b2b_force_close_recv_protocol_r.recv().fuse() => (None, None, Some(n)),
                else => {
                    error!("recv_mgr -> something is seriously wrong!, end recv_mgr");
                    break;
                }
            );

            if let Some((cid, p)) = addp {
                debug!(?cid, "add protocol");
                retrigger(cid, p, &mut recv_protocols);
            };
            if let Some(cid) = remp {
                // no need to stop the send_mgr here as it has been canceled before
                if remove_c(&mut recv_protocols, &cid) {
                    break;
                }
            };

            if let Some((cid, r, p)) = event {
                match r {
                    Ok(ProtocolEvent::OpenStream {
                        sid,
                        prio,
                        promises,
                        guaranteed_bandwidth,
                    }) => {
                        trace!(?sid, "open stream");
                        let _ = b2b_notify_send_of_recv_s.send(r.unwrap());
                        // waiting for receiving is not necessary, because the send_mgr will first
                        // process this before process messages!
                        let stream = self
                            .create_stream(
                                sid,
                                prio,
                                promises,
                                guaranteed_bandwidth,
                                &a2b_msg_s,
                                &a2b_close_stream_s,
                            )
                            .await;
                        b2a_stream_opened_s.send(stream).unwrap();
                        retrigger(cid, p, &mut recv_protocols);
                    },
                    Ok(ProtocolEvent::CloseStream { sid }) => {
                        trace!(?sid, "close stream");
                        let _ = b2b_notify_send_of_recv_s.send(r.unwrap());
                        self.delete_stream(sid).await;
                        retrigger(cid, p, &mut recv_protocols);
                    },
                    Ok(ProtocolEvent::Message { data, mid: _, sid }) => {
                        let lock = self.streams.read().await;
                        match lock.get(&sid) {
                            Some(stream) => {
                                let _ = stream.b2a_msg_recv_s.lock().await.send(data).await;
                            },
                            None => warn!("recv a msg with orphan stream"),
                        };
                        retrigger(cid, p, &mut recv_protocols);
                    },
                    Ok(ProtocolEvent::Shutdown) => {
                        info!(?cid, "shutdown protocol");
                        if let Err(e) = b2b_close_send_protocol_s.send(cid).await {
                            debug!(?e, ?cid, "send_mgr was already closed simultaneously");
                        }
                        if remove_c(&mut recv_protocols, &cid) {
                            break;
                        }
                    },
                    Err(e) => {
                        info!(?e, ?cid, "protocol failed, shutting down channel");
                        if let Err(e) = b2b_close_send_protocol_s.send(cid).await {
                            debug!(?e, ?cid, "send_mgr was already closed simultaneously");
                        }
                        if remove_c(&mut recv_protocols, &cid) {
                            break;
                        }
                    },
                }
            }
        }
        trace!("receiving no longer possible, closing all streams");
        for (_, si) in self.streams.write().await.drain() {
            si.send_closed.store(true, Ordering::Relaxed);
            self.metrics.streams_closed(&self.remote_pid_string);
        }
        trace!("Stop recv_mgr");
        self.shutdown_barrier
            .fetch_sub(Self::BARR_RECV, Ordering::Relaxed);
    }

    async fn create_channel_mgr(
        &self,
        s2b_create_channel_r: mpsc::UnboundedReceiver<S2bCreateChannel>,
        b2b_add_send_protocol_s: mpsc::UnboundedSender<(Cid, SendProtocols)>,
        b2b_add_recv_protocol_s: mpsc::UnboundedSender<(Cid, RecvProtocols)>,
    ) {
        let s2b_create_channel_r = UnboundedReceiverStream::new(s2b_create_channel_r);
        s2b_create_channel_r
            .for_each_concurrent(None, |(cid, _, protocol, b2s_create_channel_done_s)| {
                // This channel is now configured, and we are running it in scope of the
                // participant.
                let channels = Arc::clone(&self.channels);
                let b2b_add_send_protocol_s = b2b_add_send_protocol_s.clone();
                let b2b_add_recv_protocol_s = b2b_add_recv_protocol_s.clone();
                async move {
                    let mut lock = channels.write().await;
                    let mut channel_no = lock.len();
                    lock.insert(
                        cid,
                        Mutex::new(ChannelInfo {
                            cid,
                            cid_string: cid.to_string(),
                        }),
                    );
                    drop(lock);
                    let (send, recv) = protocol.split();
                    b2b_add_send_protocol_s.send((cid, send)).unwrap();
                    b2b_add_recv_protocol_s.send((cid, recv)).unwrap();
                    b2s_create_channel_done_s.send(()).unwrap();
                    if channel_no > 5 {
                        debug!(?channel_no, "metrics will overwrite channel #5");
                        channel_no = 5;
                    }
                    self.metrics
                        .channels_connected(&self.remote_pid_string, channel_no, cid);
                }
            })
            .await;
        trace!("Stop create_channel_mgr");
        self.shutdown_barrier
            .fetch_sub(Self::BARR_CHANNEL, Ordering::Relaxed);
    }

    /// sink shutdown:
    ///  Situation AS, AR, BS, BR. A wants to close.
    ///  AS shutdown.
    ///  BR notices shutdown and tries to stops BS. (success)
    ///  BS shutdown
    ///  AR notices shutdown and tries to stop AS. (fails)
    /// For the case where BS didn't get shutdowned, e.g. by a handing situation
    /// on the remote, we have a timeout to also force close AR.
    ///
    /// This fn will:
    ///  - 1. stop api to interact with bparticipant by closing sendmsg and
    /// openstream
    ///  - 2. stop the send_mgr (it will take care of clearing the
    /// queue and finish with a Shutdown)
    ///  - (3). force stop recv after 60
    /// seconds
    ///  - (4). this fn finishes last and afterwards BParticipant
    /// drops
    ///
    /// before calling this fn, make sure `s2b_create_channel` is closed!
    /// If BParticipant kills itself managers stay active till this function is
    /// called by api to get the result status
    async fn participant_shutdown_mgr(
        &self,
        s2b_shutdown_bparticipant_r: oneshot::Receiver<S2bShutdownBparticipant>,
        b2b_close_send_protocol_s: async_channel::Sender<Cid>,
        b2b_force_close_recv_protocol_s: async_channel::Sender<Cid>,
    ) {
        let wait_for_manager = || async {
            let mut sleep = 0.01f64;
            loop {
                let bytes = self.shutdown_barrier.load(Ordering::Relaxed);
                if bytes == 0 {
                    break;
                }
                sleep *= 1.4;
                tokio::time::sleep(Duration::from_secs_f64(sleep)).await;
                if sleep > 0.2 {
                    trace!(?bytes, "wait for mgr to close");
                }
            }
        };

        let (timeout_time, sender) = s2b_shutdown_bparticipant_r.await.unwrap();
        debug!("participant_shutdown_mgr triggered. Closing all streams for send");
        {
            let lock = self.streams.read().await;
            for si in lock.values() {
                si.send_closed.store(true, Ordering::Relaxed);
            }
        }

        let lock = self.channels.read().await;
        assert!(
            !lock.is_empty(),
            "no channel existed remote_pid={}",
            self.remote_pid
        );
        for cid in lock.keys() {
            if let Err(e) = b2b_close_send_protocol_s.send(*cid).await {
                debug!(
                    ?e,
                    ?cid,
                    "closing send_mgr may fail if we got a recv error simultaneously"
                );
            }
        }
        drop(lock);

        trace!("wait for other managers");
        let timeout = tokio::time::sleep(timeout_time);
        let timeout = tokio::select! {
            _ = wait_for_manager() => false,
            _ = timeout => true,
        };
        if timeout {
            warn!("timeout triggered: for killing recv");
            let lock = self.channels.read().await;
            for cid in lock.keys() {
                if let Err(e) = b2b_force_close_recv_protocol_s.send(*cid).await {
                    debug!(
                        ?e,
                        ?cid,
                        "closing recv_mgr may fail if we got a recv error simultaneously"
                    );
                }
            }
        }

        trace!("wait again");
        wait_for_manager().await;

        sender.send(Ok(())).unwrap();

        #[cfg(feature = "metrics")]
        self.metrics.participants_disconnected_total.inc();
        trace!("Stop participant_shutdown_mgr");
    }

    /// Stopping API and participant usage
    /// Protocol will take care of the order of the frame
    async fn delete_stream(&self, sid: Sid) {
        let stream = { self.streams.write().await.remove(&sid) };
        match stream {
            Some(si) => {
                si.send_closed.store(true, Ordering::Relaxed);
                si.b2a_msg_recv_s.lock().await.close();
            },
            None => {
                trace!("Couldn't find the stream, might be simultaneous close from local/remote")
            },
        }
        self.metrics.streams_closed(&self.remote_pid_string);
    }

    async fn create_stream(
        &self,
        sid: Sid,
        prio: Prio,
        promises: Promises,
        guaranteed_bandwidth: Bandwidth,
        a2b_msg_s: &crossbeam_channel::Sender<(Sid, Bytes)>,
        a2b_close_stream_s: &mpsc::UnboundedSender<Sid>,
    ) -> Stream {
        let (b2a_msg_recv_s, b2a_msg_recv_r) = async_channel::unbounded::<Bytes>();
        let send_closed = Arc::new(AtomicBool::new(false));
        self.streams.write().await.insert(sid, StreamInfo {
            prio,
            promises,
            send_closed: Arc::clone(&send_closed),
            b2a_msg_recv_s: Mutex::new(b2a_msg_recv_s),
        });
        self.metrics.streams_opened(&self.remote_pid_string);
        Stream::new(
            self.local_pid,
            self.remote_pid,
            sid,
            prio,
            promises,
            guaranteed_bandwidth,
            send_closed,
            a2b_msg_s.clone(),
            b2a_msg_recv_r,
            a2b_close_stream_s.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_protocol::ProtocolMetrics;
    use tokio::{
        runtime::Runtime,
        sync::{mpsc, oneshot},
        task::JoinHandle,
    };

    #[allow(clippy::type_complexity)]
    fn mock_bparticipant() -> (
        Arc<Runtime>,
        mpsc::UnboundedSender<A2bStreamOpen>,
        mpsc::UnboundedReceiver<Stream>,
        mpsc::UnboundedSender<S2bCreateChannel>,
        oneshot::Sender<S2bShutdownBparticipant>,
        mpsc::UnboundedReceiver<B2sPrioStatistic>,
        JoinHandle<()>,
    ) {
        let runtime = Arc::new(tokio::runtime::Runtime::new().unwrap());
        let runtime_clone = Arc::clone(&runtime);

        let (b2s_prio_statistic_s, b2s_prio_statistic_r) =
            mpsc::unbounded_channel::<B2sPrioStatistic>();

        let (
            bparticipant,
            a2b_open_stream_s,
            b2a_stream_opened_r,
            s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
        ) = runtime_clone.block_on(async move {
            let local_pid = Pid::fake(0);
            let remote_pid = Pid::fake(1);
            let sid = Sid::new(1000);
            let metrics = Arc::new(NetworkMetrics::new(&local_pid).unwrap());

            BParticipant::new(local_pid, remote_pid, sid, Arc::clone(&metrics))
        });

        let handle = runtime_clone.spawn(bparticipant.run(b2s_prio_statistic_s));
        (
            runtime_clone,
            a2b_open_stream_s,
            b2a_stream_opened_r,
            s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
            b2s_prio_statistic_r,
            handle,
        )
    }

    async fn mock_mpsc(
        cid: Cid,
        _runtime: &Arc<Runtime>,
        create_channel: &mut mpsc::UnboundedSender<S2bCreateChannel>,
    ) -> Protocols {
        let (s1, r1) = mpsc::channel(100);
        let (s2, r2) = mpsc::channel(100);
        let metrics = Arc::new(ProtocolMetrics::new().unwrap());
        let p1 = Protocols::new_mpsc(s1, r2, cid, Arc::clone(&metrics));
        let (complete_s, complete_r) = oneshot::channel();
        create_channel
            .send((cid, Sid::new(0), p1, complete_s))
            .unwrap();
        complete_r.await.unwrap();
        Protocols::new_mpsc(s2, r1, cid, Arc::clone(&metrics))
    }

    #[test]
    fn close_bparticipant_by_timeout_during_close() {
        let (
            runtime,
            a2b_open_stream_s,
            b2a_stream_opened_r,
            mut s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
            b2s_prio_statistic_r,
            handle,
        ) = mock_bparticipant();

        let _remote = runtime.block_on(mock_mpsc(0, &runtime, &mut s2b_create_channel_s));
        std::thread::sleep(Duration::from_millis(50));

        let (s, r) = oneshot::channel();
        let before = Instant::now();
        runtime.block_on(async {
            drop(s2b_create_channel_s);
            s2b_shutdown_bparticipant_s
                .send((Duration::from_secs(1), s))
                .unwrap();
            r.await.unwrap().unwrap();
        });
        assert!(
            before.elapsed() > Duration::from_millis(900),
            "timeout wasn't triggered"
        );

        runtime.block_on(handle).unwrap();

        drop((a2b_open_stream_s, b2a_stream_opened_r, b2s_prio_statistic_r));
        drop(runtime);
    }

    #[test]
    fn close_bparticipant_cleanly() {
        let (
            runtime,
            a2b_open_stream_s,
            b2a_stream_opened_r,
            mut s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
            b2s_prio_statistic_r,
            handle,
        ) = mock_bparticipant();

        let remote = runtime.block_on(mock_mpsc(0, &runtime, &mut s2b_create_channel_s));
        std::thread::sleep(Duration::from_millis(50));

        let (s, r) = oneshot::channel();
        let before = Instant::now();
        runtime.block_on(async {
            drop(s2b_create_channel_s);
            s2b_shutdown_bparticipant_s
                .send((Duration::from_secs(2), s))
                .unwrap();
            drop(remote); // remote needs to be dropped as soon as local.sender is closed
            r.await.unwrap().unwrap();
        });
        assert!(
            before.elapsed() < Duration::from_millis(1900),
            "timeout was triggered"
        );

        runtime.block_on(handle).unwrap();

        drop((a2b_open_stream_s, b2a_stream_opened_r, b2s_prio_statistic_r));
        drop(runtime);
    }

    #[test]
    fn create_stream() {
        let (
            runtime,
            a2b_open_stream_s,
            b2a_stream_opened_r,
            mut s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
            b2s_prio_statistic_r,
            handle,
        ) = mock_bparticipant();

        let remote = runtime.block_on(mock_mpsc(0, &runtime, &mut s2b_create_channel_s));
        std::thread::sleep(Duration::from_millis(50));

        // created stream
        let (rs, mut rr) = remote.split();
        let (stream_sender, _stream_receiver) = oneshot::channel();
        a2b_open_stream_s
            .send((7u8, Promises::ENCRYPTED, 1_000_000, stream_sender))
            .unwrap();

        let stream_event = runtime.block_on(rr.recv()).unwrap();
        match stream_event {
            ProtocolEvent::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth,
            } => {
                assert_eq!(sid, Sid::new(1000));
                assert_eq!(prio, 7u8);
                assert_eq!(promises, Promises::ENCRYPTED);
                assert_eq!(guaranteed_bandwidth, 1_000_000);
            },
            _ => panic!("wrong event"),
        };

        let (s, r) = oneshot::channel();
        runtime.block_on(async {
            drop(s2b_create_channel_s);
            s2b_shutdown_bparticipant_s
                .send((Duration::from_secs(1), s))
                .unwrap();
            drop((rs, rr));
            r.await.unwrap().unwrap();
        });

        runtime.block_on(handle).unwrap();

        drop((a2b_open_stream_s, b2a_stream_opened_r, b2s_prio_statistic_r));
        drop(runtime);
    }

    #[test]
    fn created_stream() {
        let (
            runtime,
            a2b_open_stream_s,
            mut b2a_stream_opened_r,
            mut s2b_create_channel_s,
            s2b_shutdown_bparticipant_s,
            b2s_prio_statistic_r,
            handle,
        ) = mock_bparticipant();

        let remote = runtime.block_on(mock_mpsc(0, &runtime, &mut s2b_create_channel_s));
        std::thread::sleep(Duration::from_millis(50));

        // create stream
        let (mut rs, rr) = remote.split();
        runtime
            .block_on(rs.send(ProtocolEvent::OpenStream {
                sid: Sid::new(1000),
                prio: 9u8,
                promises: Promises::ORDERED,
                guaranteed_bandwidth: 1_000_000,
            }))
            .unwrap();

        let stream = runtime.block_on(b2a_stream_opened_r.recv()).unwrap();
        assert_eq!(stream.promises(), Promises::ORDERED);

        let (s, r) = oneshot::channel();
        runtime.block_on(async {
            drop(s2b_create_channel_s);
            s2b_shutdown_bparticipant_s
                .send((Duration::from_secs(1), s))
                .unwrap();
            drop((rs, rr));
            r.await.unwrap().unwrap();
        });

        runtime.block_on(handle).unwrap();

        drop((a2b_open_stream_s, b2a_stream_opened_r, b2s_prio_statistic_r));
        drop(runtime);
    }
}
