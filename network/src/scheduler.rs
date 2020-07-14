use crate::{
    api::{Participant, ProtocolAddr},
    channel::Handshake,
    metrics::NetworkMetrics,
    participant::{B2sPrioStatistic, BParticipant, S2bCreateChannel},
    protocols::{Protocols, TcpProtocol, UdpProtocol},
    types::Pid,
};
use async_std::{
    io, net,
    sync::{Mutex, RwLock},
};
use futures::{
    channel::{mpsc, oneshot},
    executor::ThreadPool,
    future::FutureExt,
    select,
    sink::SinkExt,
    stream::StreamExt,
};
use prometheus::Registry;
use rand::Rng;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use tracing::*;
use tracing_futures::Instrument;

/// Naming of Channels `x2x`
///  - a: api
///  - s: scheduler
///  - b: bparticipant
///  - p: prios
///  - r: protocol
///  - w: wire
///  - c: channel/handshake

#[derive(Debug)]
struct ParticipantInfo {
    secret: u128,
    s2b_create_channel_s: mpsc::UnboundedSender<S2bCreateChannel>,
    s2b_shutdown_bparticipant_s:
        Option<oneshot::Sender<oneshot::Sender<async_std::io::Result<()>>>>,
}

type A2sListen = (ProtocolAddr, oneshot::Sender<io::Result<()>>);
type A2sConnect = (ProtocolAddr, oneshot::Sender<io::Result<Participant>>);
type A2sDisconnect = (Pid, oneshot::Sender<async_std::io::Result<()>>);

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
    pool: Arc<ThreadPool>,
    run_channels: Option<ControlChannels>,
    participant_channels: Arc<Mutex<Option<ParticipantChannels>>>,
    participants: Arc<RwLock<HashMap<Pid, ParticipantInfo>>>,
    channel_ids: Arc<AtomicU64>,
    channel_listener: RwLock<HashMap<ProtocolAddr, oneshot::Sender<()>>>,
    metrics: Arc<NetworkMetrics>,
}

impl Scheduler {
    pub fn new(
        local_pid: Pid,
        registry: Option<&Registry>,
    ) -> (
        Self,
        mpsc::UnboundedSender<A2sListen>,
        mpsc::UnboundedSender<A2sConnect>,
        mpsc::UnboundedReceiver<Participant>,
        oneshot::Sender<()>,
    ) {
        let (a2s_listen_s, a2s_listen_r) = mpsc::unbounded::<A2sListen>();
        let (a2s_connect_s, a2s_connect_r) = mpsc::unbounded::<A2sConnect>();
        let (s2a_connected_s, s2a_connected_r) = mpsc::unbounded::<Participant>();
        let (a2s_scheduler_shutdown_s, a2s_scheduler_shutdown_r) = oneshot::channel::<()>();
        let (a2s_disconnect_s, a2s_disconnect_r) = mpsc::unbounded::<A2sDisconnect>();
        let (b2s_prio_statistic_s, b2s_prio_statistic_r) = mpsc::unbounded::<B2sPrioStatistic>();

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
        if let Some(registry) = registry {
            metrics.register(registry).unwrap();
        }

        let mut rng = rand::thread_rng();
        let local_secret: u128 = rng.gen();

        (
            Self {
                local_pid,
                local_secret,
                closed: AtomicBool::new(false),
                pool: Arc::new(ThreadPool::new().unwrap()),
                run_channels,
                participant_channels: Arc::new(Mutex::new(Some(participant_channels))),
                participants: Arc::new(RwLock::new(HashMap::new())),
                channel_ids: Arc::new(AtomicU64::new(0)),
                channel_listener: RwLock::new(HashMap::new()),
                metrics,
            },
            a2s_listen_s,
            a2s_connect_s,
            s2a_connected_r,
            a2s_scheduler_shutdown_s,
        )
    }

    pub async fn run(mut self) {
        let run_channels = self.run_channels.take().unwrap();

        futures::join!(
            self.listen_mgr(run_channels.a2s_listen_r),
            self.connect_mgr(run_channels.a2s_connect_r),
            self.disconnect_mgr(run_channels.a2s_disconnect_r),
            self.prio_adj_mgr(run_channels.b2s_prio_statistic_r),
            self.scheduler_shutdown_mgr(run_channels.a2s_scheduler_shutdown_r),
        );
    }

