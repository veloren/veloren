use crate::{
    metrics::NetworkMetrics,
    protocols::Protocols,
    types::{
        Cid, Frame, Pid, Sid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2, VELOREN_MAGIC_NUMBER,
        VELOREN_NETWORK_VERSION,
    },
};
use futures::{
    channel::{mpsc, oneshot},
    join,
    sink::SinkExt,
    stream::StreamExt,
    FutureExt,
};
use std::sync::Arc;
use tracing::*;

pub(crate) struct Channel {
    cid: Cid,
    remote_pid: Pid,
    to_wire_receiver: Option<mpsc::UnboundedReceiver<Frame>>,
    read_stop_receiver: Option<oneshot::Receiver<()>>,
}

impl Channel {
    pub fn new(
        cid: u64,
        remote_pid: Pid,
    ) -> (Self, mpsc::UnboundedSender<Frame>, oneshot::Sender<()>) {
        let (to_wire_sender, to_wire_receiver) = mpsc::unbounded::<Frame>();
        let (read_stop_sender, read_stop_receiver) = oneshot::channel();
        (
            Self {
                cid,
                remote_pid,
                to_wire_receiver: Some(to_wire_receiver),
                read_stop_receiver: Some(read_stop_receiver),
            },
            to_wire_sender,
            read_stop_sender,
        )
    }

    pub async fn run(
        mut self,
        protocol: Protocols,
        from_wire_sender: mpsc::UnboundedSender<(Cid, Frame)>,
    ) {
        let to_wire_receiver = self.to_wire_receiver.take().unwrap();
        let read_stop_receiver = self.read_stop_receiver.take().unwrap();

        trace!(?self.remote_pid, "start up channel");
        match protocol {
            Protocols::Tcp(tcp) => {
                futures::join!(
                    tcp.read(self.cid, from_wire_sender, read_stop_receiver),
                    tcp.write(self.cid, to_wire_receiver),
                );
            },
            Protocols::Udp(udp) => {
                futures::join!(
                    udp.read(self.cid, from_wire_sender, read_stop_receiver),
                    udp.write(self.cid, to_wire_receiver),
                );
            },
        }

        trace!(?self.remote_pid, "shut down channel");
    }
}

#[derive(Debug)]
pub(crate) struct Handshake {
    cid: Cid,
    local_pid: Pid,
    secret: u128,
    init_handshake: bool,
    metrics: Arc<NetworkMetrics>,
}

impl Handshake {
    #[cfg(debug_assertions)]
    const WRONG_NUMBER: &'static [u8] = "Handshake does not contain the magic number requiered by \
                                         veloren server.\nWe are not sure if you are a valid \
                                         veloren client.\nClosing the connection"
        .as_bytes();
    #[cfg(debug_assertions)]
    const WRONG_VERSION: &'static str = "Handshake does contain a correct magic number, but \
                                         invalid version.\nWe don't know how to communicate with \
                                         you.\nClosing the connection";

    pub fn new(
        cid: u64,
        local_pid: Pid,
        secret: u128,
        metrics: Arc<NetworkMetrics>,
        init_handshake: bool,
    ) -> Self {
        Self {
            cid,
            local_pid,
            secret,
            metrics,
            init_handshake,
        }
    }

    pub async fn setup(self, protocol: &Protocols) -> Result<(Pid, Sid, u128), ()> {
        let (to_wire_sender, to_wire_receiver) = mpsc::unbounded::<Frame>();
        let (from_wire_sender, from_wire_receiver) = mpsc::unbounded::<(Cid, Frame)>();
        let (read_stop_sender, read_stop_receiver) = oneshot::channel();

        let handler_future =
            self.frame_handler(from_wire_receiver, to_wire_sender, read_stop_sender);
        match protocol {
            Protocols::Tcp(tcp) => {
                (join! {
                    tcp.read(self.cid, from_wire_sender, read_stop_receiver),
                    tcp.write(self.cid, to_wire_receiver).fuse(),
                    handler_future,
                })
                .2
            },
            Protocols::Udp(udp) => {
                (join! {
                    udp.read(self.cid, from_wire_sender, read_stop_receiver),
                    udp.write(self.cid, to_wire_receiver),
                    handler_future,
                })
                .2
            },
        }
    }

