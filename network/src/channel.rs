#[cfg(feature = "metrics")]
use crate::metrics::NetworkMetrics;
use crate::{
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
#[cfg(feature = "metrics")] use std::sync::Arc;
use tracing::*;

pub(crate) struct Channel {
    cid: Cid,
    c2w_frame_r: Option<mpsc::UnboundedReceiver<Frame>>,
    read_stop_receiver: Option<oneshot::Receiver<()>>,
}

impl Channel {
    pub fn new(cid: u64) -> (Self, mpsc::UnboundedSender<Frame>, oneshot::Sender<()>) {
        let (c2w_frame_s, c2w_frame_r) = mpsc::unbounded::<Frame>();
        let (read_stop_sender, read_stop_receiver) = oneshot::channel();
        (
            Self {
                cid,
                c2w_frame_r: Some(c2w_frame_r),
                read_stop_receiver: Some(read_stop_receiver),
            },
            c2w_frame_s,
            read_stop_sender,
        )
    }

    pub async fn run(
        mut self,
        protocol: Protocols,
        mut w2c_cid_frame_s: mpsc::UnboundedSender<(Cid, Frame)>,
        mut leftover_cid_frame: Vec<(Cid, Frame)>,
    ) {
        let c2w_frame_r = self.c2w_frame_r.take().unwrap();
        let read_stop_receiver = self.read_stop_receiver.take().unwrap();

        //reapply leftovers from handshake
        let cnt = leftover_cid_frame.len();
        trace!(?self.cid, ?cnt, "Reapplying leftovers");
        for cid_frame in leftover_cid_frame.drain(..) {
            w2c_cid_frame_s.send(cid_frame).await.unwrap();
        }
        trace!(?self.cid, ?cnt, "All leftovers reapplied");

        trace!(?self.cid, "Start up channel");
        match protocol {
            Protocols::Tcp(tcp) => {
                futures::join!(
                    tcp.read_from_wire(self.cid, &mut w2c_cid_frame_s, read_stop_receiver),
                    tcp.write_to_wire(self.cid, c2w_frame_r),
                );
            },
            Protocols::Udp(udp) => {
                futures::join!(
                    udp.read_from_wire(self.cid, &mut w2c_cid_frame_s, read_stop_receiver),
                    udp.write_to_wire(self.cid, c2w_frame_r),
                );
            },
        }

        trace!(?self.cid, "Shut down channel");
    }
}

#[derive(Debug)]
pub(crate) struct Handshake {
    cid: Cid,
    local_pid: Pid,
    secret: u128,
    init_handshake: bool,
    #[cfg(feature = "metrics")]
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
        #[cfg(feature = "metrics")] metrics: Arc<NetworkMetrics>,
        init_handshake: bool,
    ) -> Self {
        Self {
            cid,
            local_pid,
            secret,
            #[cfg(feature = "metrics")]
            metrics,
            init_handshake,
        }
    }

    pub async fn setup(
        self,
        protocol: &Protocols,
    ) -> Result<(Pid, Sid, u128, Vec<(Cid, Frame)>), ()> {
        let (c2w_frame_s, c2w_frame_r) = mpsc::unbounded::<Frame>();
        let (mut w2c_cid_frame_s, mut w2c_cid_frame_r) = mpsc::unbounded::<(Cid, Frame)>();

        let (read_stop_sender, read_stop_receiver) = oneshot::channel();
        let handler_future =
            self.frame_handler(&mut w2c_cid_frame_r, c2w_frame_s, read_stop_sender);
        let res = match protocol {
            Protocols::Tcp(tcp) => {
                (join! {
                    tcp.read_from_wire(self.cid, &mut w2c_cid_frame_s, read_stop_receiver),
                    tcp.write_to_wire(self.cid, c2w_frame_r).fuse(),
                    handler_future,
                })
                .2
            },
            Protocols::Udp(udp) => {
                (join! {
                    udp.read_from_wire(self.cid, &mut w2c_cid_frame_s, read_stop_receiver),
                    udp.write_to_wire(self.cid, c2w_frame_r),
                    handler_future,
                })
                .2
            },
        };

        match res {
            Ok(res) => {
                let mut leftover_frames = vec![];
                while let Ok(Some(cid_frame)) = w2c_cid_frame_r.try_next() {
                    leftover_frames.push(cid_frame);
                }
                let cnt = leftover_frames.len();
                if cnt > 0 {
                    debug!(?self.cid, ?cnt, "Some additional frames got already transfered, piping them to the bparticipant as leftover_frames");
                }
                Ok((res.0, res.1, res.2, leftover_frames))
            },
            Err(()) => Err(()),
        }
    }

    async fn frame_handler(
        &self,
        w2c_cid_frame_r: &mut mpsc::UnboundedReceiver<(Cid, Frame)>,
        mut c2w_frame_s: mpsc::UnboundedSender<Frame>,
        read_stop_sender: oneshot::Sender<()>,
    ) -> Result<(Pid, Sid, u128), ()> {
        const ERR_S: &str = "Got A Raw Message, these are usually Debug Messages indicating that \
                             something went wrong on network layer and connection will be closed";
        #[cfg(feature = "metrics")]
        let cid_string = self.cid.to_string();

        if self.init_handshake {
            self.send_handshake(&mut c2w_frame_s).await;
        }

        let frame = w2c_cid_frame_r.next().await.map(|(_cid, frame)| frame);
        #[cfg(feature = "metrics")]
        {
            if let Some(ref frame) = frame {
                self.metrics
                    .frames_in_total
                    .with_label_values(&["", &cid_string, &frame.get_string()])
                    .inc();
            }
        }
        let r = match frame {
            Some(Frame::Handshake {
                magic_number,
                version,
            }) => {
                trace!(?magic_number, ?version, "Recv handshake");
                if magic_number != VELOREN_MAGIC_NUMBER {
                    error!(?magic_number, "Connection with invalid magic_number");
                    #[cfg(debug_assertions)]
                    self.send_raw_and_shutdown(&mut c2w_frame_s, Self::WRONG_NUMBER.to_vec())
                        .await;
                    Err(())
                } else if version != VELOREN_NETWORK_VERSION {
                    error!(?version, "Connection with wrong network version");
                    #[cfg(debug_assertions)]
                    self.send_raw_and_shutdown(
                        &mut c2w_frame_s,
                        format!(
                            "{} Our Version: {:?}\nYour Version: {:?}\nClosing the connection",
                            Self::WRONG_VERSION,
                            VELOREN_NETWORK_VERSION,
                            version,
                        )
                        .as_bytes()
                        .to_vec(),
                    )
                    .await;
                    Err(())
                } else {
                    debug!("Handshake completed");
                    if self.init_handshake {
                        self.send_init(&mut c2w_frame_s, "").await;
                    } else {
                        self.send_handshake(&mut c2w_frame_s).await;
                    }
                    Ok(())
                }
            },
            Some(Frame::Shutdown) => {
                info!("Shutdown signal received");
                Err(())
            },
            Some(Frame::Raw(bytes)) => {
                match std::str::from_utf8(bytes.as_slice()) {
                    Ok(string) => error!(?string, ERR_S),
                    _ => error!(?bytes, ERR_S),
                }
                Err(())
            },
            Some(_) => Err(()),
            None => Err(()),
        };
        if let Err(()) = r {
            if let Err(e) = read_stop_sender.send(()) {
                trace!(
                    ?e,
                    "couldn't stop protocol, probably it encountered a Protocol Stop and closed \
                     itself already, which is fine"
                );
            }
            return Err(());
        }

        let frame = w2c_cid_frame_r.next().await.map(|(_cid, frame)| frame);
        let r = match frame {
            Some(Frame::Init { pid, secret }) => {
                debug!(?pid, "Participant send their ID");
                let pid_string = pid.to_string();
                #[cfg(feature = "metrics")]
                self.metrics
                    .frames_in_total
                    .with_label_values(&[&pid_string, &cid_string, "ParticipantId"])
                    .inc();
                let stream_id_offset = if self.init_handshake {
                    STREAM_ID_OFFSET1
                } else {
                    self.send_init(&mut c2w_frame_s, &pid_string).await;
                    STREAM_ID_OFFSET2
                };
                info!(?pid, "This Handshake is now configured!");
                Ok((pid, stream_id_offset, secret))
            },
            Some(frame) => {
                #[cfg(feature = "metrics")]
                self.metrics
                    .frames_in_total
                    .with_label_values(&["", &cid_string, frame.get_string()])
                    .inc();
                match frame {
                    Frame::Shutdown => info!("Shutdown signal received"),
                    Frame::Raw(bytes) => match std::str::from_utf8(bytes.as_slice()) {
                        Ok(string) => error!(?string, ERR_S),
                        _ => error!(?bytes, ERR_S),
                    },
                    _ => (),
                }
                Err(())
            },
            None => Err(()),
        };
        if r.is_err() {
            if let Err(e) = read_stop_sender.send(()) {
                trace!(
                    ?e,
                    "couldn't stop protocol, probably it encountered a Protocol Stop and closed \
                     itself already, which is fine"
                );
            }
        }
        r
    }

    async fn send_handshake(&self, c2w_frame_s: &mut mpsc::UnboundedSender<Frame>) {
        #[cfg(feature = "metrics")]
        self.metrics
            .frames_out_total
            .with_label_values(&["", &self.cid.to_string(), "Handshake"])
            .inc();
        c2w_frame_s
            .send(Frame::Handshake {
                magic_number: VELOREN_MAGIC_NUMBER,
                version: VELOREN_NETWORK_VERSION,
            })
            .await
            .unwrap();
    }

    async fn send_init(
        &self,
        c2w_frame_s: &mut mpsc::UnboundedSender<Frame>,
        #[cfg(feature = "metrics")] pid_string: &str,
        #[cfg(not(feature = "metrics"))] _pid_string: &str,
    ) {
        #[cfg(feature = "metrics")]
        self.metrics
            .frames_out_total
            .with_label_values(&[pid_string, &self.cid.to_string(), "ParticipantId"])
            .inc();
        c2w_frame_s
            .send(Frame::Init {
                pid: self.local_pid,
                secret: self.secret,
            })
            .await
            .unwrap();
    }

    #[cfg(debug_assertions)]
    async fn send_raw_and_shutdown(
        &self,
        c2w_frame_s: &mut mpsc::UnboundedSender<Frame>,
        data: Vec<u8>,
    ) {
        debug!("Sending client instructions before killing");
        #[cfg(feature = "metrics")]
        {
            let cid_string = self.cid.to_string();
            self.metrics
                .frames_out_total
                .with_label_values(&["", &cid_string, "Raw"])
                .inc();
            self.metrics
                .frames_out_total
                .with_label_values(&["", &cid_string, "Shutdown"])
                .inc();
        }
        c2w_frame_s.send(Frame::Raw(data)).await.unwrap();
        c2w_frame_s.send(Frame::Shutdown).await.unwrap();
    }
}