    async fn listen_mgr(&self, a2s_listen_r: mpsc::UnboundedReceiver<A2sListen>) {
        trace!("Start listen_mgr");
        a2s_listen_r
            .for_each_concurrent(None, |(address, s2a_listen_result_s)| {
                let address = address;

                async move {
                    debug!(?address, "Got request to open a channel_creator");
                    self.metrics
                        .listen_requests_total
                        .with_label_values(&[match address {
                            ProtocolAddr::Tcp(_) => "tcp",
                            ProtocolAddr::Udp(_) => "udp",
                            ProtocolAddr::Mpsc(_) => "mpsc",
                        }])
                        .inc();
                    let (end_sender, end_receiver) = oneshot::channel::<()>();
                    self.channel_listener
                        .write()
                        .await
                        .insert(address.clone(), end_sender);
                    self.channel_creator(address, end_receiver, s2a_listen_result_s)
                        .await;
                }
            })
            .await;
        trace!("Stop listen_mgr");
    }

    async fn connect_mgr(
        &self,
        mut a2s_connect_r: mpsc::UnboundedReceiver<(
            ProtocolAddr,
            oneshot::Sender<io::Result<Participant>>,
        )>,
    ) {
        trace!("Start connect_mgr");
        while let Some((addr, pid_sender)) = a2s_connect_r.next().await {
            let (protocol, handshake) = match addr {
                ProtocolAddr::Tcp(addr) => {
                    self.metrics
                        .connect_requests_total
                        .with_label_values(&["tcp"])
                        .inc();
                    let stream = match net::TcpStream::connect(addr).await {
                        Ok(stream) => stream,
                        Err(e) => {
                            pid_sender.send(Err(e)).unwrap();
                            continue;
                        },
                    };
                    info!("Connecting Tcp to: {}", stream.peer_addr().unwrap());
                    (
                        Protocols::Tcp(TcpProtocol::new(stream, self.metrics.clone())),
                        false,
                    )
                },
                ProtocolAddr::Udp(addr) => {
                    self.metrics
                        .connect_requests_total
                        .with_label_values(&["udp"])
                        .inc();
                    let socket = match net::UdpSocket::bind("0.0.0.0:0").await {
                        Ok(socket) => Arc::new(socket),
                        Err(e) => {
                            pid_sender.send(Err(e)).unwrap();
                            continue;
                        },
                    };
                    if let Err(e) = socket.connect(addr).await {
                        pid_sender.send(Err(e)).unwrap();
                        continue;
                    };
                    info!("Connecting Udp to: {}", addr);
                    let (udp_data_sender, udp_data_receiver) = mpsc::unbounded::<Vec<u8>>();
                    let protocol = UdpProtocol::new(
                        socket.clone(),
                        addr,
                        self.metrics.clone(),
                        udp_data_receiver,
                    );
                    self.pool.spawn_ok(
                        Self::udp_single_channel_connect(socket.clone(), udp_data_sender)
                            .instrument(tracing::info_span!("udp", ?addr)),
                    );
                    (Protocols::Udp(protocol), true)
                },
                _ => unimplemented!(),
            };
            self.init_protocol(protocol, Some(pid_sender), handshake)
                .await;
        }
        trace!("Stop connect_mgr");
    }

    async fn disconnect_mgr(&self, mut a2s_disconnect_r: mpsc::UnboundedReceiver<A2sDisconnect>) {
        trace!("Start disconnect_mgr");
        while let Some((pid, return_once_successful_shutdown)) = a2s_disconnect_r.next().await {
            //Closing Participants is done the following way:
            // 1. We drop our senders and receivers
            // 2. we need to close BParticipant, this will drop its senderns and receivers
            // 3. Participant will try to access the BParticipant senders and receivers with
            // their next api action, it will fail and be closed then.
            trace!(?pid, "Got request to close participant");
            if let Some(mut pi) = self.participants.write().await.remove(&pid) {
                let (finished_sender, finished_receiver) = oneshot::channel();
                pi.s2b_shutdown_bparticipant_s
                    .take()
                    .unwrap()
                    .send(finished_sender)
                    .unwrap();
                drop(pi);
                let e = finished_receiver.await.unwrap();
                return_once_successful_shutdown.send(e).unwrap();
            } else {
                debug!(?pid, "Looks like participant is already dropped");
                return_once_successful_shutdown.send(Ok(())).unwrap();
            }
            trace!(?pid, "Closed participant");
        }
        trace!("Stop disconnect_mgr");
    }

