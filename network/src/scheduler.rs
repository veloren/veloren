use crate::{
    api::{Address, Participant},
    channel::Channel,
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
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};
use tracing::*;
use tracing_futures::Instrument;
//use futures::prelude::*;

type ParticipantInfo = (
    mpsc::UnboundedSender<(Cid, mpsc::UnboundedSender<Frame>)>,
    mpsc::UnboundedSender<(Pid, Sid, Frame)>,
    oneshot::Sender<()>,
);
type UnknownChannelInfo = (
    mpsc::UnboundedSender<Frame>,
    Option<oneshot::Sender<io::Result<Participant>>>,
);
pub(crate) type ConfigureInfo = (
    Cid,
    Pid,
    Sid,
    oneshot::Sender<mpsc::UnboundedSender<(Cid, Frame)>>,
);

#[derive(Debug)]
struct ControlChannels {
    listen_receiver: mpsc::UnboundedReceiver<(Address, oneshot::Sender<io::Result<()>>)>,
    connect_receiver: mpsc::UnboundedReceiver<(Address, oneshot::Sender<io::Result<Participant>>)>,
    connected_sender: mpsc::UnboundedSender<Participant>,
    shutdown_receiver: oneshot::Receiver<()>,
    prios_sender: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
}

#[derive(Debug)]
pub struct Scheduler {
    local_pid: Pid,
    closed: AtomicBool,
    pool: Arc<ThreadPool>,
    run_channels: Option<ControlChannels>,
    participants: Arc<RwLock<HashMap<Pid, ParticipantInfo>>>,
    participant_from_channel: Arc<RwLock<HashMap<Cid, Pid>>>,
    channel_ids: Arc<AtomicU64>,
    channel_listener: RwLock<HashMap<Address, oneshot::Sender<()>>>,
    unknown_channels: Arc<RwLock<HashMap<Cid, UnknownChannelInfo>>>,
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
        let (listen_sender, listen_receiver) =
            mpsc::unbounded::<(Address, oneshot::Sender<io::Result<()>>)>();
        let (connect_sender, connect_receiver) =
            mpsc::unbounded::<(Address, oneshot::Sender<io::Result<Participant>>)>();
        let (connected_sender, connected_receiver) = mpsc::unbounded::<Participant>();
        let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
        let (prios, prios_sender) = PrioManager::new();

        let run_channels = Some(ControlChannels {
            listen_receiver,
            connect_receiver,
            connected_sender,
            shutdown_receiver,
            prios_sender,
        });

        let metrics = Arc::new(NetworkMetrics::new().unwrap());
        if let Some(registry) = registry {
            metrics.register(registry).unwrap();
        }

