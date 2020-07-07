use crate::{
    metrics::{CidFrameCache, NetworkMetrics},
    types::{Cid, Frame, Mid, Pid, Sid},
};
use async_std::{
    net::{TcpStream, UdpSocket},
    prelude::*,
};
use futures::{
    channel::{mpsc, oneshot},
    future::FutureExt,
    lock::Mutex,
    select,
    sink::SinkExt,
    stream::StreamExt,
};
use std::{net::SocketAddr, sync::Arc};
use tracing::*;

// Reserving bytes 0, 10, 13 as i have enough space and want to make it easy to
// detect a invalid client, e.g. sending an empty line would make 10 first char
// const FRAME_RESERVED_1: u8 = 0;
const FRAME_HANDSHAKE: u8 = 1;
const FRAME_INIT: u8 = 2;
const FRAME_SHUTDOWN: u8 = 3;
const FRAME_OPEN_STREAM: u8 = 4;
const FRAME_CLOSE_STREAM: u8 = 5;
const FRAME_DATA_HEADER: u8 = 6;
const FRAME_DATA: u8 = 7;
const FRAME_RAW: u8 = 8;
//const FRAME_RESERVED_2: u8 = 10;
//const FRAME_RESERVED_3: u8 = 13;

#[derive(Debug)]
pub(crate) enum Protocols {
    Tcp(TcpProtocol),
    Udp(UdpProtocol),
    //Mpsc(MpscChannel),
}

#[derive(Debug)]
pub(crate) struct TcpProtocol {
    stream: TcpStream,
    metrics: Arc<NetworkMetrics>,
}

#[derive(Debug)]
pub(crate) struct UdpProtocol {
    socket: Arc<UdpSocket>,
    remote_addr: SocketAddr,
    metrics: Arc<NetworkMetrics>,
    data_in: Mutex<mpsc::UnboundedReceiver<Vec<u8>>>,
}

//TODO: PERFORMACE: Use BufWriter and BufReader from std::io!
impl TcpProtocol {
    pub(crate) fn new(stream: TcpStream, metrics: Arc<NetworkMetrics>) -> Self {
        Self { stream, metrics }
    }

    /// read_except and if it fails, close the protocol
    async fn read_except_or_close(
        cid: Cid,
        mut stream: &TcpStream,
        mut bytes: &mut [u8],
        w2c_cid_frame_s: &mut mpsc::UnboundedSender<(Cid, Frame)>,
    ) {
        if let Err(e) = stream.read_exact(&mut bytes).await {
            warn!(
                ?e,
                "closing tcp protocol due to read error, sending close frame to gracefully \
                 shutdown"
            );
            w2c_cid_frame_s
                .send((cid, Frame::Shutdown))
                .await
                .expect("Channel or Participant seems no longer to exist to be Shutdown");
        }
    }

    pub async fn read_from_wire(
        &self,
        cid: Cid,
        w2c_cid_frame_s: &mut mpsc::UnboundedSender<(Cid, Frame)>,
        end_receiver: oneshot::Receiver<()>,
    ) {
        trace!("starting up tcp read()");
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_in_total.clone(), cid);
        let throughput_cache = self
            .metrics
            .wire_in_throughput
            .with_label_values(&[&cid.to_string()]);
        let mut stream = self.stream.clone();
        let mut end_receiver = end_receiver.fuse();