    async fn prio_adj_mgr(
        &self,
        mut b2s_prio_statistic_r: mpsc::UnboundedReceiver<B2sPrioStatistic>,
    ) {
        trace!("Start prio_adj_mgr");
        while let Some((_pid, _frame_cnt, _unused)) = b2s_prio_statistic_r.next().await {

            //TODO adjust prios in participants here!
        }
        trace!("Stop prio_adj_mgr");
    }

    async fn scheduler_shutdown_mgr(&self, a2s_scheduler_shutdown_r: oneshot::Receiver<()>) {
        trace!("Start scheduler_shutdown_mgr");
        a2s_scheduler_shutdown_r.await.unwrap();
        self.closed.store(true, Ordering::Relaxed);
        debug!("Shutting down all BParticipants gracefully");
        let mut participants = self.participants.write().await;
        let waitings = participants
            .drain()
            .map(|(pid, mut pi)| {
                trace!(?pid, "Shutting down BParticipants");
                let (finished_sender, finished_receiver) = oneshot::channel();
                pi.s2b_shutdown_bparticipant_s
                    .take()
                    .unwrap()
                    .send(finished_sender)
                    .unwrap();
                (pid, finished_receiver)
            })
            .collect::<Vec<_>>();
        debug!("Wait for partiticipants to be shut down");
        for (pid, recv) in waitings {
            if let Err(e) = recv.await {
                error!(
                    ?pid,
                    ?e,
                    "Failed to finish sending all remainding messages to participant when \
                     shutting down"
                );
            };
        }
        debug!("Scheduler shut down gracefully");
        //removing the possibility to create new participants, needed to close down
        // some mgr:
        self.participant_channels.lock().await.take();

        trace!("Stop scheduler_shutdown_mgr");
    }

