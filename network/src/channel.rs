use crate::{
    frames::Frame,
    types::{
        Cid, NetworkBuffer, Pid, Sid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2, VELOREN_MAGIC_NUMBER,
        VELOREN_NETWORK_VERSION,
    },
};
use async_std::{net::TcpStream, prelude::*, sync::RwLock};
use futures::{channel::mpsc, future::FutureExt, select, sink::SinkExt, stream::StreamExt};
use tracing::*;
//use futures::prelude::*;

pub(crate) struct Channel {
    cid: Cid,
    local_pid: Pid,
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
    const WRONG_NUMBER: &'static [u8] = "Handshake does not contain the magic number requiered by \
                                         veloren server.\nWe are not sure if you are a valid \
                                         veloren client.\nClosing the connection"
        .as_bytes();
    const WRONG_VERSION: &'static str = "Handshake does contain a correct magic number, but \
                                         invalid version.\nWe don't know how to communicate with \
                                         you.\nClosing the connection";

    pub fn new(cid: u64, local_pid: Pid) -> Self {
        Self {
            cid,
            local_pid,
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
        protocol: TcpStream,
        part_in_receiver: mpsc::UnboundedReceiver<Frame>,
        part_out_sender: mpsc::UnboundedSender<(Cid, Frame)>,
        configured_sender: mpsc::UnboundedSender<(Cid, Pid, Sid)>,
    ) {
        let (prot_in_sender, prot_in_receiver) = mpsc::unbounded::<Frame>();
        let (prot_out_sender, prot_out_receiver) = mpsc::unbounded::<Frame>();

        futures::join!(
            self.read(protocol.clone(), prot_in_sender),
            self.write(protocol, prot_out_receiver, part_in_receiver),
            self.frame_handler(
                prot_in_receiver,
                prot_out_sender,
                part_out_sender,
                configured_sender
            )
        );

        //return part_out_receiver;
    }

    pub async fn frame_handler(
        &self,
        mut frames: mpsc::UnboundedReceiver<Frame>,
        mut frame_sender: mpsc::UnboundedSender<Frame>,
        mut external_frame_sender: mpsc::UnboundedSender<(Cid, Frame)>,
        mut configured_sender: mpsc::UnboundedSender<(Cid, Pid, Sid)>,
    ) {
        const ERR_S: &str = "Got A Raw Message, these are usually Debug Messages indicating that \
                             something went wrong on network layer and connection will be closed";
        while let Some(frame) = frames.next().await {
            trace!(?frame, "recv frame");
            match frame {
                Frame::Handshake {
                    magic_number,
                    version,
                } => {
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
                    let stream_id_offset = if *self.send_state.read().await != ChannelState::Pid {
                        self.send_pid(&mut frame_sender).await;
                        STREAM_ID_OFFSET2
                    } else {
                        STREAM_ID_OFFSET1
                    };
                    info!(?pid, "this channel is now configured!");
                    configured_sender
                        .send((self.cid, pid, stream_id_offset))
                        .await
                        .unwrap();
                },
                Frame::Shutdown => {
                    info!("shutdown signal received");
                    *self.recv_state.write().await = ChannelState::Shutdown;
                },
                /* Sending RAW is only used for debug purposes in case someone write a
                 * new API against veloren Server! */
                Frame::Raw(bytes) => match std::str::from_utf8(bytes.as_slice()) {
                    Ok(string) => error!(?string, ERR_S),
                    _ => error!(?bytes, ERR_S),
                },
                _ => {
                    trace!("forward frame");
                    external_frame_sender.send((self.cid, frame)).await.unwrap();
                },
            }
        }
    }

    pub async fn read(
        &self,
        mut protocol: TcpStream,
        mut frame_handler: mpsc::UnboundedSender<Frame>,
    ) {
        let mut buffer = NetworkBuffer::new();
        loop {
            match protocol.read(buffer.get_write_slice(2048)).await {
                Ok(0) => {
                    debug!(?buffer, "shutdown of tcp channel detected");
                    frame_handler.send(Frame::Shutdown).await.unwrap();
                    break;
                },
                Ok(n) => {
                    buffer.actually_written(n);
                    trace!("incomming message with len: {}", n);
                    let slice = buffer.get_read_slice();
                    let mut cur = std::io::Cursor::new(slice);
                    let mut read_ok = 0;
                    while cur.position() < n as u64 {
                        let round_start = cur.position() as usize;
                        let r: Result<Frame, _> = bincode::deserialize_from(&mut cur);
                        match r {
                            Ok(frame) => {
                                frame_handler.send(frame).await.unwrap();
                                read_ok = cur.position() as usize;
                            },
                            Err(e) => {
                                // Probably we have to wait for moare data!
                                let first_bytes_of_msg =
                                    &slice[round_start..std::cmp::min(n, round_start + 16)];
                                debug!(
                                    ?buffer,
                                    ?e,
                                    ?n,
                                    ?round_start,
                                    ?first_bytes_of_msg,
                                    "message cant be parsed, probably because we need to wait for \
                                     more data"
                                );
                                break;
                            },
                        }
                    }
                    buffer.actually_read(read_ok);
                },
                Err(e) => panic!("{}", e),
            }
        }
    }

    pub async fn write(
        &self,
        mut protocol: TcpStream,
        mut internal_frame_receiver: mpsc::UnboundedReceiver<Frame>,
        mut external_frame_receiver: mpsc::UnboundedReceiver<Frame>,
    ) {
        while let Some(frame) = select! {
            next = internal_frame_receiver.next().fuse() => next,
            next = external_frame_receiver.next().fuse() => next,
        } {
            //dezerialize here as this is executed in a seperate thread PER channel.
            // Limites Throughput per single Receiver but stays in same thread (maybe as its
            // in a threadpool)
            trace!(?frame, "going to send frame via tcp");
            let data = bincode::serialize(&frame).unwrap();
            protocol.write_all(data.as_slice()).await.unwrap();
        }
    }

    async fn verify_handshake(
        &self,
        magic_number: String,
        version: [u32; 3],
        frame_sender: &mut mpsc::UnboundedSender<Frame>,
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
                magic_number: VELOREN_MAGIC_NUMBER.to_string(),
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