    async fn frame_handler(
        &self,
        mut from_wire_receiver: mpsc::UnboundedReceiver<(Cid, Frame)>,
        mut to_wire_sender: mpsc::UnboundedSender<Frame>,
        _read_stop_sender: oneshot::Sender<()>,
    ) -> Result<(Pid, Sid, u128), ()> {
        const ERR_S: &str = "Got A Raw Message, these are usually Debug Messages indicating that \
                             something went wrong on network layer and connection will be closed";
        let mut pid_string = "".to_string();
        let cid_string = self.cid.to_string();

        if self.init_handshake {
            self.send_handshake(&mut to_wire_sender).await;
        }

        match from_wire_receiver.next().await {
            Some((
                _,
                Frame::Handshake {
                    magic_number,
                    version,
                },
            )) => {
                trace!(?magic_number, ?version, "recv handshake");
                self.metrics
                    .frames_in_total
                    .with_label_values(&["", &cid_string, "Handshake"])
                    .inc();
                if magic_number != VELOREN_MAGIC_NUMBER {
                    error!(?magic_number, "connection with invalid magic_number");
                    #[cfg(debug_assertions)]
                    {
                        self.metrics
                            .frames_out_total
                            .with_label_values(&["", &cid_string, "Raw"])
                            .inc();
                        debug!("sending client instructions before killing");
                        to_wire_sender
                            .send(Frame::Raw(Self::WRONG_NUMBER.to_vec()))
                            .await
                            .unwrap();
                        to_wire_sender.send(Frame::Shutdown).await.unwrap();
                    }
                    return Err(());
                }
                if version != VELOREN_NETWORK_VERSION {
                    error!(?version, "connection with wrong network version");
                    #[cfg(debug_assertions)]
                    {
                        debug!("sending client instructions before killing");
                        self.metrics
                            .frames_out_total
                            .with_label_values(&["", &cid_string, "Raw"])
                            .inc();
                        to_wire_sender
                            .send(Frame::Raw(
                                format!(
                                    "{} Our Version: {:?}\nYour Version: {:?}\nClosing the \
                                     connection",
                                    Self::WRONG_VERSION,
                                    VELOREN_NETWORK_VERSION,
                                    version,
                                )
                                .as_bytes()
                                .to_vec(),
                            ))
                            .await
                            .unwrap();
                        to_wire_sender.send(Frame::Shutdown {}).await.unwrap();
                    }
                    return Err(());
                }
                debug!("handshake completed");
                if self.init_handshake {
                    self.send_init(&mut to_wire_sender, &pid_string).await;
                } else {
                    self.send_handshake(&mut to_wire_sender).await;
                }
            },
            Some((_, Frame::Shutdown)) => {
                info!("shutdown signal received");
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, "Shutdown"])
                    .inc();
                return Err(());
            },
            Some((_, Frame::Raw(bytes))) => {
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, "Raw"])
                    .inc();
                match std::str::from_utf8(bytes.as_slice()) {
                    Ok(string) => error!(?string, ERR_S),
                    _ => error!(?bytes, ERR_S),
                }
                return Err(());
            },
            Some((_, frame)) => {
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, frame.get_string()])
                    .inc();
                return Err(());
            },
            None => return Err(()),
        };

        match from_wire_receiver.next().await {
            Some((_, Frame::Init { pid, secret })) => {
                debug!(?pid, "Participant send their ID");
                pid_string = pid.to_string();
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, "ParticipantId"])
                    .inc();
                let stream_id_offset = if self.init_handshake {
                    STREAM_ID_OFFSET1
                } else {
                    self.send_init(&mut to_wire_sender, &pid_string).await;
                    STREAM_ID_OFFSET2
                };
                info!(?pid, "this Handshake is now configured!");
                return Ok((pid, stream_id_offset, secret));
            },
            Some((_, Frame::Shutdown)) => {
                info!("shutdown signal received");
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, "Shutdown"])
                    .inc();
                return Err(());
            },
            Some((_, Frame::Raw(bytes))) => {
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, "Raw"])
                    .inc();
                match std::str::from_utf8(bytes.as_slice()) {
                    Ok(string) => error!(?string, ERR_S),
                    _ => error!(?bytes, ERR_S),
                }
                return Err(());
            },
            Some((_, frame)) => {
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, frame.get_string()])
                    .inc();
                return Err(());
            },
            None => return Err(()),
        };
    }

    async fn send_handshake(&self, to_wire_sender: &mut mpsc::UnboundedSender<Frame>) {
        self.metrics
            .frames_out_total
            .with_label_values(&["", &self.cid.to_string(), "Handshake"])
            .inc();
        to_wire_sender
            .send(Frame::Handshake {
                magic_number: VELOREN_MAGIC_NUMBER,
                version: VELOREN_NETWORK_VERSION,
            })
            .await
            .unwrap();
    }

    async fn send_init(&self, to_wire_sender: &mut mpsc::UnboundedSender<Frame>, pid_string: &str) {
        self.metrics
            .frames_out_total
            .with_label_values(&[pid_string, &self.cid.to_string(), "ParticipantId"])
            .inc();
        to_wire_sender
            .send(Frame::Init {
                pid: self.local_pid,
                secret: self.secret,
            })
            .await
            .unwrap();
    }
}
