use crate::{
    api::{Address, Participant},
    channel::Handshake,
    message::OutGoingMessage,
    metrics::NetworkMetrics,
    participant::BParticipant,
    prios::PrioManager,
    protocols::{Protocols, TcpProtocol, UdpProtocol},
    types::{Cid, Frame, Pid, Prio, Sid},
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
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use tracing::*;
use tracing_futures::Instrument;

#[derive(Debug)]
struct ParticipantInfo {
    s2b_create_channel_s: mpsc::UnboundedSender<(Cid, Sid, Protocols, oneshot::Sender<()>)>,
    s2b_frame_s: mpsc::UnboundedSender<(Pid, Sid, Frame)>,
    s2b_shutdown_bparticipant_s:
        Option<oneshot::Sender<oneshot::Sender<async_std::io::Result<()>>>>,
}

/// Naming of Channels `x2x`
///  - a: api
///  - s: scheduler
///  - b: bparticipant
///  - p: prios
///  - r: protocol
///  - w: wire
#[derive(Debug)]
struct ControlChannels {
    a2s_listen_r: mpsc::UnboundedReceiver<(Address, oneshot::Sender<io::Result<()>>)>,
    a2s_connect_r: mpsc::UnboundedReceiver<(Address, oneshot::Sender<io::Result<Participant>>)>,
    a2s_scheduler_shutdown_r: oneshot::Receiver<()>,
    a2s_disconnect_r: mpsc::UnboundedReceiver<(Pid, oneshot::Sender<async_std::io::Result<()>>)>,
}

#[derive(Debug, Clone)]
struct ParticipantChannels {
    s2a_connected_s: mpsc::UnboundedSender<Participant>,
    a2s_disconnect_s: mpsc::UnboundedSender<(Pid, oneshot::Sender<async_std::io::Result<()>>)>,
    a2p_msg_s: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
    p2b_notify_empty_stream_s: std::sync::mpsc::Sender<(Pid, Sid, oneshot::Sender<()>)>,
}

#[derive(Debug)]
pub struct Scheduler {
    local_pid: Pid,
    closed: AtomicBool,
    pool: Arc<ThreadPool>,
    run_channels: Option<ControlChannels>,
    participant_channels: Arc<Mutex<Option<ParticipantChannels>>>,
    participants: Arc<RwLock<HashMap<Pid, ParticipantInfo>>>,
    channel_ids: Arc<AtomicU64>,
    channel_listener: RwLock<HashMap<Address, oneshot::Sender<()>>>,
    prios: Arc<Mutex<PrioManager>>,
    metrics: Arc<NetworkMetrics>,
}

impl Scheduler {
    pub fn new(
        local_pid: Pid,
        registry: Option<&Registry>,
    ) -> (
        Self,
        mpsc::UnboundedSender<(Address, oneshot::Sender<io::Result<()>>)>,
        mpsc::UnboundedSender<(Address, oneshot::Sender<io::Result<Participant>>)>,
        mpsc::UnboundedReceiver<Participant>,
        oneshot::Sender<()>,
    ) {
        let (a2s_listen_s, a2s_listen_r) =
            mpsc::unbounded::<(Address, oneshot::Sender<io::Result<()>>)>();
        let (a2s_connect_s, a2s_connect_r) =
            mpsc::unbounded::<(Address, oneshot::Sender<io::Result<Participant>>)>();
        let (s2a_connected_s, s2a_connected_r) = mpsc::unbounded::<Participant>();
        let (a2s_scheduler_shutdown_s, a2s_scheduler_shutdown_r) = oneshot::channel::<()>();
        let (prios, a2p_msg_s, p2b_notify_empty_stream_s) = PrioManager::new();
        let (a2s_disconnect_s, a2s_disconnect_r) =
            mpsc::unbounded::<(Pid, oneshot::Sender<async_std::io::Result<()>>)>();

        let run_channels = Some(ControlChannels {
            a2s_listen_r,
            a2s_connect_r,
            a2s_scheduler_shutdown_r,
            a2s_disconnect_r,
        });

        let participant_channels = ParticipantChannels {
            s2a_connected_s,
            a2s_disconnect_s,
            a2p_msg_s,
            p2b_notify_empty_stream_s,
        };

        let metrics = Arc::new(NetworkMetrics::new(&local_pid).unwrap());
        if let Some(registry) = registry {
            metrics.register(registry).unwrap();
        }

        (
            Self {
                local_pid,
                closed: AtomicBool::new(false),
                pool: Arc::new(ThreadPool::new().unwrap()),
                run_channels,
                participant_channels: Arc::new(Mutex::new(Some(participant_channels))),
                participants: Arc::new(RwLock::new(HashMap::new())),
                channel_ids: Arc::new(AtomicU64::new(0)),
                channel_listener: RwLock::new(HashMap::new()),
                prios: Arc::new(Mutex::new(prios)),
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
            self.send_outgoing_mgr(),
            self.scheduler_shutdown_mgr(run_channels.a2s_scheduler_shutdown_r),
        );
    }

    async fn listen_mgr(
        &self,
        a2s_listen_r: mpsc::UnboundedReceiver<(Address, oneshot::Sender<io::Result<()>>)>,
    ) {
        trace!("start listen_mgr");
        a2s_listen_r
            .for_each_concurrent(None, |(address, s2a_result_s)| {
                let address = address.clone();

                async move {
                    debug!(?address, "got request to open a channel_creator");
                    self.metrics
                        .listen_requests_total
                        .with_label_values(&[match address {
                            Address::Tcp(_) => "tcp",
                            Address::Udp(_) => "udp",
                            Address::Mpsc(_) => "mpsc",
                        }])
                        .inc();
                    let (end_sender, end_receiver) = oneshot::channel::<()>();
                    self.channel_listener
                        .write()
                        .await
                        .insert(address.clone(), end_sender);
                    self.channel_creator(address, end_receiver, s2a_result_s)
                        .await;
                }
            })
            .await;
        trace!("stop listen_mgr");
    }

    async fn connect_mgr(
        &self,
        mut a2s_connect_r: mpsc::UnboundedReceiver<(
            Address,
            oneshot::Sender<io::Result<Participant>>,
        )>,
    ) {
        trace!("start connect_mgr");
        while let Some((addr, pid_sender)) = a2s_connect_r.next().await {
            let (protocol, handshake) = match addr {
                Address::Tcp(addr) => {
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
                    let protocol = Protocols::Tcp(TcpProtocol::new(stream, self.metrics.clone()));
                    (protocol, false)
                },
                Address::Udp(addr) => {
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
                    let protocol = Protocols::Udp(UdpProtocol::new(
                        socket.clone(),
                        addr,
                        self.metrics.clone(),
                        udp_data_receiver,
                    ));
                    self.pool.spawn_ok(
                        Self::udp_single_channel_connect(socket.clone(), udp_data_sender)
                            .instrument(tracing::info_span!("udp", ?addr)),
                    );
                    (protocol, true)
                },
                _ => unimplemented!(),
            };
            self.init_protocol(protocol, Some(pid_sender), handshake)
                .await;
        }
        trace!("stop connect_mgr");
    }

    async fn disconnect_mgr(
        &self,
        mut a2s_disconnect_r: mpsc::UnboundedReceiver<(
            Pid,
            oneshot::Sender<async_std::io::Result<()>>,
        )>,
    ) {
        trace!("start disconnect_mgr");
        while let Some((pid, return_once_successfull_shutdown)) = a2s_disconnect_r.next().await {
            //Closing Participants is done the following way:
            // 1. We drop our senders and receivers
            // 2. we need to close BParticipant, this will drop its senderns and receivers
            // 3. Participant will try to access the BParticipant senders and receivers with
            // their next api action, it will fail and be closed then.
            let (finished_sender, finished_receiver) = oneshot::channel();
            if let Some(pi) = self.participants.write().await.get_mut(&pid) {
                pi.s2b_shutdown_bparticipant_s
                    .take()
                    .unwrap()
                    .send(finished_sender)
                    .unwrap();
            }
            let e = finished_receiver.await.unwrap();
            //only remove after flush!
            self.participants.write().await.remove(&pid).unwrap();
            return_once_successfull_shutdown.send(e);
        }
        trace!("stop disconnect_mgr");
    }

    async fn send_outgoing_mgr(&self) {
        //This time equals the MINIMUM Latency in average, so keep it down and //Todo:
        // make it configureable or switch to await E.g. Prio 0 = await, prio 50
        // wait for more messages
        const TICK_TIME: std::time::Duration = std::time::Duration::from_millis(10);
        const FRAMES_PER_TICK: usize = 1000005;
        trace!("start send_outgoing_mgr");
        while !self.closed.load(Ordering::Relaxed) {
            let mut frames = VecDeque::new();
            self.prios
                .lock()
                .await
                .fill_frames(FRAMES_PER_TICK, &mut frames);
            if frames.len() > 0 {
                trace!("tick {}", frames.len());
            }
            let mut already_traced = HashSet::new();
            for (pid, sid, frame) in frames {
                if let Some(pi) = self.participants.write().await.get_mut(&pid) {
                    pi.s2b_frame_s.send((pid, sid, frame)).await.unwrap();
                } else {
                    if !already_traced.contains(&(pid, sid)) {
                        error!(
                            ?pid,
                            ?sid,
                            "dropping frames, as participant no longer exists!"
                        );
                        already_traced.insert((pid, sid));
                    }
                }
            }
            async_std::task::sleep(TICK_TIME).await;
        }
        trace!("stop send_outgoing_mgr");
    }

    async fn scheduler_shutdown_mgr(&self, a2s_scheduler_shutdown_r: oneshot::Receiver<()>) {
        trace!("start scheduler_shutdown_mgr");
        a2s_scheduler_shutdown_r.await.unwrap();
        self.closed.store(true, Ordering::Relaxed);
        debug!("shutting down all BParticipants gracefully");
        let mut participants = self.participants.write().await;
        let mut waitings = vec![];
        //close participants but don't remove them from self.participants yet
        for (pid, pi) in participants.iter_mut() {
            trace!(?pid, "shutting down BParticipants");
            let (finished_sender, finished_receiver) = oneshot::channel();
            waitings.push((pid, finished_receiver));
            pi.s2b_shutdown_bparticipant_s
                .take()
                .unwrap()
                .send(finished_sender)
                .unwrap();
        }
        debug!("wait for partiticipants to be shut down");
        for (pid, recv) in waitings {
            match recv.await {
                Err(e) => error!(
                    ?pid,
                    ?e,
                    "failed to finish sending all remainding messages to participant when \
                     shutting down"
                ),
                _ => (),
            };
        }
        //remove participants once everything is shut down
        participants.clear();
        //removing the possibility to create new participants, needed to close down
        // some mgr:
        self.participant_channels.lock().await.take();

        trace!("stop scheduler_shutdown_mgr");
    }

    async fn channel_creator(
        &self,
        addr: Address,
        s2s_stop_listening_r: oneshot::Receiver<()>,
        s2a_listen_result_s: oneshot::Sender<io::Result<()>>,
    ) {
        trace!(?addr, "start up channel creator");
        match addr {
            Address::Tcp(addr) => {
                let listener = match net::TcpListener::bind(addr).await {
                    Ok(listener) => {
                        s2a_listen_result_s.send(Ok(())).unwrap();
                        listener
                    },
                    Err(e) => {
                        info!(
                            ?addr,
                            ?e,
                            "listener couldn't be started due to error on tcp bind"
                        );
                        s2a_listen_result_s.send(Err(e)).unwrap();
                        return;
                    },
                };
                trace!(?addr, "listener bound");
                let mut incoming = listener.incoming();
                let mut end_receiver = s2s_stop_listening_r.fuse();
                while let Some(stream) = select! {
                    next = incoming.next().fuse() => next,
                    _ = end_receiver => None,
                } {
                    let stream = stream.unwrap();
                    info!("Accepting Tcp from: {}", stream.peer_addr().unwrap());
                    self.init_protocol(
                        Protocols::Tcp(TcpProtocol::new(stream, self.metrics.clone())),
                        None,
                        true,
                    )
                    .await;
                }
            },
            Address::Udp(addr) => {
                let socket = match net::UdpSocket::bind(addr).await {
                    Ok(socket) => {
                        s2a_listen_result_s.send(Ok(())).unwrap();
                        Arc::new(socket)
                    },
                    Err(e) => {
                        info!(
                            ?addr,
                            ?e,
                            "listener couldn't be started due to error on udp bind"
                        );
                        s2a_listen_result_s.send(Err(e)).unwrap();
                        return;
                    },
                };
                trace!(?addr, "listener bound");
                // receiving is done from here and will be piped to protocol as UDP does not
                // have any state
                let mut listeners = HashMap::new();
                let mut end_receiver = s2s_stop_listening_r.fuse();
                let mut data = [0u8; 9216];
                while let Ok((size, remote_addr)) = select! {
                    next = socket.recv_from(&mut data).fuse() => next,
                    _ = end_receiver => Err(std::io::Error::new(std::io::ErrorKind::Other, "")),
                } {
                    let mut datavec = Vec::with_capacity(size);
                    datavec.extend_from_slice(&data[0..size]);
                    if !listeners.contains_key(&remote_addr) {
                        info!("Accepting Udp from: {}", &remote_addr);
                        let (udp_data_sender, udp_data_receiver) = mpsc::unbounded::<Vec<u8>>();
                        listeners.insert(remote_addr.clone(), udp_data_sender);
                        let protocol = Protocols::Udp(UdpProtocol::new(
                            socket.clone(),
                            remote_addr.clone(),
                            self.metrics.clone(),
                            udp_data_receiver,
                        ));
                        self.init_protocol(protocol, None, false).await;
                    }
                    let udp_data_sender = listeners.get_mut(&remote_addr).unwrap();
                    udp_data_sender.send(datavec).await.unwrap();
                }
            },
            _ => unimplemented!(),
        }
        trace!(?addr, "ending channel creator");
    }

    async fn udp_single_channel_connect(
        socket: Arc<net::UdpSocket>,
        mut w2p_udp_package_s: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        let addr = socket.local_addr();
        trace!(?addr, "start udp_single_channel_connect");
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
        trace!(?addr, "stop udp_single_channel_connect");
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
        self.pool.spawn_ok(async move {
            trace!(?cid, "open channel and be ready for Handshake");
            let handshake = Handshake::new(cid, local_pid, metrics.clone(), send_handshake);
            match handshake.setup(&protocol).await {
                Ok((pid, sid)) => {
                    trace!(
                        ?cid,
                        ?pid,
                        "detected that my channel is ready!, activating it :)"
                    );
                    let mut participants = participants.write().await;
                    if !participants.contains_key(&pid) {
                        debug!(?cid, "new participant connected via a channel");
                        let (
                            bparticipant,
                            a2b_steam_open_s,
                            b2a_stream_opened_r,
                            mut s2b_create_channel_s,
                            s2b_frame_s,
                            s2b_shutdown_bparticipant_s,
                        ) = BParticipant::new(
                            pid,
                            sid,
                            metrics.clone(),
                            participant_channels.a2p_msg_s,
                            participant_channels.p2b_notify_empty_stream_s,
                        );

                        let participant = Participant::new(
                            local_pid,
                            pid,
                            a2b_steam_open_s,
                            b2a_stream_opened_r,
                            participant_channels.a2s_disconnect_s,
                        );

                        metrics.participants_connected_total.inc();
                        participants.insert(pid, ParticipantInfo {
                            s2b_create_channel_s: s2b_create_channel_s.clone(),
                            s2b_frame_s,
                            s2b_shutdown_bparticipant_s: Some(s2b_shutdown_bparticipant_s),
                        });
                        pool.spawn_ok(
                            bparticipant
                                .run()
                                .instrument(tracing::info_span!("participant", ?pid)),
                        );
                        //create a new channel within BParticipant and wait for it to run
                        let (b2s_create_channel_done_s, b2s_create_channel_done_r) =
                            oneshot::channel();
                        s2b_create_channel_s
                            .send((cid, sid, protocol, b2s_create_channel_done_s))
                            .await
                            .unwrap();
                        b2s_create_channel_done_r.await.unwrap();
                        if let Some(pid_oneshot) = s2a_return_pid_s {
                            // someone is waiting with connect, so give them their PID
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
                        error!(
                            "2ND channel of participants opens, but we cannot verify that this is \
                             not a attack to "
                        );
                        //ERROR DEADLOCK AS NO SENDER HERE!
                        //sender.send(frame_recv_sender).unwrap();
                    }
                    //From now on this CHANNEL can receiver other frames! move
                    // directly to participant!
                },
                Err(()) => {},
            }
        });
    }
}