    async fn channel_creator(
        &self,
        addr: ProtocolAddr,
        s2s_stop_listening_r: oneshot::Receiver<()>,
        s2a_listen_result_s: oneshot::Sender<io::Result<()>>,
    ) {
        trace!(?addr, "Start up channel creator");
        match addr {
            ProtocolAddr::Tcp(addr) => {
                let listener = match net::TcpListener::bind(addr).await {
                    Ok(listener) => {
                        s2a_listen_result_s.send(Ok(())).unwrap();
                        listener
                    },
                    Err(e) => {
                        info!(
                            ?addr,
                            ?e,
                            "Listener couldn't be started due to error on tcp bind"
                        );
                        s2a_listen_result_s.send(Err(e)).unwrap();
                        return;
                    },
                };
                trace!(?addr, "Listener bound");
                let mut incoming = listener.incoming();
                let mut end_receiver = s2s_stop_listening_r.fuse();
                while let Some(stream) = select! {
                    next = incoming.next().fuse() => next,
                    _ = end_receiver => None,
                } {
                    let stream = match stream {
                        Ok(s) => s,
                        Err(e) => {
                            warn!(?e, "TcpStream Error, ignoring connection attempt");
                            continue;
                        },
                    };
                    let peer_addr = match stream.peer_addr() {
                        Ok(s) => s,
                        Err(e) => {
                            warn!(?e, "TcpStream Error, ignoring connection attempt");
                            continue;
                        },
                    };
                    info!("Accepting Tcp from: {}", peer_addr);
                    let protocol = TcpProtocol::new(stream, self.metrics.clone());
                    self.init_protocol(Protocols::Tcp(protocol), None, true)
                        .await;
                }
            },
            ProtocolAddr::Udp(addr) => {
                let socket = match net::UdpSocket::bind(addr).await {
                    Ok(socket) => {
                        s2a_listen_result_s.send(Ok(())).unwrap();
                        Arc::new(socket)
                    },
                    Err(e) => {
                        info!(
                            ?addr,
                            ?e,
                            "Listener couldn't be started due to error on udp bind"
                        );
                        s2a_listen_result_s.send(Err(e)).unwrap();
                        return;
                    },
                };
                trace!(?addr, "Listener bound");
                // receiving is done from here and will be piped to protocol as UDP does not
                // have any state
                let mut listeners = HashMap::new();
                let mut end_receiver = s2s_stop_listening_r.fuse();
                const UDP_MAXIMUM_SINGLE_PACKET_SIZE_EVER: usize = 9216;
                let mut data = [0u8; UDP_MAXIMUM_SINGLE_PACKET_SIZE_EVER];
                while let Ok((size, remote_addr)) = select! {
                    next = socket.recv_from(&mut data).fuse() => next,
                    _ = end_receiver => Err(std::io::Error::new(std::io::ErrorKind::Other, "")),
                } {
                    let mut datavec = Vec::with_capacity(size);
                    datavec.extend_from_slice(&data[0..size]);
                    //Due to the async nature i cannot make of .entry() as it would lead to a still
                    // borrowed in another branch situation
                    #[allow(clippy::map_entry)]
                    if !listeners.contains_key(&remote_addr) {
                        info!("Accepting Udp from: {}", &remote_addr);
                        let (udp_data_sender, udp_data_receiver) = mpsc::unbounded::<Vec<u8>>();
                        listeners.insert(remote_addr, udp_data_sender);
                        let protocol = UdpProtocol::new(
                            socket.clone(),
                            remote_addr,
                            self.metrics.clone(),
                            udp_data_receiver,
                        );
                        self.init_protocol(Protocols::Udp(protocol), None, false)
                            .await;
                    }
                    let udp_data_sender = listeners.get_mut(&remote_addr).unwrap();
                    udp_data_sender.send(datavec).await.unwrap();
                }
            },
            _ => unimplemented!(),
        }
        trace!(?addr, "Ending channel creator");
    }

    async fn udp_single_channel_connect(
        socket: Arc<net::UdpSocket>,
        mut w2p_udp_package_s: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        let addr = socket.local_addr();
        trace!(?addr, "Start udp_single_channel_connect");
        //TODO: implement real closing
        let (_end_sender, end_receiver) = oneshot::channel::<()>();

        // receiving is done from here and will be piped to protocol as UDP does not
        // have any state
        let mut end_receiver = end_receiver.fuse();
        let mut data = [0u8; 9216];
        while let Ok(size) = select! {
            next = socket.recv(&mut data).fuse() => next,
            _ = end_receiver => Err(std::io::Error::new(std::io::ErrorKind::Other, "")),
        } {
            let mut datavec = Vec::with_capacity(size);
            datavec.extend_from_slice(&data[0..size]);
            w2p_udp_package_s.send(datavec).await.unwrap();
        }
        trace!(?addr, "Stop udp_single_channel_connect");
    }

