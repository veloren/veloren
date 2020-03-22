use crate::{
    api::{Address, Participant},
    channel::Channel,
    frames::Frame,
    message::OutGoingMessage,
    participant::BParticipant,
    prios::PrioManager,
    types::{Cid, Pid, Prio, Sid},
};
use async_std::sync::RwLock;
use futures::{
    channel::{mpsc, oneshot},
    executor::ThreadPool,
    future::FutureExt,
    select,
    sink::SinkExt,
    stream::StreamExt,
};
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

#[derive(Debug)]
struct ControlChannels {
    listen_receiver: mpsc::UnboundedReceiver<Address>,
    connect_receiver: mpsc::UnboundedReceiver<(Address, oneshot::Sender<Participant>)>,
    connected_sender: mpsc::UnboundedSender<Participant>,
    shutdown_receiver: oneshot::Receiver<()>,
    prios: PrioManager,
    prios_sender: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
}

#[derive(Debug)]
pub struct Scheduler {
    local_pid: Pid,
    closed: AtomicBool,
    pool: Arc<ThreadPool>,
    run_channels: Option<ControlChannels>,
    participants: Arc<
        RwLock<
            HashMap<
                Pid,
                (
                    mpsc::UnboundedSender<(Cid, mpsc::UnboundedSender<Frame>)>,
                    mpsc::UnboundedSender<Frame>,
                    mpsc::UnboundedSender<(Pid, Sid, Frame)>,
                ),
            >,
        >,
    >,
    participant_from_channel: Arc<RwLock<HashMap<Cid, Pid>>>,
    channel_ids: Arc<AtomicU64>,
    channel_listener: RwLock<HashMap<Address, oneshot::Sender<()>>>,
    unknown_channels: Arc<
        RwLock<
            HashMap<
                Cid,
                (
                    mpsc::UnboundedSender<Frame>,
                    Option<oneshot::Sender<Participant>>,
                ),
            >,
        >,
    >,
}

