use crate::types::Frame;
use async_std::{
    net::{TcpStream, UdpSocket},
    prelude::*,
    sync::RwLock,
};
use futures::{channel::mpsc, future::FutureExt, select, sink::SinkExt, stream::StreamExt};
use std::{net::SocketAddr, sync::Arc};
use tracing::*;

#[derive(Debug)]
pub(crate) enum Protocols {
    Tcp(TcpProtocol),
    Udp(UdpProtocol),
    //Mpsc(MpscChannel),
}

#[derive(Debug)]
pub(crate) struct TcpProtocol {
    stream: TcpStream,
}

#[derive(Debug)]
pub(crate) struct UdpProtocol {
    socket: Arc<UdpSocket>,
    remote_addr: SocketAddr,
    data_in: RwLock<mpsc::UnboundedReceiver<Vec<u8>>>,
}

impl TcpProtocol {
    pub(crate) fn new(stream: TcpStream) -> Self { Self { stream } }

    pub async fn read(&self, mut frame_handler: mpsc::UnboundedSender<Frame>) {
        let mut stream = self.stream.clone();
        let mut buffer = NetworkBuffer::new();
        loop {
            match stream.read(buffer.get_write_slice(2048)).await {
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
                                trace!(
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

    //dezerialize here as this is executed in a seperate thread PER channel.
    // Limites Throughput per single Receiver but stays in same thread (maybe as its
    // in a threadpool) for TCP, UDP and MPSC
    pub async fn write(
        &self,
        mut internal_frame_receiver: mpsc::UnboundedReceiver<Frame>,
        mut external_frame_receiver: mpsc::UnboundedReceiver<Frame>,
    ) {
        let mut stream = self.stream.clone();
        while let Some(frame) = select! {
            next = internal_frame_receiver.next().fuse() => next,
            next = external_frame_receiver.next().fuse() => next,
        } {
            let data = bincode::serialize(&frame).unwrap();
            let len = data.len();
            trace!(?len, "going to send frame via Tcp");
            stream.write_all(data.as_slice()).await.unwrap();
        }
    }
}

impl UdpProtocol {
    pub(crate) fn new(
        socket: Arc<UdpSocket>,
        remote_addr: SocketAddr,
        data_in: mpsc::UnboundedReceiver<Vec<u8>>,
    ) -> Self {
        Self {
            socket,
            remote_addr,
            data_in: RwLock::new(data_in),
        }
    }

    pub async fn read(&self, mut frame_handler: mpsc::UnboundedSender<Frame>) {
        let mut data_in = self.data_in.write().await;
        let mut buffer = NetworkBuffer::new();
        while let Some(data) = data_in.next().await {
            let n = data.len();
            let slice = &mut buffer.get_write_slice(n)[0..n]; //get_write_slice can return  more then n!
            slice.clone_from_slice(data.as_slice());
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
                            "message cant be parsed, probably because we need to wait for more \
                             data"
                        );
                        break;
                    },
                }
            }
            buffer.actually_read(read_ok);
        }
    }

    pub async fn write(
        &self,
        mut internal_frame_receiver: mpsc::UnboundedReceiver<Frame>,
        mut external_frame_receiver: mpsc::UnboundedReceiver<Frame>,
    ) {
        let mut buffer = NetworkBuffer::new();
        while let Some(frame) = select! {
            next = internal_frame_receiver.next().fuse() => next,
            next = external_frame_receiver.next().fuse() => next,
        } {
            let len = bincode::serialized_size(&frame).unwrap() as usize;
            match bincode::serialize_into(buffer.get_write_slice(len), &frame) {
                Ok(_) => buffer.actually_written(len),
                Err(e) => error!("Oh nooo {}", e),
            };
            trace!(?len, "going to send frame via Udp");
            let mut to_send = buffer.get_read_slice();
            while to_send.len() > 0 {
                match self.socket.send_to(to_send, self.remote_addr).await {
                    Ok(n) => buffer.actually_read(n),
                    Err(e) => error!(?e, "need to handle that error!"),
                }
                to_send = buffer.get_read_slice();
            }
        }
    }
}

// INTERNAL NetworkBuffer

struct NetworkBuffer {
    pub(crate) data: Vec<u8>,
    pub(crate) read_idx: usize,
    pub(crate) write_idx: usize,
}

/// NetworkBuffer to use for streamed access
/// valid data is between read_idx and write_idx!
/// everything before read_idx is already processed and no longer important
/// everything after write_idx is either 0 or random data buffered
impl NetworkBuffer {
    fn new() -> Self {
        NetworkBuffer {
            data: vec![0; 2048],
            read_idx: 0,
            write_idx: 0,
        }
    }

    fn get_write_slice(&mut self, min_size: usize) -> &mut [u8] {
        if self.data.len() < self.write_idx + min_size {
            trace!(
                ?self,
                ?min_size,
                "need to resize because buffer is to small"
            );
            self.data.resize(self.write_idx + min_size, 0);
        }
        &mut self.data[self.write_idx..]
    }

    fn actually_written(&mut self, cnt: usize) { self.write_idx += cnt; }

    fn get_read_slice(&self) -> &[u8] { &self.data[self.read_idx..self.write_idx] }

    fn actually_read(&mut self, cnt: usize) {
        self.read_idx += cnt;
        if self.read_idx == self.write_idx {
            if self.read_idx > 10485760 {
                trace!(?self, "buffer empty, resetting indices");
            }
            self.read_idx = 0;
            self.write_idx = 0;
        }
        if self.write_idx > 10485760 {
            if self.write_idx - self.read_idx < 65536 {
                debug!(
                    ?self,
                    "This buffer is filled over 10 MB, but the actual data diff is less then \
                     65kB, which is a sign of stressing this connection much as always new data \
                     comes in - nevertheless, in order to handle this we will remove some data \
                     now so that this buffer doesn't grow endlessly"
                );
                let mut i2 = 0;
                for i in self.read_idx..self.write_idx {
                    self.data[i2] = self.data[i];
                    i2 += 1;
                }
                self.read_idx = 0;
                self.write_idx = i2;
            }
            if self.data.len() > 67108864 {
                warn!(
                    ?self,
                    "over 64Mbyte used, something seems fishy, len: {}",
                    self.data.len()
                );
            }
        }
    }
}

impl std::fmt::Debug for NetworkBuffer {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NetworkBuffer(len: {}, read: {}, write: {})",
            self.data.len(),
            self.read_idx,
            self.write_idx
        )
    }
}
