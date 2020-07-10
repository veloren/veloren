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
    future::{Fuse, FutureExt},
    lock::Mutex,
    select,
    sink::SinkExt,
    stream::StreamExt,
};
use std::{convert::TryFrom, net::SocketAddr, sync::Arc};
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
    async fn read_or_close(
        cid: Cid,
        mut stream: &TcpStream,
        mut bytes: &mut [u8],
        w2c_cid_frame_s: &mut mpsc::UnboundedSender<(Cid, Frame)>,
        mut end_receiver: &mut Fuse<oneshot::Receiver<()>>,
    ) -> bool {
        match select! {
            r = stream.read_exact(&mut bytes).fuse() => Some(r),
            _ = end_receiver => None,
        } {
            Some(Ok(_)) => false,
            Some(Err(e)) => {
                debug!(
                    ?cid,
                    ?e,
                    "Closing tcp protocol due to read error, sending close frame to gracefully \
                     shutdown"
                );
                w2c_cid_frame_s
                    .send((cid, Frame::Shutdown))
                    .await
                    .expect("Channel or Participant seems no longer to exist to be Shutdown");
                true
            },
            None => {
                trace!(?cid, "shutdown requested");
                true
            },
        }
    }

    pub async fn read_from_wire(
        &self,
        cid: Cid,
        w2c_cid_frame_s: &mut mpsc::UnboundedSender<(Cid, Frame)>,
        end_r: oneshot::Receiver<()>,
    ) {
        trace!("Starting up tcp read()");
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_in_total.clone(), cid);
        let throughput_cache = self
            .metrics
            .wire_in_throughput
            .with_label_values(&[&cid.to_string()]);
        let stream = self.stream.clone();
        let mut end_r = end_r.fuse();

        macro_rules! read_or_close {
            ($x:expr) => {
                if TcpProtocol::read_or_close(cid, &stream, $x, w2c_cid_frame_s, &mut end_r).await {
                    info!("Tcp stream closed, shutting down read");
                    break;
                }
            };
        }

        loop {
            let frame_no = {
                let mut bytes = [0u8; 1];
                read_or_close!(&mut bytes);
                bytes[0]
            };
            let frame = match frame_no {
                FRAME_HANDSHAKE => {
                    let mut bytes = [0u8; 19];
                    read_or_close!(&mut bytes);
                    let magic_number = *<&[u8; 7]>::try_from(&bytes[0..7]).unwrap();
                    Frame::Handshake {
                        magic_number,
                        version: [
                            u32::from_le_bytes(*<&[u8; 4]>::try_from(&bytes[7..11]).unwrap()),
                            u32::from_le_bytes(*<&[u8; 4]>::try_from(&bytes[11..15]).unwrap()),
                            u32::from_le_bytes(*<&[u8; 4]>::try_from(&bytes[15..19]).unwrap()),
                        ],
                    }
                },
                FRAME_INIT => {
                    let mut bytes = [0u8; 16];
                    read_or_close!(&mut bytes);
                    let pid = Pid::from_le_bytes(bytes);
                    read_or_close!(&mut bytes);
                    let secret = u128::from_le_bytes(bytes);
                    Frame::Init { pid, secret }
                },
                FRAME_SHUTDOWN => Frame::Shutdown,
                FRAME_OPEN_STREAM => {
                    let mut bytes = [0u8; 10];
                    read_or_close!(&mut bytes);
                    let sid = Sid::from_le_bytes(*<&[u8; 8]>::try_from(&bytes[0..8]).unwrap());
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
                    read_or_close!(&mut bytes);
                    let sid = Sid::from_le_bytes(*<&[u8; 8]>::try_from(&bytes[0..8]).unwrap());
                    Frame::CloseStream { sid }
                },
                FRAME_DATA_HEADER => {
                    let mut bytes = [0u8; 24];
                    read_or_close!(&mut bytes);
                    let mid = Mid::from_le_bytes(*<&[u8; 8]>::try_from(&bytes[0..8]).unwrap());
                    let sid = Sid::from_le_bytes(*<&[u8; 8]>::try_from(&bytes[8..16]).unwrap());
                    let length = u64::from_le_bytes(*<&[u8; 8]>::try_from(&bytes[16..24]).unwrap());
                    Frame::DataHeader { mid, sid, length }
                },
                FRAME_DATA => {
                    let mut bytes = [0u8; 18];
                    read_or_close!(&mut bytes);
                    let mid = Mid::from_le_bytes(*<&[u8; 8]>::try_from(&bytes[0..8]).unwrap());
                    let start = u64::from_le_bytes(*<&[u8; 8]>::try_from(&bytes[8..16]).unwrap());
                    let length = u16::from_le_bytes(*<&[u8; 2]>::try_from(&bytes[16..18]).unwrap());
                    let mut data = vec![0; length as usize];
                    throughput_cache.inc_by(length as i64);
                    read_or_close!(&mut data);
                    Frame::Data { mid, start, data }
                },
                FRAME_RAW => {
                    let mut bytes = [0u8; 2];
                    read_or_close!(&mut bytes);
                    let length = u16::from_le_bytes([bytes[0], bytes[1]]);
                    let mut data = vec![0; length as usize];
                    read_or_close!(&mut data);
                    Frame::Raw(data)
                },
                other => {
                    // report a RAW frame, but cannot rely on the next 2 bytes to be a size.
                    // guessing 32 bytes, which might help to sort down issues
                    let mut data = vec![0; 32];
                    //keep the first byte!
                    read_or_close!(&mut data[1..]);
                    data[0] = other;
                    Frame::Raw(data)
                },
            };
            metrics_cache.with_label_values(&frame).inc();
            w2c_cid_frame_s
                .send((cid, frame))
                .await
                .expect("Channel or Participant seems no longer to exist");
        }
        trace!("Shutting down tcp read()");
    }

    /// read_except and if it fails, close the protocol
    async fn write_or_close(
        stream: &mut TcpStream,
        bytes: &[u8],
        c2w_frame_r: &mut mpsc::UnboundedReceiver<Frame>,
    ) -> bool {
        match stream.write_all(&bytes).await {
            Err(e) => {
                debug!(
                    ?e,
                    "Got an error writing to tcp, going to close this channel"
                );
                c2w_frame_r.close();
                true
            },
            _ => false,
        }
    }

    pub async fn write_to_wire(&self, cid: Cid, mut c2w_frame_r: mpsc::UnboundedReceiver<Frame>) {
        trace!("Starting up tcp write()");
        let mut stream = self.stream.clone();
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_out_total.clone(), cid);
        let throughput_cache = self
            .metrics
            .wire_out_throughput
            .with_label_values(&[&cid.to_string()]);

        macro_rules! write_or_close {
            ($x:expr) => {
                if TcpProtocol::write_or_close(&mut stream, $x, &mut c2w_frame_r).await {
                    info!("Tcp stream closed, shutting down write");
                    break;
                }
            };
        }

        while let Some(frame) = c2w_frame_r.next().await {
            metrics_cache.with_label_values(&frame).inc();
            match frame {
                Frame::Handshake {
                    magic_number,
                    version,
                } => {
                    write_or_close!(&FRAME_HANDSHAKE.to_be_bytes());
                    write_or_close!(&magic_number);
                    write_or_close!(&version[0].to_le_bytes());
                    write_or_close!(&version[1].to_le_bytes());
                    write_or_close!(&version[2].to_le_bytes());
                },
                Frame::Init { pid, secret } => {
                    write_or_close!(&FRAME_INIT.to_be_bytes());
                    write_or_close!(&pid.to_le_bytes());
                    write_or_close!(&secret.to_le_bytes());
                },
                Frame::Shutdown => {
                    write_or_close!(&FRAME_SHUTDOWN.to_be_bytes());
                },
                Frame::OpenStream {
                    sid,
                    prio,
                    promises,
                } => {
                    write_or_close!(&FRAME_OPEN_STREAM.to_be_bytes());
                    write_or_close!(&sid.to_le_bytes());
                    write_or_close!(&prio.to_le_bytes());
                    write_or_close!(&promises.to_le_bytes());
                },
                Frame::CloseStream { sid } => {
                    write_or_close!(&FRAME_CLOSE_STREAM.to_be_bytes());
                    write_or_close!(&sid.to_le_bytes());
                },
                Frame::DataHeader { mid, sid, length } => {
                    write_or_close!(&FRAME_DATA_HEADER.to_be_bytes());
                    write_or_close!(&mid.to_le_bytes());
                    write_or_close!(&sid.to_le_bytes());
                    write_or_close!(&length.to_le_bytes());
                },
                Frame::Data { mid, start, data } => {
                    throughput_cache.inc_by(data.len() as i64);
                    write_or_close!(&FRAME_DATA.to_be_bytes());
                    write_or_close!(&mid.to_le_bytes());
                    write_or_close!(&start.to_le_bytes());
                    write_or_close!(&(data.len() as u16).to_le_bytes());
                    write_or_close!(&data);
                },
                Frame::Raw(data) => {
                    write_or_close!(&FRAME_RAW.to_be_bytes());
                    write_or_close!(&(data.len() as u16).to_le_bytes());
                    write_or_close!(&data);
                },
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
        end_r: oneshot::Receiver<()>,
    ) {
        trace!("Starting up udp read()");
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_in_total.clone(), cid);
        let throughput_cache = self
            .metrics
            .wire_in_throughput
            .with_label_values(&[&cid.to_string()]);
        let mut data_in = self.data_in.lock().await;
        let mut end_r = end_r.fuse();
        while let Some(bytes) = select! {
            r = data_in.next().fuse() => r,
            _ = end_r => None,
        } {
            trace!("Got raw UDP message with len: {}", bytes.len());
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
        trace!("Shutting down udp read()");
    }

    pub async fn write_to_wire(&self, cid: Cid, mut c2w_frame_r: mpsc::UnboundedReceiver<Frame>) {
        trace!("Starting up udp write()");
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
                trace!(?start, ?len, "Splitting up udp frame in multiple packages");
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
                    Err(e) => error!(?e, "Need to handle that error!"),
                }
            }
        }
        trace!("Shutting down udp write()");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        metrics::NetworkMetrics,
        types::{Cid, Pid},
    };
    use async_std::net;
    use futures::{executor::block_on, stream::StreamExt};
    use std::sync::Arc;

    #[test]
    fn tcp_read_handshake() {
        let pid = Pid::new();
        let cid = 80085;
        let metrics = Arc::new(NetworkMetrics::new(&pid).unwrap());
        let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 50500);
        block_on(async {
            let server = net::TcpListener::bind(addr).await.unwrap();
            let mut client = net::TcpStream::connect(addr).await.unwrap();

            let s_stream = server.incoming().next().await.unwrap().unwrap();
            let prot = TcpProtocol::new(s_stream, metrics);

            //Send Handshake
            client.write_all(&[FRAME_HANDSHAKE]).await.unwrap();
            client.write_all(b"HELLOWO").await.unwrap();
            client.write_all(&1337u32.to_le_bytes()).await.unwrap();
            client.write_all(&0u32.to_le_bytes()).await.unwrap();
            client.write_all(&42u32.to_le_bytes()).await.unwrap();
            client.flush();

            //handle data
            let (mut w2c_cid_frame_s, mut w2c_cid_frame_r) = mpsc::unbounded::<(Cid, Frame)>();
            let (read_stop_sender, read_stop_receiver) = oneshot::channel();
            let cid2 = cid;
            let t = std::thread::spawn(move || {
                block_on(async {
                    prot.read_from_wire(cid2, &mut w2c_cid_frame_s, read_stop_receiver)
                        .await;
                })
            });
            // Assert than we get some value back! Its a Handshake!
            //async_std::task::sleep(std::time::Duration::from_millis(1000));
            let (cid_r, frame) = w2c_cid_frame_r.next().await.unwrap();
            assert_eq!(cid, cid_r);
            if let Frame::Handshake {
                magic_number,
                version,
            } = frame
            {
                assert_eq!(&magic_number, b"HELLOWO");
                assert_eq!(version, [1337, 0, 42]);
            } else {
                panic!("wrong handshake");
            }
            read_stop_sender.send(()).unwrap();
            t.join().unwrap();
        });
    }

    #[test]
    fn tcp_read_garbage() {
        let pid = Pid::new();
        let cid = 80085;
        let metrics = Arc::new(NetworkMetrics::new(&pid).unwrap());
        let addr = std::net::SocketAddrV4::new(std::net::Ipv4Addr::new(127, 0, 0, 1), 50501);
        block_on(async {
            let server = net::TcpListener::bind(addr).await.unwrap();
            let mut client = net::TcpStream::connect(addr).await.unwrap();

            let s_stream = server.incoming().next().await.unwrap().unwrap();
            let prot = TcpProtocol::new(s_stream, metrics);

            //Send Handshake
            client
                .write_all("x4hrtzsektfhxugzdtz5r78gzrtzfhxfdthfthuzhfzzufasgasdfg".as_bytes())
                .await
                .unwrap();
            client.flush();

            //handle data
            let (mut w2c_cid_frame_s, mut w2c_cid_frame_r) = mpsc::unbounded::<(Cid, Frame)>();
            let (read_stop_sender, read_stop_receiver) = oneshot::channel();
            let cid2 = cid;
            let t = std::thread::spawn(move || {
                block_on(async {
                    prot.read_from_wire(cid2, &mut w2c_cid_frame_s, read_stop_receiver)
                        .await;
                })
            });
            // Assert than we get some value back! Its a Raw!
            let (cid_r, frame) = w2c_cid_frame_r.next().await.unwrap();
            assert_eq!(cid, cid_r);
            if let Frame::Raw(data) = frame {
                assert_eq!(&data.as_slice(), b"x4hrtzsektfhxugzdtz5r78gzrtzfhxf");
            } else {
                panic!("wrong frame type");
            }
            read_stop_sender.send(()).unwrap();
            t.join().unwrap();
        });
    }
}
