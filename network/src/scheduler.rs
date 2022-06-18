use crate::{
    api::{ConnectAddr, ListenAddr, NetworkConnectError, Participant},
    channel::Protocols,
    metrics::{NetworkMetrics, ProtocolInfo},
    participant::{B2sPrioStatistic, BParticipant, S2bCreateChannel, S2bShutdownBparticipant},
};
use futures_util::StreamExt;
use hashbrown::HashMap;
use network_protocol::{Cid, Pid, ProtocolMetricCache, ProtocolMetrics};
#[cfg(feature = "metrics")]
use prometheus::Registry;
use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io,
    sync::{mpsc, oneshot, Mutex},
};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::*;

// Naming of Channels `x2x`
//  - a: api
//  - s: scheduler
//  - b: bparticipant
//  - p: prios
//  - r: protocol
//  - w: wire
//  - c: channel/handshake

#[derive(Debug)]
struct ParticipantInfo {
    secret: u128,
    #[allow(dead_code)]
    s2b_create_channel_s: mpsc::UnboundedSender<S2bCreateChannel>,
    s2b_shutdown_bparticipant_s: Option<oneshot::Sender<S2bShutdownBparticipant>>,
}

type A2sListen = (ListenAddr, oneshot::Sender<io::Result<()>>);
pub(crate) type A2sConnect = (
    ConnectAddr,
    oneshot::Sender<Result<Participant, NetworkConnectError>>,
);
type A2sDisconnect = (Pid, S2bShutdownBparticipant);

#[derive(Debug)]
struct ControlChannels {
    a2s_listen_r: mpsc::UnboundedReceiver<A2sListen>,
    a2s_connect_r: mpsc::UnboundedReceiver<A2sConnect>,
    a2s_scheduler_shutdown_r: oneshot::Receiver<()>,
    a2s_disconnect_r: mpsc::UnboundedReceiver<A2sDisconnect>,
    b2s_prio_statistic_r: mpsc::UnboundedReceiver<B2sPrioStatistic>,
}

#[derive(Debug, Clone)]
struct ParticipantChannels {
    s2a_connected_s: mpsc::UnboundedSender<Participant>,
    a2s_disconnect_s: mpsc::UnboundedSender<A2sDisconnect>,
    b2s_prio_statistic_s: mpsc::UnboundedSender<B2sPrioStatistic>,
}

#[derive(Debug)]
pub struct Scheduler {
    local_pid: Pid,
    local_secret: u128,
    closed: AtomicBool,
    run_channels: Option<ControlChannels>,
    participant_channels: Arc<Mutex<Option<ParticipantChannels>>>,
    participants: Arc<Mutex<HashMap<Pid, ParticipantInfo>>>,
    channel_ids: Arc<AtomicU64>,
    channel_listener: Mutex<HashMap<ProtocolInfo, oneshot::Sender<()>>>,
    metrics: Arc<NetworkMetrics>,
    protocol_metrics: Arc<ProtocolMetrics>,
}

