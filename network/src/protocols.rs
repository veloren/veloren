#[cfg(feature = "metrics")]
use crate::metrics::{CidFrameCache, NetworkMetrics};
use crate::{
    participant::C2pFrame,
    types::{Cid, Frame},
};
use async_std::{
    io::prelude::*,
    net::{TcpStream, UdpSocket},
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
    #[cfg(feature = "metrics")]
    metrics: Arc<NetworkMetrics>,
}

#[derive(Debug)]
pub(crate) struct UdpProtocol {
    socket: Arc<UdpSocket>,
    remote_addr: SocketAddr,
    #[cfg(feature = "metrics")]
    metrics: Arc<NetworkMetrics>,
    data_in: Mutex<mpsc::UnboundedReceiver<Vec<u8>>>,
}

//TODO: PERFORMACE: Use BufWriter and BufReader from std::io!
impl TcpProtocol {
    pub(crate) fn new(
        stream: TcpStream,
        #[cfg(feature = "metrics")] metrics: Arc<NetworkMetrics>,
    ) -> Self {
        Self {
            stream,
            #[cfg(feature = "metrics")]
            metrics,
        }
    }

    async fn read_frame<R: ReadExt + std::marker::Unpin>(
        r: &mut R,
        mut end_receiver: &mut Fuse<oneshot::Receiver<()>>,
    ) -> Result<Frame, Option<std::io::Error>> {
        let handle = |read_result| match read_result {
            Ok(_) => Ok(()),
            Err(e) => Err(Some(e)),
        };

        let mut frame_no = [0u8; 1];
        match select! {
            r = r.read_exact(&mut frame_no).fuse() => Some(r),
            _ = end_receiver => None,
        } {
            Some(read_result) => handle(read_result)?,
            None => {
                trace!("shutdown requested");
                return Err(None);
            },
        };

        match frame_no[0] {
            FRAME_HANDSHAKE => {
                let mut bytes = [0u8; 19];
                handle(r.read_exact(&mut bytes).await)?;
                Ok(Frame::gen_handshake(bytes))
            },
            FRAME_INIT => {
                let mut bytes = [0u8; 32];
                handle(r.read_exact(&mut bytes).await)?;
                Ok(Frame::gen_init(bytes))
            },
            FRAME_SHUTDOWN => Ok(Frame::Shutdown),
            FRAME_OPEN_STREAM => {
                let mut bytes = [0u8; 10];
                handle(r.read_exact(&mut bytes).await)?;
                Ok(Frame::gen_open_stream(bytes))
            },
            FRAME_CLOSE_STREAM => {
                let mut bytes = [0u8; 8];
                handle(r.read_exact(&mut bytes).await)?;
                Ok(Frame::gen_close_stream(bytes))
            },
            FRAME_DATA_HEADER => {
                let mut bytes = [0u8; 24];
                handle(r.read_exact(&mut bytes).await)?;
                Ok(Frame::gen_data_header(bytes))
            },
            FRAME_DATA => {
                let mut bytes = [0u8; 18];
                handle(r.read_exact(&mut bytes).await)?;
                let (mid, start, length) = Frame::gen_data(bytes);
                let mut data = vec![0; length as usize];
                handle(r.read_exact(&mut data).await)?;
                Ok(Frame::Data { mid, start, data })
            },
            FRAME_RAW => {
                let mut bytes = [0u8; 2];
                handle(r.read_exact(&mut bytes).await)?;
                let length = Frame::gen_raw(bytes);
                let mut data = vec![0; length as usize];
                handle(r.read_exact(&mut data).await)?;
                Ok(Frame::Raw(data))
            },
            other => {
                // report a RAW frame, but cannot rely on the next 2 bytes to be a size.
                // guessing 32 bytes, which might help to sort down issues
                let mut data = vec![0; 32];
                //keep the first byte!
                match r.read(&mut data[1..]).await {
                    Ok(n) => {
                        data.truncate(n + 1);
                        Ok(())
                    },
                    Err(e) => Err(Some(e)),
                }?;
                data[0] = other;
                warn!(?data, "got a unexpected RAW msg");
                Ok(Frame::Raw(data))
            },
        }
    }

    pub async fn read_from_wire(
        &self,
        cid: Cid,
        w2c_cid_frame_s: &mut mpsc::UnboundedSender<C2pFrame>,
        end_r: oneshot::Receiver<()>,
    ) {
        trace!("Starting up tcp read()");
        #[cfg(feature = "metrics")]
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_in_total.clone(), cid);
        #[cfg(feature = "metrics")]
        let throughput_cache = self
            .metrics
            .wire_in_throughput
            .with_label_values(&[&cid.to_string()]);
        let mut stream = self.stream.clone();
        let mut end_r = end_r.fuse();