        (
            Self {
                local_pid,
                closed: AtomicBool::new(false),
                pool: Arc::new(ThreadPool::new().unwrap()),
                run_channels,
                participants: Arc::new(RwLock::new(HashMap::new())),
                participant_from_channel: Arc::new(RwLock::new(HashMap::new())),
                channel_ids: Arc::new(AtomicU64::new(0)),
                channel_listener: RwLock::new(HashMap::new()),
                unknown_channels: Arc::new(RwLock::new(HashMap::new())),
                prios: Arc::new(Mutex::new(prios)),
                metrics,
            },
            listen_sender,
            connect_sender,
            connected_receiver,
            shutdown_sender,
        )
    }

    pub async fn run(mut self) {
        let (configured_sender, configured_receiver) = mpsc::unbounded::<ConfigureInfo>();
        let (disconnect_sender, disconnect_receiver) = mpsc::unbounded::<Pid>();
        let (stream_finished_request_sender, stream_finished_request_receiver) = mpsc::unbounded();
        let run_channels = self.run_channels.take().unwrap();

        futures::join!(
            self.listen_manager(run_channels.listen_receiver, configured_sender.clone(),),
            self.connect_manager(run_channels.connect_receiver, configured_sender,),
            self.disconnect_manager(disconnect_receiver,),
            self.send_outgoing(),
            self.stream_finished_manager(stream_finished_request_receiver),
            self.shutdown_manager(run_channels.shutdown_receiver),
            self.channel_configurer(
                run_channels.connected_sender,
                configured_receiver,
                disconnect_sender,
                run_channels.prios_sender.clone(),
                stream_finished_request_sender.clone(),
            ),
        );
    }

    async fn listen_manager(
        &self,
        listen_receiver: mpsc::UnboundedReceiver<(Address, oneshot::Sender<io::Result<()>>)>,
        configured_sender: mpsc::UnboundedSender<ConfigureInfo>,
    ) {
        trace!("start listen_manager");
        listen_receiver
            .for_each_concurrent(None, |(address, result_sender)| {
                let address = address.clone();
                let configured_sender = configured_sender.clone();

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
                    self.channel_creator(
                        address,
                        end_receiver,
                        configured_sender.clone(),
                        result_sender,
                    )
                    .await;
                }
            })
            .await;
        trace!("stop listen_manager");
    }

    async fn connect_manager(
        &self,
        mut connect_receiver: mpsc::UnboundedReceiver<(
            Address,
            oneshot::Sender<io::Result<Participant>>,
        )>,
        configured_sender: mpsc::UnboundedSender<ConfigureInfo>,
    ) {
        trace!("start connect_manager");
        while let Some((addr, pid_sender)) = connect_receiver.next().await {
            let (addr, protocol, handshake) = match addr {
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
                    (addr, protocol, false)
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
                    (addr, protocol, true)
                },
                _ => unimplemented!(),
            };
            self.init_protocol(
                addr,
                &configured_sender,
                protocol,
                Some(pid_sender),
                handshake,
            )
            .await;
        }
        trace!("stop connect_manager");
    }

    async fn disconnect_manager(&self, mut disconnect_receiver: mpsc::UnboundedReceiver<Pid>) {
        trace!("start disconnect_manager");
        while let Some(pid) = disconnect_receiver.next().await {
            //Closing Participants is done the following way:
            // 1. We drop our senders and receivers
            // 2. we need to close BParticipant, this will drop its senderns and receivers
            // 3. Participant will try to access the BParticipant senders and receivers with
            // their next api action, it will fail and be closed then.
            if let Some((_, _, sender)) = self.participants.write().await.remove(&pid) {
                sender.send(()).unwrap();
            }
        }
        trace!("stop disconnect_manager");
    }

    async fn send_outgoing(&self) {
        //This time equals the MINIMUM Latency in average, so keep it down and //Todo:
        // make it configureable or switch to await E.g. Prio 0 = await, prio 50
        // wait for more messages
        const TICK_TIME: std::time::Duration = std::time::Duration::from_millis(10);
        const FRAMES_PER_TICK: usize = 1000000;
        trace!("start send_outgoing");
        while !self.closed.load(Ordering::Relaxed) {
            let mut frames = VecDeque::new();
            self.prios
                .lock()
                .await
                .fill_frames(FRAMES_PER_TICK, &mut frames);
            for (pid, sid, frame) in frames {
                if let Some((_, sender, _)) = self.participants.write().await.get_mut(&pid) {
                    sender.send((pid, sid, frame)).await.unwrap();
                }
            }
            async_std::task::sleep(TICK_TIME).await;
        }
        trace!("stop send_outgoing");
    }

    //TODO: //ERROR CHECK IF THIS SHOULD BE PUT IN A ASYNC FUNC WHICH IS SEND OVER
    // TO CHANNEL OR NOT FOR RETURN VALUE!

    async fn channel_configurer(
        &self,
        mut connected_sender: mpsc::UnboundedSender<Participant>,
        mut receiver: mpsc::UnboundedReceiver<ConfigureInfo>,
        disconnect_sender: mpsc::UnboundedSender<Pid>,
        prios_sender: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
        stream_finished_request_sender: mpsc::UnboundedSender<(Pid, Sid, oneshot::Sender<()>)>,
    ) {
        trace!("start channel_activator");
        while let Some((cid, pid, offset_sid, sender)) = receiver.next().await {
            if let Some((frame_sender, pid_oneshot)) =
                self.unknown_channels.write().await.remove(&cid)
            {
                trace!(
                    ?cid,
                    ?pid,
                    "detected that my channel is ready!, activating it :)"
                );
                let mut participants = self.participants.write().await;
                if !participants.contains_key(&pid) {
                    debug!(?cid, "new participant connected via a channel");
                    let (
                        bparticipant,
                        stream_open_sender,
                        stream_opened_receiver,
                        mut transfer_channel_receiver,
                        frame_recv_sender,
                        frame_send_sender,
                        shutdown_sender,
                    ) = BParticipant::new(
                        pid,
                        offset_sid,
                        self.metrics.clone(),
                        prios_sender.clone(),
                        stream_finished_request_sender.clone(),
                    );

                    let participant = Participant::new(
                        self.local_pid,
                        pid,
                        stream_open_sender,
                        stream_opened_receiver,
                        disconnect_sender.clone(),
                    );
                    if let Some(pid_oneshot) = pid_oneshot {
                        // someone is waiting with connect, so give them their PID
                        pid_oneshot.send(Ok(participant)).unwrap();
                    } else {
                        // noone is waiting on this Participant, return in to Network
                        connected_sender.send(participant).await.unwrap();
                    }
                    self.metrics.participants_connected_total.inc();
                    transfer_channel_receiver
                        .send((cid, frame_sender))
                        .await
                        .unwrap();
                    participants.insert(
                        pid,
                        (
                            transfer_channel_receiver,
                            frame_send_sender,
                            shutdown_sender,
                        ),
                    );
                    self.participant_from_channel.write().await.insert(cid, pid);
                    self.pool.spawn_ok(
                        bparticipant
                            .run()
                            .instrument(tracing::info_span!("participant", ?pid)),
                    );
                    sender.send(frame_recv_sender).unwrap();
                } else {
                    error!(
                        "2ND channel of participants opens, but we cannot verify that this is not \
                         a attack to "
                    );
                    //ERROR DEADLOCK AS NO SENDER HERE!
                    //sender.send(frame_recv_sender).unwrap();
                }
                //From now on this CHANNEL can receiver other frames! move
                // directly to participant!
            }
        }
        trace!("stop channel_activator");
    }

    // requested by participant when stream wants to close from api, checking if no
    // more msg is in prio and return
    pub(crate) async fn stream_finished_manager(
        &self,
        stream_finished_request_receiver: mpsc::UnboundedReceiver<(Pid, Sid, oneshot::Sender<()>)>,
    ) {
        trace!("start stream_finished_manager");
        stream_finished_request_receiver
            .for_each_concurrent(None, async move |(pid, sid, sender)| {
                //TODO: THERE MUST BE A MORE CLEVER METHOD THAN SPIN LOCKING! LIKE REGISTERING
                // DIRECTLY IN PRIO AS A FUTURE WERE PRIO IS WAKER! TODO: also this
                // has a great potential for handing network, if you create a network, send
                // gigabytes close it then. Also i need a Mutex, which really adds
                // to cost if alot strems want to close
                self.stream_finished_waiter(pid, sid, sender).await;
            })
            .await;
    }

    async fn stream_finished_waiter(&self, pid: Pid, sid: Sid, sender: oneshot::Sender<()>) {
        const TICK_TIME: std::time::Duration = std::time::Duration::from_millis(5);
        //TODO: ARRRG, i need to wait for AT LEAST 1 TICK, because i am lazy i just
        // wait 15mn and tick count is 10ms because recv is only done with a
        // tick and not async as soon as we send....
        async_std::task::sleep(TICK_TIME * 3).await;
        let mut n = 0u64;
        loop {
            if !self.prios.lock().await.contains_pid_sid(pid, sid) {
                trace!("prio is clear, go to close stream as requested from api");
                sender.send(()).unwrap();
                break;
            }
            n += 1;
            async_std::task::sleep(match n {
                0..=199 => TICK_TIME,
                n if n.rem_euclid(100) == 0 => {
                    warn!(?pid, ?sid, ?n, "cant close stream, as it still queued");
                    TICK_TIME * (n as f32 * (n as f32).sqrt() / 100.0) as u32
                },
                n => TICK_TIME * (n as f32 * (n as f32).sqrt() / 100.0) as u32,
            })
            .await;
        }
    }

    pub(crate) async fn shutdown_manager(&self, receiver: oneshot::Receiver<()>) {
        trace!("start shutdown_manager");
        receiver.await.unwrap();
        self.closed.store(true, Ordering::Relaxed);
        debug!("shutting down all BParticipants gracefully");
        let mut participants = self.participants.write().await;
        for (pid, (_, _, sender)) in participants.drain() {
            trace!(?pid, "shutting down BParticipants");
            sender.send(()).unwrap();
        }
        trace!("stop shutdown_manager");
    }

    pub(crate) async fn channel_creator(
        &self,
        addr: Address,
        end_receiver: oneshot::Receiver<()>,
        configured_sender: mpsc::UnboundedSender<ConfigureInfo>,
        result_sender: oneshot::Sender<io::Result<()>>,
    ) {
        info!(?addr, "start up channel creator");
        match addr {
            Address::Tcp(addr) => {
                let listener = match net::TcpListener::bind(addr).await {
                    Ok(listener) => {
                        result_sender.send(Ok(())).unwrap();
                        listener
                    },
                    Err(e) => {
                        info!(
                            ?addr,
                            ?e,
                            "listener couldn't be started due to error on tcp bind"
                        );
                        result_sender.send(Err(e)).unwrap();
                        return;
                    },
                };
                trace!(?addr, "listener bound");
                let mut incoming = listener.incoming();
                let mut end_receiver = end_receiver.fuse();
                while let Some(stream) = select! {
                    next = incoming.next().fuse() => next,
                    _ = end_receiver => None,
                } {
                    let stream = stream.unwrap();
                    info!("Accepting Tcp from: {}", stream.peer_addr().unwrap());
                    self.init_protocol(
                        addr,
                        &configured_sender,
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
                        result_sender.send(Ok(())).unwrap();
                        Arc::new(socket)
                    },
                    Err(e) => {
                        info!(
                            ?addr,
                            ?e,
                            "listener couldn't be started due to error on udp bind"
                        );
                        result_sender.send(Err(e)).unwrap();
                        return;
                    },
                };
                trace!(?addr, "listener bound");
                // receiving is done from here and will be piped to protocol as UDP does not
                // have any state
                let mut listeners = HashMap::new();
                let mut end_receiver = end_receiver.fuse();
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
                            remote_addr,
                            self.metrics.clone(),
                            udp_data_receiver,
                        ));
                        self.init_protocol(addr, &configured_sender, protocol, None, true)
                            .await;
                    }
                    let udp_data_sender = listeners.get_mut(&remote_addr).unwrap();
                    udp_data_sender.send(datavec).await.unwrap();
                }
            },
            _ => unimplemented!(),
        }
        info!(?addr, "ending channel creator");
    }

    pub(crate) async fn udp_single_channel_connect(
        socket: Arc<net::UdpSocket>,
        mut udp_data_sender: mpsc::UnboundedSender<Vec<u8>>,
    ) {
        let addr = socket.local_addr();
        info!(?addr, "start udp_single_channel_connect");
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
            udp_data_sender.send(datavec).await.unwrap();
        }
        info!(?addr, "stop udp_single_channel_connect");
    }

    async fn init_protocol(
        &self,
        addr: std::net::SocketAddr,
        configured_sender: &mpsc::UnboundedSender<ConfigureInfo>,
        protocol: Protocols,
        pid_sender: Option<oneshot::Sender<io::Result<Participant>>>,
        send_handshake: bool,
    ) {
        let (mut part_in_sender, part_in_receiver) = mpsc::unbounded::<Frame>();
        //channels are unknown till PID is known!
        /* When A connects to a NETWORK, we, the listener answers with a Handshake.
          Pro: - Its easier to debug, as someone who opens a port gets a magic number back!
          Contra: - DOS posibility because we answer fist
                  - Speed, because otherwise the message can be send with the creation
        */
        let cid = self.channel_ids.fetch_add(1, Ordering::Relaxed);
        let channel = Channel::new(cid, self.local_pid, self.metrics.clone());
        if send_handshake {
            channel.send_handshake(&mut part_in_sender).await;
        }
        self.pool.spawn_ok(
            channel
                .run(protocol, part_in_receiver, configured_sender.clone())
                .instrument(tracing::info_span!("channel", ?addr)),
        );
        self.unknown_channels
            .write()
            .await
            .insert(cid, (part_in_sender, pid_sender));
    }
}