impl Scheduler {
    pub fn new(
        local_pid: Pid,
        #[cfg(feature = "metrics")] registry: Option<&Registry>,
    ) -> (
        Self,
        mpsc::UnboundedSender<A2sListen>,
        mpsc::UnboundedSender<A2sConnect>,
        mpsc::UnboundedReceiver<Participant>,
        oneshot::Sender<()>,
    ) {
        let (a2s_listen_s, a2s_listen_r) = mpsc::unbounded_channel::<A2sListen>();
        let (a2s_connect_s, a2s_connect_r) = mpsc::unbounded_channel::<A2sConnect>();
        let (s2a_connected_s, s2a_connected_r) = mpsc::unbounded_channel::<Participant>();
        let (a2s_scheduler_shutdown_s, a2s_scheduler_shutdown_r) = oneshot::channel::<()>();
        let (a2s_disconnect_s, a2s_disconnect_r) = mpsc::unbounded_channel::<A2sDisconnect>();
        let (b2s_prio_statistic_s, b2s_prio_statistic_r) =
            mpsc::unbounded_channel::<B2sPrioStatistic>();

        let run_channels = Some(ControlChannels {
            a2s_listen_r,
            a2s_connect_r,
            a2s_scheduler_shutdown_r,
            a2s_disconnect_r,
            b2s_prio_statistic_r,
        });

        let participant_channels = ParticipantChannels {
            s2a_connected_s,
            a2s_disconnect_s,
            b2s_prio_statistic_s,
        };

        let metrics = Arc::new(NetworkMetrics::new(&local_pid).unwrap());
        let protocol_metrics = Arc::new(ProtocolMetrics::new().unwrap());

        #[cfg(feature = "metrics")]
        {
            if let Some(registry) = registry {
                metrics.register(registry).unwrap();
                protocol_metrics.register(registry).unwrap();
            }
        }

        let mut rng = rand::thread_rng();
        let local_secret: u128 = rng.gen();

        (
            Self {
                local_pid,
                local_secret,
                closed: AtomicBool::new(false),
                run_channels,
                participant_channels: Arc::new(Mutex::new(Some(participant_channels))),
                participants: Arc::new(Mutex::new(HashMap::new())),
                channel_ids: Arc::new(AtomicU64::new(0)),
                channel_listener: Mutex::new(HashMap::new()),
                metrics,
                protocol_metrics,
            },
            a2s_listen_s,
            a2s_connect_s,
            s2a_connected_r,
            a2s_scheduler_shutdown_s,
        )
    }

    pub async fn run(mut self) {
        let run_channels = self
            .run_channels
            .take()
            .expect("run() can only be called once");

        tokio::join!(
            self.listen_mgr(run_channels.a2s_listen_r),
            self.connect_mgr(run_channels.a2s_connect_r),
            self.disconnect_mgr(run_channels.a2s_disconnect_r),
            self.prio_adj_mgr(run_channels.b2s_prio_statistic_r),
            self.scheduler_shutdown_mgr(run_channels.a2s_scheduler_shutdown_r),
        );
    }

    async fn listen_mgr(&self, a2s_listen_r: mpsc::UnboundedReceiver<A2sListen>) {
        trace!("Start listen_mgr");
        let a2s_listen_r = UnboundedReceiverStream::new(a2s_listen_r);
        a2s_listen_r
            .for_each_concurrent(None, |(address, s2a_listen_result_s)| {
                let address = address;
                let cids = Arc::clone(&self.channel_ids);

                #[cfg(feature = "metrics")]
                let mcache = self.metrics.connect_requests_cache(&address);

                debug!(?address, "Got request to open a channel_creator");
                self.metrics.listen_request(&address);
                let (s2s_stop_listening_s, s2s_stop_listening_r) = oneshot::channel::<()>();
                let (c2s_protocol_s, mut c2s_protocol_r) = mpsc::unbounded_channel();
                let metrics = Arc::clone(&self.protocol_metrics);

                async move {
                    self.channel_listener
                        .lock()
                        .await
                        .insert(address.clone().into(), s2s_stop_listening_s);

                    #[cfg(feature = "metrics")]
                    mcache.inc();

                    let res = match address {
                        ListenAddr::Tcp(addr) => {
                            Protocols::with_tcp_listen(
                                addr,
                                cids,
                                metrics,
                                s2s_stop_listening_r,
                                c2s_protocol_s,
                            )
                            .await
                        },
                        #[cfg(feature = "quic")]
                        ListenAddr::Quic(addr, ref server_config) => {
                            Protocols::with_quic_listen(
                                addr,
                                server_config.clone(),
                                cids,
                                metrics,
                                s2s_stop_listening_r,
                                c2s_protocol_s,
                            )
                            .await
                        },
                        ListenAddr::Mpsc(addr) => {
                            Protocols::with_mpsc_listen(
                                addr,
                                cids,
                                metrics,
                                s2s_stop_listening_r,
                                c2s_protocol_s,
                            )
                            .await
                        },
                        _ => unimplemented!(),
                    };
                    let _ = s2a_listen_result_s.send(res);

                    while let Some((prot, con_addr, cid)) = c2s_protocol_r.recv().await {
                        self.init_protocol(prot, con_addr, cid, None, true).await;
                    }
                }
            })
            .await;
        trace!("Stop listen_mgr");
    }