    async fn init_protocol(
        &self,
        protocol: Protocols,
        s2a_return_pid_s: Option<oneshot::Sender<io::Result<Participant>>>,
        send_handshake: bool,
    ) {
        //channels are unknown till PID is known!
        /* When A connects to a NETWORK, we, the listener answers with a Handshake.
          Pro: - Its easier to debug, as someone who opens a port gets a magic number back!
          Contra: - DOS posibility because we answer fist
                  - Speed, because otherwise the message can be send with the creation
        */
        let mut participant_channels = self.participant_channels.lock().await.clone().unwrap();
        // spawn is needed here, e.g. for TCP connect it would mean that only 1
        // participant can be in handshake phase ever! Someone could deadlock
        // the whole server easily for new clients UDP doesnt work at all, as
        // the UDP listening is done in another place.
        let cid = self.channel_ids.fetch_add(1, Ordering::Relaxed);
        let participants = self.participants.clone();
        let metrics = self.metrics.clone();
        let pool = self.pool.clone();
        let local_pid = self.local_pid;
        let local_secret = self.local_secret;
        // this is necessary for UDP to work at all and to remove code duplication
        self.pool.spawn_ok(
            async move {
                trace!(?cid, "Open channel and be ready for Handshake");
                let handshake = Handshake::new(
                    cid,
                    local_pid,
                    local_secret,
                    metrics.clone(),
                    send_handshake,
                );
                match handshake.setup(&protocol).await {
                    Ok((pid, sid, secret, leftover_cid_frame)) => {
                        trace!(
                            ?cid,
                            ?pid,
                            "Detected that my channel is ready!, activating it :)"
                        );
                        let mut participants = participants.write().await;
                        if !participants.contains_key(&pid) {
                            debug!(?cid, "New participant connected via a channel");
                            let (
                                bparticipant,
                                a2b_stream_open_s,
                                b2a_stream_opened_r,
                                mut s2b_create_channel_s,
                                s2b_shutdown_bparticipant_s,
                                api_participant_closed,
                            ) = BParticipant::new(pid, sid, metrics.clone());

                            let participant = Participant::new(
                                local_pid,
                                pid,
                                a2b_stream_open_s,
                                b2a_stream_opened_r,
                                participant_channels.a2s_disconnect_s,
                                api_participant_closed,
                            );

                            metrics.participants_connected_total.inc();
                            participants.insert(pid, ParticipantInfo {
                                secret,
                                s2b_create_channel_s: s2b_create_channel_s.clone(),
                                s2b_shutdown_bparticipant_s: Some(s2b_shutdown_bparticipant_s),
                            });
                            pool.spawn_ok(
                                bparticipant
                                    .run(participant_channels.b2s_prio_statistic_s)
                                    .instrument(tracing::info_span!("participant", ?pid)),
                            );
                            //create a new channel within BParticipant and wait for it to run
                            let (b2s_create_channel_done_s, b2s_create_channel_done_r) =
                                oneshot::channel();
                            //From now on wire connects directly with bparticipant!
                            s2b_create_channel_s
                                .send((
                                    cid,
                                    sid,
                                    protocol,
                                    leftover_cid_frame,
                                    b2s_create_channel_done_s,
                                ))
                                .await
                                .unwrap();
                            b2s_create_channel_done_r.await.unwrap();
                            if let Some(pid_oneshot) = s2a_return_pid_s {
                                // someone is waiting with `connect`, so give them their PID
                                pid_oneshot.send(Ok(participant)).unwrap();
                            } else {
                                // noone is waiting on this Participant, return in to Network
                                participant_channels
                                    .s2a_connected_s
                                    .send(participant)
                                    .await
                                    .unwrap();
                            }
                        } else {
                            let pi = &participants[&pid];
                            trace!("2nd+ channel of participant, going to compare security ids");
                            if pi.secret != secret {
                                warn!(
                                    ?pid,
                                    ?secret,
                                    "Detected incompatible Secret!, this is probably an attack!"
                                );
                                error!("Just dropping here, TODO handle this correctly!");
                                //TODO
                                if let Some(pid_oneshot) = s2a_return_pid_s {
                                    // someone is waiting with `connect`, so give them their Error
                                    pid_oneshot
                                        .send(Err(std::io::Error::new(
                                            std::io::ErrorKind::PermissionDenied,
                                            "invalid secret, denying connection",
                                        )))
                                        .unwrap();
                                }
                                return;
                            }
                            error!(
                                "Ufff i cant answer the pid_oneshot. as i need to create the SAME \
                                 participant. maybe switch to ARC"
                            );
                        }
                        //From now on this CHANNEL can receiver other frames!
                        // move directly to participant!
                    },
                    Err(()) => {
                        if let Some(pid_oneshot) = s2a_return_pid_s {
                            // someone is waiting with `connect`, so give them their Error
                            trace!("returning the Err to api who requested the connect");
                            pid_oneshot
                                .send(Err(std::io::Error::new(
                                    std::io::ErrorKind::PermissionDenied,
                                    "Handshake failed, denying connection",
                                )))
                                .unwrap();
                        }
                    },
                }
            }
            .instrument(tracing::trace_span!("")),
        ); /*WORKAROUND FOR SPAN NOT TO GET LOST*/
    }
}