impl Scheduler {
    pub fn new(
        local_pid: Pid,
    ) -> (
        Self,
        mpsc::UnboundedSender<Address>,
        mpsc::UnboundedSender<(Address, oneshot::Sender<Participant>)>,
        mpsc::UnboundedReceiver<Participant>,
        oneshot::Sender<()>,
    ) {
        let (listen_sender, listen_receiver) = mpsc::unbounded::<Address>();
        let (connect_sender, connect_receiver) =
            mpsc::unbounded::<(Address, oneshot::Sender<Participant>)>();
        let (connected_sender, connected_receiver) = mpsc::unbounded::<Participant>();
        let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
        let (prios, prios_sender) = PrioManager::new();

        let run_channels = Some(ControlChannels {
            listen_receiver,
            connect_receiver,
            connected_sender,
            shutdown_receiver,
            prios,
            prios_sender,
        });

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
            },
            listen_sender,
            connect_sender,
            connected_receiver,
            shutdown_sender,
        )
    }

    pub async fn run(mut self) {
        let (part_out_sender, part_out_receiver) = mpsc::unbounded::<(Cid, Frame)>();
        let (configured_sender, configured_receiver) = mpsc::unbounded::<(Cid, Pid, Sid)>();
        let (disconnect_sender, disconnect_receiver) = mpsc::unbounded::<Pid>();
        let run_channels = self.run_channels.take().unwrap();

        futures::join!(
            self.listen_manager(
                run_channels.listen_receiver,
                part_out_sender.clone(),
                configured_sender.clone(),
            ),
            self.connect_manager(
                run_channels.connect_receiver,
                part_out_sender,
                configured_sender,
            ),
            self.disconnect_manager(disconnect_receiver,),
            self.send_outgoing(run_channels.prios),
            self.shutdown_manager(run_channels.shutdown_receiver),
            self.handle_frames(part_out_receiver),
            self.channel_configurer(
                run_channels.connected_sender,
                configured_receiver,
                disconnect_sender,
                run_channels.prios_sender.clone(),
            ),
        );
    }

    async fn listen_manager(
        &self,
        mut listen_receiver: mpsc::UnboundedReceiver<Address>,
        part_out_sender: mpsc::UnboundedSender<(Cid, Frame)>,
        configured_sender: mpsc::UnboundedSender<(Cid, Pid, Sid)>,
    ) {
        trace!("start listen_manager");
        while let Some(address) = listen_receiver.next().await {
            debug!(?address, "got request to open a channel_creator");
            let (end_sender, end_receiver) = oneshot::channel::<()>();
            self.channel_listener
                .write()
                .await
                .insert(address.clone(), end_sender);
            self.pool.spawn_ok(Self::channel_creator(
                self.channel_ids.clone(),
                self.local_pid,
                address.clone(),
                end_receiver,
                self.pool.clone(),
                part_out_sender.clone(),
                configured_sender.clone(),
                self.unknown_channels.clone(),
            ));
        }
        trace!("stop listen_manager");
    }

    async fn connect_manager(
        &self,
        mut connect_receiver: mpsc::UnboundedReceiver<(Address, oneshot::Sender<Participant>)>,
        part_out_sender: mpsc::UnboundedSender<(Cid, Frame)>,
        configured_sender: mpsc::UnboundedSender<(Cid, Pid, Sid)>,
    ) {
        trace!("start connect_manager");
        while let Some((addr, pid_sender)) = connect_receiver.next().await {
            match addr {
                Address::Tcp(addr) => {
                    let stream = async_std::net::TcpStream::connect(addr).await.unwrap();
                    info!("Connectiong TCP to: {}", stream.peer_addr().unwrap());
                    let (part_in_sender, part_in_receiver) = mpsc::unbounded::<Frame>();
                    //channels are unknown till PID is known!
                    let cid = self.channel_ids.fetch_add(1, Ordering::Relaxed);
                    self.unknown_channels
                        .write()
                        .await
                        .insert(cid, (part_in_sender, Some(pid_sender)));
                    self.pool.spawn_ok(
                        Channel::new(cid, self.local_pid)
                            .run(
                                stream,
                                part_in_receiver,
                                part_out_sender.clone(),
                                configured_sender.clone(),
                            )
                            .instrument(tracing::info_span!("channel", ?addr)),
                    );
                },
                _ => unimplemented!(),
            }
        }
        trace!("stop connect_manager");
    }

    async fn disconnect_manager(&self, mut disconnect_receiver: mpsc::UnboundedReceiver<Pid>) {
        trace!("start disconnect_manager");
        while let Some(pid) = disconnect_receiver.next().await {
            error!(?pid, "I need to disconnect the pid");
        }
        trace!("stop disconnect_manager");
    }

    async fn send_outgoing(&self, mut prios: PrioManager) {
        //This time equals the MINIMUM Latency in average, so keep it down and //Todo:
        // make it configureable or switch to await E.g. Prio 0 = await, prio 50
        // wait for more messages
        const TICK_TIME: std::time::Duration = std::time::Duration::from_millis(10);
        trace!("start send_outgoing");
        while !self.closed.load(Ordering::Relaxed) {
            let mut frames = VecDeque::new();
            prios.fill_frames(3, &mut frames);
            for (pid, sid, frame) in frames {
                if let Some((_, _, sender)) = self.participants.write().await.get_mut(&pid) {
                    sender.send((pid, sid, frame)).await.unwrap();
                }
            }
            async_std::task::sleep(TICK_TIME).await;
        }
        trace!("stop send_outgoing");
    }

    async fn handle_frames(&self, mut part_out_receiver: mpsc::UnboundedReceiver<(Cid, Frame)>) {
        trace!("start handle_frames");
        while let Some((cid, frame)) = part_out_receiver.next().await {
            trace!("handling frame");
            if let Some(pid) = self.participant_from_channel.read().await.get(&cid) {
                if let Some((_, sender, _)) = self.participants.write().await.get_mut(&pid) {
                    sender.send(frame).await.unwrap();
                }
            } else {
                error!("dropping frame, unreachable, got a frame from a non existing channel");
            }
        }
        trace!("stop handle_frames");
    }

    //
    async fn channel_configurer(
        &self,
        mut connected_sender: mpsc::UnboundedSender<Participant>,
        mut receiver: mpsc::UnboundedReceiver<(Cid, Pid, Sid)>,
        disconnect_sender: mpsc::UnboundedSender<Pid>,
        prios_sender: std::sync::mpsc::Sender<(Prio, Pid, Sid, OutGoingMessage)>,
    ) {
        trace!("start channel_activator");
        while let Some((cid, pid, offset_sid)) = receiver.next().await {
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
                    let (shutdown_sender, shutdown_receiver) = oneshot::channel();

                    let (
                        bparticipant,
                        stream_open_sender,
                        stream_opened_receiver,
                        mut transfer_channel_receiver,
                        frame_recv_sender,
                        frame_send_sender,
                    ) = BParticipant::new(pid, offset_sid, prios_sender.clone());

                    let participant = Participant::new(
                        self.local_pid,
                        pid,
                        stream_open_sender,
                        stream_opened_receiver,
                        shutdown_receiver,
                        disconnect_sender.clone(),
                    );
                    if let Some(pid_oneshot) = pid_oneshot {
                        // someone is waiting with connect, so give them their PID
                        pid_oneshot.send(participant).unwrap();
                    } else {
                        // noone is waiting on this Participant, return in to Network
                        connected_sender.send(participant).await.unwrap();
                    }
                    transfer_channel_receiver
                        .send((cid, frame_sender))
                        .await
                        .unwrap();
                    participants.insert(
                        pid,
                        (
                            transfer_channel_receiver,
                            frame_recv_sender,
                            frame_send_sender,
                        ),
                    );
                    self.participant_from_channel.write().await.insert(cid, pid);
                    self.pool.spawn_ok(
                        bparticipant
                            .run()
                            .instrument(tracing::info_span!("participant", ?pid)),
                    );
                } else {
                    error!(
                        "2ND channel of participants opens, but we cannot verify that this is not \
                         a attack to "
                    )
                }
            }
        }
        trace!("stop channel_activator");
    }

    pub async fn shutdown_manager(&self, receiver: oneshot::Receiver<()>) {
        trace!("start shutdown_manager");
        receiver.await.unwrap();
        self.closed.store(true, Ordering::Relaxed);
        trace!("stop shutdown_manager");
    }

    pub async fn channel_creator(
        channel_ids: Arc<AtomicU64>,
        local_pid: Pid,
        addr: Address,
        end_receiver: oneshot::Receiver<()>,
        pool: Arc<ThreadPool>,
        part_out_sender: mpsc::UnboundedSender<(Cid, Frame)>,
        configured_sender: mpsc::UnboundedSender<(Cid, Pid, Sid)>,
        unknown_channels: Arc<
            RwLock<
                HashMap<
                    Cid,
                    (
                        mpsc::UnboundedSender<Frame>,
                        Option<oneshot::Sender<Participant>>,
                    ),
                >,
            >,
        >,
    ) {
        info!(?addr, "start up channel creator");
        match addr {
            Address::Tcp(addr) => {
                let listener = async_std::net::TcpListener::bind(addr).await.unwrap();
                let mut incoming = listener.incoming();
                let mut end_receiver = end_receiver.fuse();
                while let Some(stream) = select! {
                    next = incoming.next().fuse() => next,
                    _ = end_receiver => None,
                } {
                    let stream = stream.unwrap();
                    info!("Accepting TCP from: {}", stream.peer_addr().unwrap());
                    let (mut part_in_sender, part_in_receiver) = mpsc::unbounded::<Frame>();
                    //channels are unknown till PID is known!
                    /* When A connects to a NETWORK, we, the listener answers with a Handshake.
                      Pro: - Its easier to debug, as someone who opens a port gets a magic number back!
                      Contra: - DOS posibility because we answer fist
                              - Speed, because otherwise the message can be send with the creation
                    */
                    let cid = channel_ids.fetch_add(1, Ordering::Relaxed);
                    let channel = Channel::new(cid, local_pid);
                    channel.send_handshake(&mut part_in_sender).await;
                    pool.spawn_ok(
                        channel
                            .run(
                                stream,
                                part_in_receiver,
                                part_out_sender.clone(),
                                configured_sender.clone(),
                            )
                            .instrument(tracing::info_span!("channel", ?addr)),
                    );
                    unknown_channels
                        .write()
                        .await
                        .insert(cid, (part_in_sender, None));
                }
            },
            _ => unimplemented!(),
        }
        info!(?addr, "ending channel creator");
    }
}