    async fn connect_mgr(&self, mut a2s_connect_r: mpsc::UnboundedReceiver<A2sConnect>) {
        trace!("Start connect_mgr");
        while let Some((addr, pid_sender)) = a2s_connect_r.recv().await {
            let cid = self.channel_ids.fetch_add(1, Ordering::Relaxed);
            let metrics =
                ProtocolMetricCache::new(&cid.to_string(), Arc::clone(&self.protocol_metrics));
            self.metrics.connect_request(&addr);
            let protocol = match addr.clone() {
                ConnectAddr::Tcp(addr) => Protocols::with_tcp_connect(addr, metrics).await,
                #[cfg(feature = "quic")]
                ConnectAddr::Quic(addr, ref config, name) => {
                    Protocols::with_quic_connect(addr, config.clone(), name, metrics).await
                },
                ConnectAddr::Mpsc(addr) => Protocols::with_mpsc_connect(addr, metrics).await,
                _ => unimplemented!(),
            };
            let protocol = match protocol {
                Ok(p) => p,
                Err(e) => {
                    pid_sender.send(Err(e)).unwrap();
                    continue;
                },
            };
            self.init_protocol(protocol, addr, cid, Some(pid_sender), false)
                .await;
        }
        trace!("Stop connect_mgr");
    }

    async fn disconnect_mgr(&self, a2s_disconnect_r: mpsc::UnboundedReceiver<A2sDisconnect>) {
        trace!("Start disconnect_mgr");

        let a2s_disconnect_r = UnboundedReceiverStream::new(a2s_disconnect_r);
        a2s_disconnect_r
            .for_each_concurrent(
                None,
                |(pid, (timeout_time, return_once_successful_shutdown))| {
                    //Closing Participants is done the following way:
                    // 1. We drop our senders and receivers
                    // 2. we need to close BParticipant, this will drop its senderns and receivers
                    // 3. Participant will try to access the BParticipant senders and receivers with
                    // their next api action, it will fail and be closed then.
                    let participants = Arc::clone(&self.participants);
                    async move {
                        trace!(?pid, "Got request to close participant");
                        let pi = participants.lock().await.remove(&pid);
                        trace!(?pid, "dropped participants lock");
                        let r = if let Some(mut pi) = pi {
                            let (finished_sender, finished_receiver) = oneshot::channel();
                            // NOTE: If there's nothing to synchronize on (because the send failed)
                            // we can assume everything relevant was shut down.
                            let _ = pi
                                .s2b_shutdown_bparticipant_s
                                .take()
                                .unwrap()
                                .send((timeout_time, finished_sender));
                            drop(pi);
                            trace!(?pid, "dropped bparticipant, waiting for finish");
                            // If await fails, already shut down, so send Ok(()).
                            let e = finished_receiver.await.unwrap_or(Ok(()));
                            trace!(?pid, "waiting completed");
                            // can fail as api.rs has a timeout
                            return_once_successful_shutdown.send(e)
                        } else {
                            debug!(?pid, "Looks like participant is already dropped");
                            return_once_successful_shutdown.send(Ok(()))
                        };
                        if r.is_err() {
                            trace!(?pid, "Closed participant with timeout");
                        } else {
                            trace!(?pid, "Closed participant");
                        }
                    }
                },
            )
            .await;
        trace!("Stop disconnect_mgr");
    }