        loop {
            let mut bytes = [0u8; 1];
            let r = select! {
                    r = stream.read_exact(&mut bytes).fuse() => r,
                    _ = end_receiver => break,
            };
            if r.is_err() {
                info!("tcp stream closed, shutting down read");
                break;
            }
            let frame_no = bytes[0];
            let frame = match frame_no {
                FRAME_HANDSHAKE => {
                    let mut bytes = [0u8; 19];
                    Self::read_except_or_close(cid, &stream, &mut bytes, w2c_cid_frame_s).await;
                    let magic_number = [
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    ];
                    Frame::Handshake {
                        magic_number,
                        version: [
                            u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
                            u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
                            u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
                        ],
                    }
                },
                FRAME_INIT => {
                    let mut bytes = [0u8; 16];
                    Self::read_except_or_close(cid, &stream, &mut bytes, w2c_cid_frame_s).await;
                    let pid = Pid::from_le_bytes(bytes);
                    stream.read_exact(&mut bytes).await.unwrap();
                    let secret = u128::from_le_bytes(bytes);
                    Frame::Init { pid, secret }
                },
                FRAME_SHUTDOWN => Frame::Shutdown,
                FRAME_OPEN_STREAM => {
                    let mut bytes = [0u8; 10];
                    Self::read_except_or_close(cid, &stream, &mut bytes, w2c_cid_frame_s).await;
                    let sid = Sid::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    let prio = bytes[8];
                    let promises = bytes[9];
                    Frame::OpenStream {
                        sid,
                        prio,
                        promises,
                    }
                },
                FRAME_CLOSE_STREAM => {
                    let mut bytes = [0u8; 8];
                    Self::read_except_or_close(cid, &stream, &mut bytes, w2c_cid_frame_s).await;
                    let sid = Sid::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    Frame::CloseStream { sid }
                },
                FRAME_DATA_HEADER => {
                    let mut bytes = [0u8; 24];
                    Self::read_except_or_close(cid, &stream, &mut bytes, w2c_cid_frame_s).await;
                    let mid = Mid::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    let sid = Sid::from_le_bytes([
                        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                        bytes[15],
                    ]);
                    let length = u64::from_le_bytes([
                        bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21],
                        bytes[22], bytes[23],
                    ]);
                    Frame::DataHeader { mid, sid, length }
                },
                FRAME_DATA => {
                    let mut bytes = [0u8; 18];
                    Self::read_except_or_close(cid, &stream, &mut bytes, w2c_cid_frame_s).await;
                    let mid = Mid::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    let start = u64::from_le_bytes([
                        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                        bytes[15],
                    ]);
                    let length = u16::from_le_bytes([bytes[16], bytes[17]]);
                    let mut cdata = vec![0; length as usize];
                    throughput_cache.inc_by(length as i64);
                    Self::read_except_or_close(cid, &stream, &mut cdata, w2c_cid_frame_s).await;
                    let data = lz4_compress::decompress(&cdata).unwrap();
                    Frame::Data { mid, start, data }
                },
                FRAME_RAW => {
                    let mut bytes = [0u8; 2];
                    Self::read_except_or_close(cid, &stream, &mut bytes, w2c_cid_frame_s).await;
                    let length = u16::from_le_bytes([bytes[0], bytes[1]]);
                    let mut data = vec![0; length as usize];
                    Self::read_except_or_close(cid, &stream, &mut data, w2c_cid_frame_s).await;
                    Frame::Raw(data)
                },
                _ => {
                    // report a RAW frame, but cannot rely on the next 2 bytes to be a size.
                    // guessing 256 bytes, which might help to sort down issues
                    let mut data = vec![0; 256];
                    Self::read_except_or_close(cid, &stream, &mut data, w2c_cid_frame_s).await;
                    Frame::Raw(data)
                },
            };
            metrics_cache.with_label_values(&frame).inc();
            w2c_cid_frame_s
                .send((cid, frame))
                .await
                .expect("Channel or Participant seems no longer to exist");
        }
        trace!("shutting down tcp read()");
    }

    /// read_except and if it fails, close the protocol
    async fn write_or_close(
        stream: &mut TcpStream,
        bytes: &[u8],
        to_wire_receiver: &mut mpsc::UnboundedReceiver<Frame>,
    ) -> bool {
        match stream.write_all(&bytes).await {
            Err(e) => {
                warn!(
                    ?e,
                    "got an error writing to tcp, going to close this channel"
                );
                to_wire_receiver.close();
                true
            },
            _ => false,
        }
    }

    //dezerialize here as this is executed in a seperate thread PER channel.
    // Limites Throughput per single Receiver but stays in same thread (maybe as its
    // in a threadpool) for TCP, UDP and MPSC
    pub async fn write_to_wire(&self, cid: Cid, mut c2w_frame_r: mpsc::UnboundedReceiver<Frame>) {
        trace!("starting up tcp write()");
        let mut stream = self.stream.clone();
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_out_total.clone(), cid);
        let throughput_cache = self
            .metrics
            .wire_out_throughput
            .with_label_values(&[&cid.to_string()]);
        while let Some(frame) = c2w_frame_r.next().await {
            metrics_cache.with_label_values(&frame).inc();
            if match frame {
                Frame::Handshake {
                    magic_number,
                    version,
                } => {
                    Self::write_or_close(
                        &mut stream,
                        &FRAME_HANDSHAKE.to_be_bytes(),
                        &mut c2w_frame_r,
                    )
                    .await
                        || Self::write_or_close(&mut stream, &magic_number, &mut c2w_frame_r).await
                        || Self::write_or_close(
                            &mut stream,
                            &version[0].to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                        || Self::write_or_close(
                            &mut stream,
                            &version[1].to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                        || Self::write_or_close(
                            &mut stream,
                            &version[2].to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                },
                Frame::Init { pid, secret } => {
                    Self::write_or_close(&mut stream, &FRAME_INIT.to_be_bytes(), &mut c2w_frame_r)
                        .await
                        || Self::write_or_close(&mut stream, &pid.to_le_bytes(), &mut c2w_frame_r)
                            .await
                        || Self::write_or_close(
                            &mut stream,
                            &secret.to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                },
                Frame::Shutdown => {
                    Self::write_or_close(
                        &mut stream,
                        &FRAME_SHUTDOWN.to_be_bytes(),
                        &mut c2w_frame_r,
                    )
                    .await
                },
                Frame::OpenStream {
                    sid,
                    prio,
                    promises,
                } => {
                    Self::write_or_close(
                        &mut stream,
                        &FRAME_OPEN_STREAM.to_be_bytes(),
                        &mut c2w_frame_r,
                    )
                    .await
                        || Self::write_or_close(&mut stream, &sid.to_le_bytes(), &mut c2w_frame_r)
                            .await
                        || Self::write_or_close(&mut stream, &prio.to_le_bytes(), &mut c2w_frame_r)
                            .await
                        || Self::write_or_close(
                            &mut stream,
                            &promises.to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                },
                Frame::CloseStream { sid } => {
                    Self::write_or_close(
                        &mut stream,
                        &FRAME_CLOSE_STREAM.to_be_bytes(),
                        &mut c2w_frame_r,
                    )
                    .await
                        || Self::write_or_close(&mut stream, &sid.to_le_bytes(), &mut c2w_frame_r)
                            .await
                },
                Frame::DataHeader { mid, sid, length } => {
                    Self::write_or_close(
                        &mut stream,
                        &FRAME_DATA_HEADER.to_be_bytes(),
                        &mut c2w_frame_r,
                    )
                    .await
                        || Self::write_or_close(&mut stream, &mid.to_le_bytes(), &mut c2w_frame_r)
                            .await
                        || Self::write_or_close(&mut stream, &sid.to_le_bytes(), &mut c2w_frame_r)
                            .await
                        || Self::write_or_close(
                            &mut stream,
                            &length.to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                },
                Frame::Data { mid, start, data } => {
                    throughput_cache.inc_by(data.len() as i64);
                    let cdata = lz4_compress::compress(&data);
                    Self::write_or_close(&mut stream, &FRAME_DATA.to_be_bytes(), &mut c2w_frame_r)
                        .await
                        || Self::write_or_close(&mut stream, &mid.to_le_bytes(), &mut c2w_frame_r)
                            .await
                        || Self::write_or_close(&mut stream, &start.to_le_bytes(), &mut c2w_frame_r)
                            .await
                        || Self::write_or_close(
                            &mut stream,
                            &(cdata.len() as u16).to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                        || Self::write_or_close(&mut stream, &cdata, &mut c2w_frame_r).await
                },
                Frame::Raw(data) => {
                    Self::write_or_close(&mut stream, &FRAME_RAW.to_be_bytes(), &mut c2w_frame_r)
                        .await
                        || Self::write_or_close(
                            &mut stream,
                            &(data.len() as u16).to_le_bytes(),
                            &mut c2w_frame_r,
                        )
                        .await
                        || Self::write_or_close(&mut stream, &data, &mut c2w_frame_r).await
                },
            } {
                //failure
                return;
            }
        }
        trace!("shutting down tcp write()");
    }
}

impl UdpProtocol {
    pub(crate) fn new(
        socket: Arc<UdpSocket>,
        remote_addr: SocketAddr,
        metrics: Arc<NetworkMetrics>,
        data_in: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Self {
        Self {
            socket,
            remote_addr,
            metrics,
            data_in: Mutex::new(data_in),
        }
    }

    pub async fn read_from_wire(
        &self,
        cid: Cid,
        w2c_cid_frame_s: &mut mpsc::UnboundedSender<(Cid, Frame)>,
        end_receiver: oneshot::Receiver<()>,
    ) {
        trace!("starting up udp read()");
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_in_total.clone(), cid);
        let throughput_cache = self
            .metrics
            .wire_in_throughput
            .with_label_values(&[&cid.to_string()]);
        let mut data_in = self.data_in.lock().await;
        let mut end_receiver = end_receiver.fuse();
        while let Some(bytes) = select! {
            r = data_in.next().fuse() => r,
            _ = end_receiver => None,
        } {
            trace!("got raw UDP message with len: {}", bytes.len());
            let frame_no = bytes[0];
            let frame = match frame_no {
                FRAME_HANDSHAKE => {
                    let bytes = &bytes[1..20];
                    let magic_number = [
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                    ];
                    Frame::Handshake {
                        magic_number,
                        version: [
                            u32::from_le_bytes([bytes[7], bytes[8], bytes[9], bytes[10]]),
                            u32::from_le_bytes([bytes[11], bytes[12], bytes[13], bytes[14]]),
                            u32::from_le_bytes([bytes[15], bytes[16], bytes[17], bytes[18]]),
                        ],
                    }
                },
                FRAME_INIT => {
                    let pid = Pid::from_le_bytes([
                        bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                        bytes[15], bytes[16],
                    ]);
                    let secret = u128::from_le_bytes([
                        bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22],
                        bytes[23], bytes[24], bytes[25], bytes[26], bytes[27], bytes[28],
                        bytes[29], bytes[30], bytes[31], bytes[32],
                    ]);
                    Frame::Init { pid, secret }
                },
                FRAME_SHUTDOWN => Frame::Shutdown,
                FRAME_OPEN_STREAM => {
                    let bytes = &bytes[1..11];
                    let sid = Sid::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    let prio = bytes[8];
                    let promises = bytes[9];
                    Frame::OpenStream {
                        sid,
                        prio,
                        promises,
                    }
                },
                FRAME_CLOSE_STREAM => {
                    let bytes = &bytes[1..9];
                    let sid = Sid::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    Frame::CloseStream { sid }
                },
                FRAME_DATA_HEADER => {
                    let bytes = &bytes[1..25];
                    let mid = Mid::from_le_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
                        bytes[7],
                    ]);
                    let sid = Sid::from_le_bytes([
                        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                        bytes[15],
                    ]);
                    let length = u64::from_le_bytes([
                        bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21],
                        bytes[22], bytes[23],
                    ]);
                    Frame::DataHeader { mid, sid, length }
                },
                FRAME_DATA => {
                    let mid = Mid::from_le_bytes([
                        bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                        bytes[8],
                    ]);
                    let start = u64::from_le_bytes([
                        bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
                        bytes[16],
                    ]);
                    let length = u16::from_le_bytes([bytes[17], bytes[18]]);
                    let mut data = vec![0; length as usize];
                    throughput_cache.inc_by(length as i64);
                    data.copy_from_slice(&bytes[19..]);
                    Frame::Data { mid, start, data }
                },
                FRAME_RAW => {
                    let length = u16::from_le_bytes([bytes[1], bytes[2]]);
                    let mut data = vec![0; length as usize];
                    data.copy_from_slice(&bytes[3..]);
                    Frame::Raw(data)
                },
                _ => Frame::Raw(bytes),
            };
            metrics_cache.with_label_values(&frame).inc();
            w2c_cid_frame_s.send((cid, frame)).await.unwrap();
        }
        trace!("shutting down udp read()");
    }

    pub async fn write_to_wire(&self, cid: Cid, mut c2w_frame_r: mpsc::UnboundedReceiver<Frame>) {
        trace!("starting up udp write()");
        let mut buffer = [0u8; 2000];
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_out_total.clone(), cid);
        let throughput_cache = self
            .metrics
            .wire_out_throughput
            .with_label_values(&[&cid.to_string()]);
        while let Some(frame) = c2w_frame_r.next().await {
            metrics_cache.with_label_values(&frame).inc();
            let len = match frame {
                Frame::Handshake {
                    magic_number,
                    version,
                } => {
                    let x = FRAME_HANDSHAKE.to_be_bytes();
                    buffer[0] = x[0];
                    buffer[1..8].copy_from_slice(&magic_number);
                    buffer[8..12].copy_from_slice(&version[0].to_le_bytes());
                    buffer[12..16].copy_from_slice(&version[1].to_le_bytes());
                    buffer[16..20].copy_from_slice(&version[2].to_le_bytes());
                    20
                },
                Frame::Init { pid, secret } => {
                    buffer[0] = FRAME_INIT.to_be_bytes()[0];
                    buffer[1..17].copy_from_slice(&pid.to_le_bytes());
                    buffer[17..33].copy_from_slice(&secret.to_le_bytes());
                    33
                },
                Frame::Shutdown => {
                    buffer[0] = FRAME_SHUTDOWN.to_be_bytes()[0];
                    1
                },
                Frame::OpenStream {
                    sid,
                    prio,
                    promises,
                } => {
                    buffer[0] = FRAME_OPEN_STREAM.to_be_bytes()[0];
                    buffer[1..9].copy_from_slice(&sid.to_le_bytes());
                    buffer[9] = prio.to_le_bytes()[0];
                    buffer[10] = promises.to_le_bytes()[0];
                    11
                },
                Frame::CloseStream { sid } => {
                    buffer[0] = FRAME_CLOSE_STREAM.to_be_bytes()[0];
                    buffer[1..9].copy_from_slice(&sid.to_le_bytes());
                    9
                },
                Frame::DataHeader { mid, sid, length } => {
                    buffer[0] = FRAME_DATA_HEADER.to_be_bytes()[0];
                    buffer[1..9].copy_from_slice(&mid.to_le_bytes());
                    buffer[9..17].copy_from_slice(&sid.to_le_bytes());
                    buffer[17..25].copy_from_slice(&length.to_le_bytes());
                    25
                },
                Frame::Data { mid, start, data } => {
                    buffer[0] = FRAME_DATA.to_be_bytes()[0];
                    buffer[1..9].copy_from_slice(&mid.to_le_bytes());
                    buffer[9..17].copy_from_slice(&start.to_le_bytes());
                    buffer[17..19].copy_from_slice(&(data.len() as u16).to_le_bytes());
                    buffer[19..(data.len() + 19)].clone_from_slice(&data[..]);
                    throughput_cache.inc_by(data.len() as i64);
                    19 + data.len()
                },
                Frame::Raw(data) => {
                    buffer[0] = FRAME_RAW.to_be_bytes()[0];
                    buffer[1..3].copy_from_slice(&(data.len() as u16).to_le_bytes());
                    buffer[3..(data.len() + 3)].clone_from_slice(&data[..]);
                    3 + data.len()
                },
            };
            let mut start = 0;
            while start < len {
                trace!(?start, ?len, "splitting up udp frame in multiple packages");
                match self
                    .socket
                    .send_to(&buffer[start..len], self.remote_addr)
                    .await
                {
                    Ok(n) => {
                        start += n;
                        if n != len {
                            error!(
                                "THIS DOESNT WORK, as RECEIVER CURRENLTY ONLY HANDLES 1 FRAME per \
                                 UDP message. splitting up will fail!"
                            );
                        }
                    },
                    Err(e) => error!(?e, "need to handle that error!"),
                }
            }
        }
        trace!("shutting down udp write()");
    }
}