/*
use crate::{
    async_serde,
    channel::{Channel, ChannelProtocol, ChannelProtocols},
    controller::Controller,
    metrics::NetworkMetrics,
    prios::PrioManager,
    tcp::TcpChannel,
    types::{CtrlMsg, Pid, RtrnMsg, Sid, TokenObjects},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
        mpsc::TryRecvError,
        Arc,
    },
    time::Instant,
};
use tlid;
use tracing::*;
use crate::types::Protocols;
use crate::frames::{ChannelFrame, ParticipantFrame, StreamFrame, Frame};

/*
The worker lives in a own thread and only communcates with the outside via a Channel

Prios are done per participant, but their throughput is split equalli,
That allows indepentend calculation of prios (no global hotspot) while no Participant is starved as the total throughput is measured and aproximated :)

streams are per participant, and channels are per participants, streams dont have a specific channel!
*/

use async_std::sync::RwLock;
use async_std::io::prelude::*;
use crate::async_serde::{SerializeFuture, DeserializeFuture};
use uvth::ThreadPoolBuilder;
use async_std::stream::Stream;
use async_std::sync::{self, Sender, Receiver};
use crate::types::{VELOREN_MAGIC_NUMBER, VELOREN_NETWORK_VERSION,};
use crate::message::InCommingMessage;