    async fn prio_adj_mgr(
        &self,
        mut b2s_prio_statistic_r: mpsc::UnboundedReceiver<B2sPrioStatistic>,
    ) {
        trace!("Start prio_adj_mgr");
        while let Some((_pid, _frame_cnt, _unused)) = b2s_prio_statistic_r.recv().await {

            //TODO adjust prios in participants here!
        }
        trace!("Stop prio_adj_mgr");
    }

    async fn scheduler_shutdown_mgr(&self, a2s_scheduler_shutdown_r: oneshot::Receiver<()>) {
        trace!("Start scheduler_shutdown_mgr");
        if a2s_scheduler_shutdown_r.await.is_err() {
            warn!("Schedule shutdown got triggered because a2s_scheduler_shutdown_r failed");
        };
        info!("Shutdown of scheduler requested");
        self.closed.store(true, Ordering::SeqCst);
        debug!("Shutting down all BParticipants gracefully");
        let mut participants = self.participants.lock().await;
        let waitings = participants
            .drain()
            .map(|(pid, mut pi)| {
                trace!(?pid, "Shutting down BParticipants");
                let (finished_sender, finished_receiver) = oneshot::channel();
                pi.s2b_shutdown_bparticipant_s
                    .take()
                    .unwrap()
                    .send((Duration::from_secs(120), finished_sender))
                    .unwrap();
                (pid, finished_receiver)
            })
            .collect::<Vec<_>>();
        drop(participants);
        debug!("Wait for partiticipants to be shut down");
        for (pid, recv) in waitings {
            if let Err(e) = recv.await {
                error!(
                    ?pid,
                    ?e,
                    "Failed to finish sending all remaining messages to participant when shutting \
                     down"
                );
            };
        }
        debug!("shutting down protocol listeners");
        for (addr, end_channel_sender) in self.channel_listener.lock().await.drain() {
            trace!(?addr, "stopping listen on protocol");
            if let Err(e) = end_channel_sender.send(()) {
                warn!(?addr, ?e, "listener crashed/disconnected already");
            }
        }
        debug!("Scheduler shut down gracefully");
        //removing the possibility to create new participants, needed to close down
        // some mgr:
        self.participant_channels.lock().await.take();

        trace!("Stop scheduler_shutdown_mgr");
    }

