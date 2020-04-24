use crate::{
    metrics::NetworkMetrics,
    protocols::Protocols,
    types::{
        Cid, Frame, Pid, Sid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2, VELOREN_MAGIC_NUMBER,
        VELOREN_NETWORK_VERSION,
    },
};
use async_std::sync::RwLock;
use futures::{
    channel::{mpsc, oneshot},
    sink::SinkExt,
    stream::StreamExt,
};
use std::sync::Arc;
use tracing::*;
//use futures::prelude::*;

pub(crate) struct Channel {
    cid: Cid,
    local_pid: Pid,
    metrics: Arc<NetworkMetrics>,
    remote_pid: RwLock<Option<Pid>>,
    send_state: RwLock<ChannelState>,
    recv_state: RwLock<ChannelState>,
}

#[derive(Debug, PartialEq)]
enum ChannelState {
    None,
    Handshake,
    Pid,
    Shutdown,
}

impl Channel {
    #[cfg(debug_assertions)]
    const WRONG_NUMBER: &'static [u8] = "Handshake does not contain the magic number requiered by \
                                         veloren server.\nWe are not sure if you are a valid \
                                         veloren client.\nClosing the connection"
        .as_bytes();
    #[cfg(debug_assertions)]
    const WRONG_VERSION: &'static str = "Handshake does contain a correct magic number, but \
                                         invalid version.\nWe don't know how to communicate with \
                                         you.\nClosing the connection";

    pub fn new(cid: u64, local_pid: Pid, metrics: Arc<NetworkMetrics>) -> Self {
        Self {
            cid,
            local_pid,
            metrics,
            remote_pid: RwLock::new(None),
            send_state: RwLock::new(ChannelState::None),
            recv_state: RwLock::new(ChannelState::None),
        }
    }

    /// (prot|part)_(in|out)_(sender|receiver)
    /// prot: TO/FROM PROTOCOL = TCP
    /// part: TO/FROM PARTICIPANT
    /// in: FROM
    /// out: TO
    /// sender: mpsc::Sender
    /// receiver: mpsc::Receiver
    pub async fn run(
        self,
        protocol: Protocols,
        part_in_receiver: mpsc::UnboundedReceiver<Frame>,
        part_out_sender: mpsc::UnboundedSender<(Cid, Frame)>,
        configured_sender: mpsc::UnboundedSender<(Cid, Pid, Sid, oneshot::Sender<()>)>,
    ) {
        let (prot_in_sender, prot_in_receiver) = mpsc::unbounded::<Frame>();
        let (prot_out_sender, prot_out_receiver) = mpsc::unbounded::<Frame>();

        let handler_future = self.frame_handler(
            prot_in_receiver,
            prot_out_sender,
            part_out_sender,
            configured_sender,
        );
        match protocol {
            Protocols::Tcp(tcp) => {
                futures::join!(
                    tcp.read(prot_in_sender),
                    tcp.write(prot_out_receiver, part_in_receiver),
                    handler_future,
                );
            },
            Protocols::Udp(udp) => {
                futures::join!(
                    udp.read(prot_in_sender),
                    udp.write(prot_out_receiver, part_in_receiver),
                    handler_future,
                );
            },
        }

        //return part_out_receiver;
    }