use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::{select, FutureExt};

#[derive(Debug)]
struct BStream {
    sid: Sid,
    prio: u8,
    promises: u8,
}

struct BChannel {
    remote_pid: Option<Pid>,
    stream: RwLock<async_std::net::TcpStream>,
    send_stream: Sender<Frame>,
    recv_stream: Receiver<Frame>,
    send_participant: Sender<Frame>,
    recv_participant: Receiver<Frame>,

    send_handshake: bool,
    send_pid: bool,
    send_shutdown: bool,
    recv_handshake: bool,
    recv_pid: bool,
    recv_shutdown: bool,
}

struct BAcceptor {
    listener: RwLock<async_std::net::TcpListener>,
}

struct BParticipant {
    remote_pid: Pid,
    channels: HashMap<Protocols, Vec<BChannel>>,
    streams: Vec<BStream>,
    sid_pool: tlid::Pool<tlid::Wrapping<Sid>>,
    prios: RwLock<PrioManager>,
    closed: AtomicBool,
}

pub(crate) struct Scheduler {
    local_pid: Pid,
    metrics: Arc<Option<NetworkMetrics>>,
    participants: HashMap<Pid, BParticipant>,
    pending_channels: HashMap<Protocols, Vec<BChannel>>,
    /* ctrl_rx: Receiver<CtrlMsg>,
     * rtrn_tx: mpsc::Sender<RtrnMsg>, */
}

impl BStream {

}

impl BChannel {
    /*
    /// Execute when ready to read
    pub async fn recv(&self) -> Vec<Frame> {
        let mut buffer: [u8; 2000] = [0; 2000];
        let read = self.stream.write().await.read(&mut buffer).await;
        match read {
            Ok(n) => {
                let x = DeserializeFuture::new(buffer[0..n].to_vec(), &ThreadPoolBuilder::new().build()).await;
                return vec!(x);
            },
            Err(e) => {
                panic!("woops {}", e);
            }
        }
    }
    /// Execute when ready to write
    pub async fn send<I: std::iter::Iterator<Item = Frame>>(&self, frames: &mut I) {
        for frame in frames {
            let x = SerializeFuture::new(frame, &ThreadPoolBuilder::new().build()).await;
            self.stream.write().await.write_all(&x).await;
        }
    }
    */