    async fn init_protocol(
        &self,
        mut protocol: Protocols,
        con_addr: ConnectAddr, //address necessary to connect to the remote
        cid: Cid,
        s2a_return_pid_s: Option<oneshot::Sender<Result<Participant, NetworkConnectError>>>,
        send_handshake: bool,
    ) {
        //channels are unknown till PID is known!
        /* When A connects to a NETWORK, we, the listener answers with a Handshake.
          Pro: - Its easier to debug, as someone who opens a port gets a magic number back!
          Contra: - DOS possibility because we answer first
                  - Speed, because otherwise the message can be send with the creation
        */
        let participant_channels = self.participant_channels.lock().await.clone().unwrap();
        // spawn is needed here, e.g. for TCP connect it would mean that only 1
        // participant can be in handshake phase ever! Someone could deadlock
        // the whole server easily for new clients UDP doesnt work at all, as
        // the UDP listening is done in another place.
        let participants = Arc::clone(&self.participants);
        let metrics = Arc::clone(&self.metrics);
        let local_pid = self.local_pid;
        let local_secret = self.local_secret;
        // this is necessary for UDP to work at all and to remove code duplication
        tokio::spawn(
            async move {
                trace!(?cid, "Open channel and be ready for Handshake");
                use network_protocol::InitProtocol;
                let init_result = protocol
                    .initialize(send_handshake, local_pid, local_secret)
                    .instrument(info_span!("handshake", ?cid))
                    .await;
                match init_result {
                    Ok((pid, sid, secret)) => {
                        trace!(
                            ?cid,
                            ?pid,
                            "Detected that my channel is ready!, activating it :)"
                        );
                        let mut participants = participants.lock().await;
                        if !participants.contains_key(&pid) {
                            debug!(?cid, "New participant connected via a channel");
                            let (
                                bparticipant,
                                a2b_open_stream_s,
                                b2a_stream_opened_r,
                                b2a_event_r,
                                s2b_create_channel_s,
                                s2b_shutdown_bparticipant_s,
                                b2a_bandwidth_stats_r,
                            ) = BParticipant::new(local_pid, pid, sid, Arc::clone(&metrics));

                            let participant = Participant::new(
                                local_pid,
                                pid,
                                a2b_open_stream_s,
                                b2a_stream_opened_r,
                                b2a_event_r,
                                b2a_bandwidth_stats_r,
                                participant_channels.a2s_disconnect_s,
                            );

                            #[cfg(feature = "metrics")]
                            metrics.participants_connected_total.inc();
                            participants.insert(pid, ParticipantInfo {
                                secret,
                                s2b_create_channel_s: s2b_create_channel_s.clone(),
                                s2b_shutdown_bparticipant_s: Some(s2b_shutdown_bparticipant_s),
                            });
                            drop(participants);
                            trace!("dropped participants lock");
                            let p = pid;
                            tokio::spawn(
                                bparticipant
                                    .run(participant_channels.b2s_prio_statistic_s)
                                    .instrument(info_span!("remote", ?p)),
                            );
                            //create a new channel within BParticipant and wait for it to run
                            let (b2s_create_channel_done_s, b2s_create_channel_done_r) =
                                oneshot::channel();
                            //From now on wire connects directly with bparticipant!
                            s2b_create_channel_s
                                .send((cid, sid, protocol, con_addr, b2s_create_channel_done_s))
                                .unwrap();
                            b2s_create_channel_done_r.await.unwrap();
                            if let Some(pid_oneshot) = s2a_return_pid_s {
                                // someone is waiting with `connect`, so give them their PID
                                pid_oneshot.send(Ok(participant)).unwrap();
                            } else {
                                // no one is waiting on this Participant, return in to Network
                                if participant_channels
                                    .s2a_connected_s
                                    .send(participant)
                                    .is_err()
                                {
                                    warn!("seems like Network already got closed");
                                };
                            }
                        } else {
                            let pi = &participants[&pid];
                            trace!(
                                ?cid,
                                "2nd+ channel of participant, going to compare security ids"
                            );
                            if pi.secret != secret {
                                warn!(
                                    ?cid,
                                    ?pid,
                                    ?secret,
                                    "Detected incompatible Secret!, this is probably an attack!"
                                );
                                error!(?cid, "Just dropping here, TODO handle this correctly!");
                                //TODO
                                if let Some(pid_oneshot) = s2a_return_pid_s {
                                    // someone is waiting with `connect`, so give them their Error
                                    pid_oneshot
                                        .send(Err(NetworkConnectError::InvalidSecret))
                                        .unwrap();
                                }
                                return;
                            }
                            error!(
                                ?cid,
                                "Ufff i cant answer the pid_oneshot. as i need to create the SAME \
                                 participant. maybe switch to ARC"
                            );
                        }
                        //From now on this CHANNEL can receiver other frames!
                        // move directly to participant!
                    },
                    Err(e) => {
                        debug!(?cid, ?e, "Handshake from a new connection failed");
                        #[cfg(feature = "metrics")]
                        metrics.failed_handshakes_total.inc();
                        if let Some(pid_oneshot) = s2a_return_pid_s {
                            // someone is waiting with `connect`, so give them their Error
                            trace!(?cid, "returning the Err to api who requested the connect");
                            pid_oneshot
                                .send(Err(NetworkConnectError::Handshake(e)))
                                .unwrap();
                        }
                    },
                }
            }
            .instrument(info_span!("")),
        ); /*WORKAROUND FOR SPAN NOT TO GET LOST*/
    }
}