        loop {
            match Self::read_frame(&mut stream, &mut end_r).await {
                Ok(frame) => {
                    #[cfg(feature = "metrics")]
                    {
                        metrics_cache.with_label_values(&frame).inc();
                        if let Frame::Data {
                            mid: _,
                            start: _,
                            ref data,
                        } = frame
                        {
                            throughput_cache.inc_by(data.len() as i64);
                        }
                    }
                    w2c_cid_frame_s
                        .send((cid, Ok(frame)))
                        .await
                        .expect("Channel or Participant seems no longer to exist");
                },
                Err(e_option) => {
                    if let Some(e) = e_option {
                        info!(?e, "Closing tcp protocol due to read error");
                        //w2c_cid_frame_s is shared, dropping it wouldn't notify the receiver as
                        // every channel is holding a sender! thats why Ne
                        // need a explicit STOP here
                        w2c_cid_frame_s
                            .send((cid, Err(())))
                            .await
                            .expect("Channel or Participant seems no longer to exist");
                    }
                    //None is clean shutdown
                    break;
                },
            }
        }
        trace!("Shutting down tcp read()");
    }

    pub async fn write_frame<W: WriteExt + std::marker::Unpin>(
        w: &mut W,
        frame: Frame,
    ) -> Result<(), std::io::Error> {
        match frame {
            Frame::Handshake {
                magic_number,
                version,
            } => {
                w.write_all(&FRAME_HANDSHAKE.to_be_bytes()).await?;
                w.write_all(&magic_number).await?;
                w.write_all(&version[0].to_le_bytes()).await?;
                w.write_all(&version[1].to_le_bytes()).await?;
                w.write_all(&version[2].to_le_bytes()).await?;
            },
            Frame::Init { pid, secret } => {
                w.write_all(&FRAME_INIT.to_be_bytes()).await?;
                w.write_all(&pid.to_le_bytes()).await?;
                w.write_all(&secret.to_le_bytes()).await?;
            },
            Frame::Shutdown => {
                w.write_all(&FRAME_SHUTDOWN.to_be_bytes()).await?;
            },
            Frame::OpenStream {
                sid,
                prio,
                promises,
            } => {
                w.write_all(&FRAME_OPEN_STREAM.to_be_bytes()).await?;
                w.write_all(&sid.to_le_bytes()).await?;
                w.write_all(&prio.to_le_bytes()).await?;
                w.write_all(&promises.to_le_bytes()).await?;
            },
            Frame::CloseStream { sid } => {
                w.write_all(&FRAME_CLOSE_STREAM.to_be_bytes()).await?;
                w.write_all(&sid.to_le_bytes()).await?;
            },
            Frame::DataHeader { mid, sid, length } => {
                w.write_all(&FRAME_DATA_HEADER.to_be_bytes()).await?;
                w.write_all(&mid.to_le_bytes()).await?;
                w.write_all(&sid.to_le_bytes()).await?;
                w.write_all(&length.to_le_bytes()).await?;
            },
            Frame::Data { mid, start, data } => {
                w.write_all(&FRAME_DATA.to_be_bytes()).await?;
                w.write_all(&mid.to_le_bytes()).await?;
                w.write_all(&start.to_le_bytes()).await?;
                w.write_all(&(data.len() as u16).to_le_bytes()).await?;
                w.write_all(&data).await?;
            },
            Frame::Raw(data) => {
                w.write_all(&FRAME_RAW.to_be_bytes()).await?;
                w.write_all(&(data.len() as u16).to_le_bytes()).await?;
                w.write_all(&data).await?;
            },
        };
        Ok(())
    }

    pub async fn write_to_wire(&self, cid: Cid, mut c2w_frame_r: mpsc::UnboundedReceiver<Frame>) {
        trace!("Starting up tcp write()");
        let mut stream = self.stream.clone();
        #[cfg(feature = "metrics")]
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_out_total.clone(), cid);
        #[cfg(feature = "metrics")]
        let throughput_cache = self
            .metrics
            .wire_out_throughput
            .with_label_values(&[&cid.to_string()]);
        #[cfg(not(feature = "metrics"))]
        let _cid = cid;

        while let Some(frame) = c2w_frame_r.next().await {
            #[cfg(feature = "metrics")]
            {
                metrics_cache.with_label_values(&frame).inc();
                if let Frame::Data {
                    mid: _,
                    start: _,
                    ref data,
                } = frame
                {
                    throughput_cache.inc_by(data.len() as i64);
                }
            }
            if let Err(e) = Self::write_frame(&mut stream, frame).await {
                info!(
                    ?e,
                    "Got an error writing to tcp, going to close this channel"
                );
                c2w_frame_r.close();
                break;
            };
        }
        trace!("shutting down tcp write()");
    }
}