    pub fn get_tx(&self) -> &Sender<Frame> {
        &self.send_stream
    }

    pub fn get_rx(&self) -> &Receiver<Frame> {
        &self.recv_stream
    }

    pub fn get_participant_tx(&self) -> &Sender<Frame> {
        &self.send_participant
    }

    pub fn get_participant_rx(&self) -> &Receiver<Frame> {
        &self.recv_participant
    }
}



impl BParticipant {
    pub async fn read(&self) {
        while self.closed.load(Ordering::Relaxed) {
            for channels in self.channels.values() {
                for channel in channels.iter() {
                    //let frames = channel.recv().await;
                    let frame = channel.get_rx().recv().await.unwrap();
                    match frame {
                        Frame::Channel(cf) => channel.handle(cf).await,
                        Frame::Participant(pf) => self.handle(pf).await,
                        Frame::Stream(sf) => {},
                    }
                }
            }
            async_std::task::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    pub async fn write(&self) {
        let mut frames = VecDeque::<(u8, StreamFrame)>::new();
        while self.closed.load(Ordering::Relaxed) {
            let todo_synced_amount_and_reasonable_choosen_throughput_based_on_feedback = 100;
            self.prios.write().await.fill_frames(
                todo_synced_amount_and_reasonable_choosen_throughput_based_on_feedback,
                &mut frames,
            );
            for (promises, frame) in frames.drain(..) {
                let channel = self.chose_channel(promises);
                channel.get_tx().send(Frame::Stream(frame)).await;
            }
        }
    }

    pub async fn handle(&self, frame: ParticipantFrame) {
        info!("got a frame to handle");
        /*
        match frame {
            ParticipantFrame::OpenStream {
                sid,
                prio,
                promises,
            } => {
                if let Some(pid) = self.remote_pid {
                    let (msg_tx, msg_rx) = futures::channel::mpsc::unbounded::<InCommingMessage>();
                    let stream = IntStream::new(sid, prio, promises.clone(), msg_tx);

                    trace!(?self.streams, "-OPEN STREAM- going to modify streams");
                    self.streams.push(stream);
                    trace!(?self.streams, "-OPEN STREAM- did to modify streams");
                    info!("opened a stream");
                    if let Err(err) = rtrn_tx.send(RtrnMsg::OpendStream {
                        pid,
                        sid,
                        prio,
                        msg_rx,
                        promises,
                    }) {
                        error!(?err, "couldn't notify of opened stream");
                    }
                } else {
                    error!("called OpenStream before PartcipantID!");
                }
            },
            ParticipantFrame::CloseStream { sid } => {
                if let Some(pid) = self.remote_pid {
                    trace!(?self.streams, "-CLOSE STREAM- going to modify streams");
                    self.streams.retain(|stream| stream.sid() != sid);
                    trace!(?self.streams, "-CLOSE STREAM- did to modify streams");
                    info!("closed a stream");
                    if let Err(err) = rtrn_tx.send(RtrnMsg::ClosedStream { pid, sid }) {
                        error!(?err, "couldn't notify of closed stream");
                    }
                }
            },
        }*/
    }

    /// Endless task that will cover sending for Participant
    pub async fn run(&mut self) {
        let (incomming_sender, incomming_receiver) = mpsc::unbounded();
        futures::join!(self.read(), self.write());
    }

    pub fn chose_channel(&self,
        promises: u8,            /*  */
    ) -> &BChannel {
        for v in self.channels.values() {
            for c in v {
                return c;
            }
        }
        panic!("No Channel!");
    }
}

impl Scheduler {
    pub fn new(
        pid: Pid,
        metrics: Arc<Option<NetworkMetrics>>,
        sid_backup_per_participant: Arc<RwLock<HashMap<Pid, tlid::Pool<tlid::Checked<Sid>>>>>,
        token_pool: tlid::Pool<tlid::Wrapping<usize>>,
    ) -> Self {
        panic!("asd");
    }

    pub fn run(&mut self) { loop {} }
}
*/