    pub async fn frame_handler(
        &self,
        mut frames: mpsc::UnboundedReceiver<Frame>,
        mut frame_sender: mpsc::UnboundedSender<Frame>,
        mut external_frame_sender: mpsc::UnboundedSender<(Cid, Frame)>,
        mut configured_sender: mpsc::UnboundedSender<(Cid, Pid, Sid, oneshot::Sender<()>)>,
    ) {
        const ERR_S: &str = "Got A Raw Message, these are usually Debug Messages indicating that \
                             something went wrong on network layer and connection will be closed";
        let mut pid_string = "".to_string();
        let cid_string = self.cid.to_string();
        while let Some(frame) = frames.next().await {
            match frame {
                Frame::Handshake {
                    magic_number,
                    version,
                } => {
                    trace!(?magic_number, ?version, "recv handshake");
                    self.metrics
                        .frames_in_total
                        .with_label_values(&["", &cid_string, "Handshake"])
                        .inc();
                    if self
                        .verify_handshake(magic_number, version, &mut frame_sender)
                        .await
                        .is_ok()
                    {
                        debug!("handshake completed");
                        *self.recv_state.write().await = ChannelState::Handshake;
                        if *self.send_state.read().await == ChannelState::Handshake {
                            self.send_pid(&mut frame_sender).await;
                        } else {
                            self.send_handshake(&mut frame_sender).await;
                        }
                    };
                },
                Frame::ParticipantId { pid } => {
                    if self.remote_pid.read().await.is_some() {
                        error!(?pid, "invalid message, cant change participantId");
                        return;
                    }
                    *self.remote_pid.write().await = Some(pid);
                    *self.recv_state.write().await = ChannelState::Pid;
                    debug!(?pid, "Participant send their ID");
                    let pid_u128: u128 = pid.into();
                    pid_string = pid_u128.to_string();
                    self.metrics
                        .frames_in_total
                        .with_label_values(&[&pid_string, &cid_string, "ParticipantId"])
                        .inc();
                    let stream_id_offset = if *self.send_state.read().await != ChannelState::Pid {
                        self.send_pid(&mut frame_sender).await;
                        STREAM_ID_OFFSET2
                    } else {
                        STREAM_ID_OFFSET1
                    };
                    info!(?pid, "this channel is now configured!");
                    let pid_u128: u128 = pid.into();
                    self.metrics
                        .channels_connected_total
                        .with_label_values(&[&pid_u128.to_string()])
                        .inc();
                    let (sender, receiver) = oneshot::channel();
                    configured_sender
                        .send((self.cid, pid, stream_id_offset, sender))
                        .await
                        .unwrap();
                    receiver.await.unwrap();
                    //TODO: this is sync anyway, because we need to wait. so find a better way than
                    // there channels like direct method call... otherwise a
                    // frame might jump in before its officially configured yet
                    debug!(
                        "STOP, if you read this, fix this error. make this a function isntead a \
                         channel here"
                    );
                },
                Frame::Shutdown => {
                    info!("shutdown signal received");
                    *self.recv_state.write().await = ChannelState::Shutdown;
                    self.metrics
                        .channels_disconnected_total
                        .with_label_values(&[&pid_string])
                        .inc();
                    self.metrics
                        .frames_in_total
                        .with_label_values(&[&pid_string, &cid_string, "Shutdown"])
                        .inc();
                },
                /* Sending RAW is only used for debug purposes in case someone write a
                 * new API against veloren Server! */
                Frame::Raw(bytes) => {
                    self.metrics
                        .frames_in_total
                        .with_label_values(&[&pid_string, &cid_string, "Raw"])
                        .inc();
                    match std::str::from_utf8(bytes.as_slice()) {
                        Ok(string) => error!(?string, ERR_S),
                        _ => error!(?bytes, ERR_S),
                    }
                },
                _ => {
                    trace!("forward frame");
                    external_frame_sender.send((self.cid, frame)).await.unwrap();
                },
            }
        }
    }

    async fn verify_handshake(
        &self,
        magic_number: [u8; 7],
        version: [u32; 3],
        #[cfg(debug_assertions)] frame_sender: &mut mpsc::UnboundedSender<Frame>,
        #[cfg(not(debug_assertions))] _: &mut mpsc::UnboundedSender<Frame>,
    ) -> Result<(), ()> {
        if magic_number != VELOREN_MAGIC_NUMBER {
            error!(?magic_number, "connection with invalid magic_number");
            #[cfg(debug_assertions)]
            {
                debug!("sending client instructions before killing");
                frame_sender
                    .send(Frame::Raw(Self::WRONG_NUMBER.to_vec()))
                    .await
                    .unwrap();
                frame_sender.send(Frame::Shutdown).await.unwrap();
                *self.send_state.write().await = ChannelState::Shutdown;
            }
            return Err(());
        }
        if version != VELOREN_NETWORK_VERSION {
            error!(?version, "connection with wrong network version");
            #[cfg(debug_assertions)]
            {
                debug!("sending client instructions before killing");
                frame_sender
                    .send(Frame::Raw(
                        format!(
                            "{} Our Version: {:?}\nYour Version: {:?}\nClosing the connection",
                            Self::WRONG_VERSION,
                            VELOREN_NETWORK_VERSION,
                            version,
                        )
                        .as_bytes()
                        .to_vec(),
                    ))
                    .await
                    .unwrap();
                frame_sender.send(Frame::Shutdown {}).await.unwrap();
                *self.send_state.write().await = ChannelState::Shutdown;
            }
            return Err(());
        }
        Ok(())
    }

    pub(crate) async fn send_handshake(&self, part_in_sender: &mut mpsc::UnboundedSender<Frame>) {
        part_in_sender
            .send(Frame::Handshake {
                magic_number: VELOREN_MAGIC_NUMBER,
                version: VELOREN_NETWORK_VERSION,
            })
            .await
            .unwrap();
        *self.send_state.write().await = ChannelState::Handshake;
    }

    pub(crate) async fn send_pid(&self, part_in_sender: &mut mpsc::UnboundedSender<Frame>) {
        part_in_sender
            .send(Frame::ParticipantId {
                pid: self.local_pid,
            })
            .await
            .unwrap();
        *self.send_state.write().await = ChannelState::Pid;
    }
    /*
    pub async fn run(&mut self) {
        //let (incomming_sender, incomming_receiver) = mpsc::unbounded();
        futures::join!(self.listen_manager(), self.send_outgoing());
    }

    pub async fn listen_manager(&self) {
        let (mut listen_sender, mut listen_receiver) = mpsc::unbounded::<Address>();

        while self.closed.load(Ordering::Relaxed) {
            while let Some(address) = listen_receiver.next().await {
                let (end_sender, end_receiver) = oneshot::channel::<()>();
                task::spawn(channel_creator(address, end_receiver));
            }
        }
    }

    pub async fn send_outgoing(&self) {
        //let prios = prios::PrioManager;
        while self.closed.load(Ordering::Relaxed) {

        }
    }*/
}