impl UdpProtocol {
    pub(crate) fn new(
        socket: Arc<UdpSocket>,
        remote_addr: SocketAddr,
        #[cfg(feature = "metrics")] metrics: Arc<NetworkMetrics>,
        data_in: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Self {
        Self {
            socket,
            remote_addr,
            #[cfg(feature = "metrics")]
            metrics,
            data_in: Mutex::new(data_in),
        }
    }

    pub async fn read_from_wire(
        &self,
        cid: Cid,
        w2c_cid_frame_s: &mut mpsc::UnboundedSender<C2pFrame>,
        end_r: oneshot::Receiver<()>,
    ) {
        trace!("Starting up udp read()");
        #[cfg(feature = "metrics")]
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_in_total.clone(), cid);
        #[cfg(feature = "metrics")]
        let throughput_cache = self
            .metrics
            .wire_in_throughput
            .with_label_values(&[&cid.to_string()]);
        let mut data_in = self.data_in.lock().await;
        let mut end_r = end_r.fuse();
        while let Some(bytes) = select! {
            r = data_in.next().fuse() => match r {
                Some(r) => Some(r),
                None => {
                    info!("Udp read ended");
                    w2c_cid_frame_s.send((cid, Err(()))).await.expect("Channel or Participant seems no longer to exist");
                    None
                }
            },
            _ = end_r => None,
        } {
            trace!("Got raw UDP message with len: {}", bytes.len());
            let frame_no = bytes[0];
            let frame = match frame_no {
                FRAME_HANDSHAKE => {
                    Frame::gen_handshake(*<&[u8; 19]>::try_from(&bytes[1..20]).unwrap())
                },
                FRAME_INIT => Frame::gen_init(*<&[u8; 32]>::try_from(&bytes[1..33]).unwrap()),
                FRAME_SHUTDOWN => Frame::Shutdown,
                FRAME_OPEN_STREAM => {
                    Frame::gen_open_stream(*<&[u8; 10]>::try_from(&bytes[1..11]).unwrap())
                },
                FRAME_CLOSE_STREAM => {
                    Frame::gen_close_stream(*<&[u8; 8]>::try_from(&bytes[1..9]).unwrap())
                },
                FRAME_DATA_HEADER => {
                    Frame::gen_data_header(*<&[u8; 24]>::try_from(&bytes[1..25]).unwrap())
                },
                FRAME_DATA => {
                    let (mid, start, length) =
                        Frame::gen_data(*<&[u8; 18]>::try_from(&bytes[1..19]).unwrap());
                    let mut data = vec![0; length as usize];
                    #[cfg(feature = "metrics")]
                    throughput_cache.inc_by(length as i64);
                    data.copy_from_slice(&bytes[19..]);
                    Frame::Data { mid, start, data }
                },
                FRAME_RAW => {
                    let length = Frame::gen_raw(*<&[u8; 2]>::try_from(&bytes[1..3]).unwrap());
                    let mut data = vec![0; length as usize];
                    data.copy_from_slice(&bytes[3..]);
                    Frame::Raw(data)
                },
                _ => Frame::Raw(bytes),
            };
            #[cfg(feature = "metrics")]
            metrics_cache.with_label_values(&frame).inc();
            w2c_cid_frame_s.send((cid, Ok(frame))).await.unwrap();
        }
        trace!("Shutting down udp read()");
    }

    pub async fn write_to_wire(&self, cid: Cid, mut c2w_frame_r: mpsc::UnboundedReceiver<Frame>) {
        trace!("Starting up udp write()");
        let mut buffer = [0u8; 2000];
        #[cfg(feature = "metrics")]
        let mut metrics_cache = CidFrameCache::new(self.metrics.frames_wire_out_total.clone(), cid);
        #[cfg(feature = "metrics")]
        let throughput_cache = self
            .metrics
            .wire_out_throughput
            .with_label_values(&[&cid.to_string()]);
        #[cfg(not(feature = "metrics"))]
        let _cid = cid;
        while let Some(frame) = c2w_frame_r.next().await {
            #[cfg(feature = "metrics")]
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
                    #[cfg(feature = "metrics")]
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
                                "THIS DOESN'T WORK, as RECEIVER CURRENTLY ONLY HANDLES 1 FRAME \
                                 per UDP message. splitting up will fail!"
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
    use crate::{metrics::NetworkMetrics, types::Pid};
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
            let (mut w2c_cid_frame_s, mut w2c_cid_frame_r) = mpsc::unbounded::<C2pFrame>();
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
            if let Ok(Frame::Handshake {
                magic_number,
                version,
            }) = frame
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
            let (mut w2c_cid_frame_s, mut w2c_cid_frame_r) = mpsc::unbounded::<C2pFrame>();
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
            if let Ok(Frame::Raw(data)) = frame {
                assert_eq!(&data.as_slice(), b"x4hrtzsektfhxugzdtz5r78gzrtzfhxf");
            } else {
                panic!("wrong frame type");
            }
            read_stop_sender.send(()).unwrap();
            t.join().unwrap();
        });
    }
}
