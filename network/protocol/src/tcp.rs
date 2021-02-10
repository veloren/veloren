use crate::{
    event::ProtocolEvent,
    frame::{Frame, InitFrame},
    handshake::{ReliableDrain, ReliableSink},
    io::{UnreliableDrain, UnreliableSink},
    metrics::{ProtocolMetricCache, RemoveReason},
    prio::PrioManager,
    types::Bandwidth,
    ProtocolError, RecvProtocol, SendProtocol,
};
use async_trait::async_trait;
use bytes::BytesMut;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::info;
#[cfg(feature = "trace_pedantic")]
use tracing::trace;

#[derive(Debug)]
pub struct TcpSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    buffer: BytesMut,
    store: PrioManager,
    closing_streams: Vec<Sid>,
    notify_closing_streams: Vec<Sid>,
    pending_shutdown: bool,
    drain: D,
    last: Instant,
    metrics: ProtocolMetricCache,
}

#[derive(Debug)]
pub struct TcpRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    buffer: BytesMut,
    incoming: HashMap<Mid, IncomingMsg>,
    sink: S,
    metrics: ProtocolMetricCache,
}

impl<D> TcpSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    pub fn new(drain: D, metrics: ProtocolMetricCache) -> Self {
        Self {
            buffer: BytesMut::new(),
            store: PrioManager::new(metrics.clone()),
            closing_streams: vec![],
            notify_closing_streams: vec![],
            pending_shutdown: false,
            drain,
            last: Instant::now(),
            metrics,
        }
    }
}

impl<S> TcpRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    pub fn new(sink: S, metrics: ProtocolMetricCache) -> Self {
        Self {
            buffer: BytesMut::new(),
            incoming: HashMap::new(),
            sink,
            metrics,
        }
    }
}

#[async_trait]
impl<D> SendProtocol for TcpSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    fn notify_from_recv(&mut self, event: ProtocolEvent) {
        match event {
            ProtocolEvent::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth,
            } => {
                self.store
                    .open_stream(sid, prio, promises, guaranteed_bandwidth);
            },
            ProtocolEvent::CloseStream { sid } => {
                if !self.store.try_close_stream(sid) {
                    #[cfg(feature = "trace_pedantic")]
                    trace!(?sid, "hold back notify close stream");
                    self.notify_closing_streams.push(sid);
                }
            },
            _ => {},
        }
    }

    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError> {
        #[cfg(feature = "trace_pedantic")]
        trace!(?event, "send");
        match event {
            ProtocolEvent::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth,
            } => {
                self.store
                    .open_stream(sid, prio, promises, guaranteed_bandwidth);
                event.to_frame().to_bytes(&mut self.buffer);
                self.drain.send(self.buffer.split()).await?;
            },
            ProtocolEvent::CloseStream { sid } => {
                if self.store.try_close_stream(sid) {
                    event.to_frame().to_bytes(&mut self.buffer);
                    self.drain.send(self.buffer.split()).await?;
                } else {
                    #[cfg(feature = "trace_pedantic")]
                    trace!(?sid, "hold back close stream");
                    self.closing_streams.push(sid);
                }
            },
            ProtocolEvent::Shutdown => {
                if self.store.is_empty() {
                    event.to_frame().to_bytes(&mut self.buffer);
                    self.drain.send(self.buffer.split()).await?;
                } else {
                    #[cfg(feature = "trace_pedantic")]
                    trace!("hold back shutdown");
                    self.pending_shutdown = true;
                }
            },
            ProtocolEvent::Message { buffer, mid, sid } => {
                self.metrics.smsg_ib(sid, buffer.data.len() as u64);
                self.store.add(buffer, mid, sid);
            },
        }
        Ok(())
    }

    async fn flush(&mut self, bandwidth: Bandwidth, dt: Duration) -> Result<(), ProtocolError> {
        let frames = self.store.grab(bandwidth, dt);
        for frame in frames {
            if let Frame::Data {
                mid: _,
                start: _,
                data,
            } = &frame
            {
                self.metrics.sdata_frames_b(data.len() as u64);
            }
            frame.to_bytes(&mut self.buffer);
            self.drain.send(self.buffer.split()).await?;
        }

        let mut finished_streams = vec![];
        for (i, &sid) in self.closing_streams.iter().enumerate() {
            if self.store.try_close_stream(sid) {
                #[cfg(feature = "trace_pedantic")]
                trace!(?sid, "close stream, as it's now empty");
                Frame::CloseStream { sid }.to_bytes(&mut self.buffer);
                self.drain.send(self.buffer.split()).await?;
                finished_streams.push(i);
            }
        }
        for i in finished_streams.iter().rev() {
            self.closing_streams.remove(*i);
        }

        let mut finished_streams = vec![];
        for (i, sid) in self.notify_closing_streams.iter().enumerate() {
            if self.store.try_close_stream(*sid) {
                #[cfg(feature = "trace_pedantic")]
                trace!(?sid, "close stream, as it's now empty");
                finished_streams.push(i);
            }
        }
        for i in finished_streams.iter().rev() {
            self.notify_closing_streams.remove(*i);
        }

        if self.pending_shutdown && self.store.is_empty() {
            #[cfg(feature = "trace_pedantic")]
            trace!("shutdown, as it's now empty");
            Frame::Shutdown {}.to_bytes(&mut self.buffer);
            self.drain.send(self.buffer.split()).await?;
            self.pending_shutdown = false;
        }
        Ok(())
    }
}

use crate::{
    message::MessageBuffer,
    types::{Mid, Sid},
};

#[derive(Debug)]
struct IncomingMsg {
    sid: Sid,
    length: u64,
    data: MessageBuffer,
}

#[async_trait]
impl<S> RecvProtocol for TcpRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError> {
        'outer: loop {
            while let Some(frame) = Frame::to_frame(&mut self.buffer) {
                #[cfg(feature = "trace_pedantic")]
                trace!(?frame, "recv");
                match frame {
                    Frame::Shutdown => break 'outer Ok(ProtocolEvent::Shutdown),
                    Frame::OpenStream {
                        sid,
                        prio,
                        promises,
                    } => {
                        break 'outer Ok(ProtocolEvent::OpenStream {
                            sid,
                            prio,
                            promises,
                            guaranteed_bandwidth: 1_000_000,
                        });
                    },
                    Frame::CloseStream { sid } => {
                        break 'outer Ok(ProtocolEvent::CloseStream { sid });
                    },
                    Frame::DataHeader { sid, mid, length } => {
                        let m = IncomingMsg {
                            sid,
                            length,
                            data: MessageBuffer { data: vec![] },
                        };
                        self.metrics.rmsg_ib(sid, length);
                        self.incoming.insert(mid, m);
                    },
                    Frame::Data {
                        mid,
                        start: _,
                        mut data,
                    } => {
                        self.metrics.rdata_frames_b(data.len() as u64);
                        let m = match self.incoming.get_mut(&mid) {
                            Some(m) => m,
                            None => {
                                info!(
                                    ?mid,
                                    "protocol violation by remote side: send Data before Header"
                                );
                                break 'outer Err(ProtocolError::Closed);
                            },
                        };
                        m.data.data.append(&mut data);
                        if m.data.data.len() == m.length as usize {
                            // finished, yay
                            drop(m);
                            let m = self.incoming.remove(&mid).unwrap();
                            self.metrics.rmsg_ob(
                                m.sid,
                                RemoveReason::Finished,
                                m.data.data.len() as u64,
                            );
                            break 'outer Ok(ProtocolEvent::Message {
                                sid: m.sid,
                                mid,
                                buffer: Arc::new(m.data),
                            });
                        }
                    },
                };
            }
            let chunk = self.sink.recv().await?;
            self.buffer.extend_from_slice(&chunk);
        }
    }
}

#[async_trait]
impl<D> ReliableDrain for TcpSendProtcol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    async fn send(&mut self, frame: InitFrame) -> Result<(), ProtocolError> {
        let mut buffer = BytesMut::with_capacity(500);
        frame.to_bytes(&mut buffer);
        self.drain.send(buffer).await
    }
}

#[async_trait]
impl<S> ReliableSink for TcpRecvProtcol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    async fn recv(&mut self) -> Result<InitFrame, ProtocolError> {
        while self.buffer.len() < 100 {
            let chunk = self.sink.recv().await?;
            self.buffer.extend_from_slice(&chunk);
            if let Some(frame) = InitFrame::to_frame(&mut self.buffer) {
                return Ok(frame);
            }
        }
        Err(ProtocolError::Closed)
    }
}

#[cfg(test)]
mod test_utils {
    //TCP protocol based on Channel
    use super::*;
    use crate::{
        io::*,
        metrics::{ProtocolMetricCache, ProtocolMetrics},
    };
    use async_channel::*;

    pub struct TcpDrain {
        pub sender: Sender<BytesMut>,
    }

    pub struct TcpSink {
        pub receiver: Receiver<BytesMut>,
    }

    /// emulate Tcp protocol on Channels
    pub fn tcp_bound(
        cap: usize,
        metrics: Option<ProtocolMetricCache>,
    ) -> [(TcpSendProtcol<TcpDrain>, TcpRecvProtcol<TcpSink>); 2] {
        let (s1, r1) = async_channel::bounded(cap);
        let (s2, r2) = async_channel::bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("tcp", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                TcpSendProtcol::new(TcpDrain { sender: s1 }, m.clone()),
                TcpRecvProtcol::new(TcpSink { receiver: r2 }, m.clone()),
            ),
            (
                TcpSendProtcol::new(TcpDrain { sender: s2 }, m.clone()),
                TcpRecvProtcol::new(TcpSink { receiver: r1 }, m.clone()),
            ),
        ]
    }

    #[async_trait]
    impl UnreliableDrain for TcpDrain {
        type DataFormat = BytesMut;

        async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
            self.sender
                .send(data)
                .await
                .map_err(|_| ProtocolError::Closed)
        }
    }

    #[async_trait]
    impl UnreliableSink for TcpSink {
        type DataFormat = BytesMut;

        async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
            self.receiver
                .recv()
                .await
                .map_err(|_| ProtocolError::Closed)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        metrics::{ProtocolMetricCache, ProtocolMetrics, RemoveReason},
        tcp::test_utils::*,
        types::{Pid, Promises, Sid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2},
        InitProtocol, MessageBuffer, ProtocolEvent, RecvProtocol, SendProtocol,
    };
    use bytes::BytesMut;
    use std::{sync::Arc, time::Duration};

    #[tokio::test]
    async fn handshake_all_good() {
        let [mut p1, mut p2] = tcp_bound(10, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move { p2.initialize(false, Pid::fake(3), 42).await });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Ok((Pid::fake(3), STREAM_ID_OFFSET1, 42)));
        assert_eq!(r2.unwrap(), Ok((Pid::fake(2), STREAM_ID_OFFSET2, 1337)));
    }

    #[tokio::test]
    async fn open_stream() {
        let [p1, p2] = tcp_bound(10, None);
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid: Sid::new(10),
            prio: 9u8,
            promises: Promises::ORDERED,
            guaranteed_bandwidth: 1_000_000,
        };
        s.send(event.clone()).await.unwrap();
        let e = r.recv().await.unwrap();
        assert_eq!(event, e);
    }

    #[tokio::test]
    async fn send_short_msg() {
        let [p1, p2] = tcp_bound(10, None);
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid: Sid::new(10),
            prio: 3u8,
            promises: Promises::ORDERED,
            guaranteed_bandwidth: 1_000_000,
        };
        s.send(event).await.unwrap();
        let _ = r.recv().await.unwrap();
        let event = ProtocolEvent::Message {
            sid: Sid::new(10),
            mid: 0,
            buffer: Arc::new(MessageBuffer {
                data: vec![188u8; 600],
            }),
        };
        s.send(event.clone()).await.unwrap();
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = r.recv().await.unwrap();
        assert_eq!(event, e);
        // 2nd short message
        let event = ProtocolEvent::Message {
            sid: Sid::new(10),
            mid: 1,
            buffer: Arc::new(MessageBuffer {
                data: vec![7u8; 30],
            }),
        };
        s.send(event.clone()).await.unwrap();
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = r.recv().await.unwrap();
        assert_eq!(event, e)
    }

    #[tokio::test]
    async fn send_long_msg() {
        let mut metrics =
            ProtocolMetricCache::new("long_tcp", Arc::new(ProtocolMetrics::new().unwrap()));
        let sid = Sid::new(1);
        let [p1, p2] = tcp_bound(10000, Some(metrics.clone()));
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED,
            guaranteed_bandwidth: 1_000_000,
        };
        s.send(event).await.unwrap();
        let _ = r.recv().await.unwrap();
        let event = ProtocolEvent::Message {
            sid,
            mid: 77,
            buffer: Arc::new(MessageBuffer {
                data: vec![99u8; 500_000],
            }),
        };
        s.send(event.clone()).await.unwrap();
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = r.recv().await.unwrap();
        assert_eq!(event, e);
        metrics.assert_msg(sid, 1, RemoveReason::Finished);
        metrics.assert_msg_bytes(sid, 500_000, RemoveReason::Finished);
        metrics.assert_data_frames(358);
        metrics.assert_data_frames_bytes(500_000);
    }

    #[tokio::test]
    async fn msg_finishes_after_close() {
        let sid = Sid::new(1);
        let [p1, p2] = tcp_bound(10000, None);
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED,
            guaranteed_bandwidth: 0,
        };
        s.send(event).await.unwrap();
        let _ = r.recv().await.unwrap();
        let event = ProtocolEvent::Message {
            sid,
            mid: 77,
            buffer: Arc::new(MessageBuffer {
                data: vec![99u8; 500_000],
            }),
        };
        s.send(event).await.unwrap();
        let event = ProtocolEvent::CloseStream { sid };
        s.send(event).await.unwrap();
        //send
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::Message { .. }));
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::CloseStream { .. }));
    }

    #[tokio::test]
    async fn msg_finishes_after_shutdown() {
        let sid = Sid::new(1);
        let [p1, p2] = tcp_bound(10000, None);
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED,
            guaranteed_bandwidth: 0,
        };
        s.send(event).await.unwrap();
        let _ = r.recv().await.unwrap();
        let event = ProtocolEvent::Message {
            sid,
            mid: 77,
            buffer: Arc::new(MessageBuffer {
                data: vec![99u8; 500_000],
            }),
        };
        s.send(event).await.unwrap();
        let event = ProtocolEvent::Shutdown {};
        s.send(event).await.unwrap();
        let event = ProtocolEvent::CloseStream { sid };
        s.send(event).await.unwrap();
        //send
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::Message { .. }));
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::CloseStream { .. }));
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::Shutdown { .. }));
    }

    #[tokio::test]
    async fn header_and_data_in_seperate_msg() {
        let sid = Sid::new(1);
        let (s, r) = async_channel::bounded(10);
        let m = ProtocolMetricCache::new("tcp", Arc::new(ProtocolMetrics::new().unwrap()));
        let mut r =
            super::TcpRecvProtcol::new(super::test_utils::TcpSink { receiver: r }, m.clone());

        const DATA1: &[u8; 69] =
            b"We need to make sure that its okay to send OPEN_STREAM and DATA_HEAD ";
        const DATA2: &[u8; 95] = b"in one chunk and (DATA and CLOSE_STREAM) in the second chunk. and then keep the connection open";
        let mut bytes = BytesMut::with_capacity(1500);
        use crate::frame::Frame;
        Frame::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED,
        }
        .to_bytes(&mut bytes);
        Frame::DataHeader {
            mid: 99,
            sid,
            length: (DATA1.len() + DATA2.len()) as u64,
        }
        .to_bytes(&mut bytes);
        s.send(bytes.split()).await.unwrap();

        Frame::Data {
            mid: 99,
            start: 0,
            data: DATA1.to_vec(),
        }
        .to_bytes(&mut bytes);
        Frame::Data {
            mid: 99,
            start: DATA1.len() as u64,
            data: DATA2.to_vec(),
        }
        .to_bytes(&mut bytes);
        Frame::CloseStream { sid }.to_bytes(&mut bytes);
        s.send(bytes.split()).await.unwrap();

        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::OpenStream { .. }));

        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::Message { .. }));

        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::CloseStream { .. }));
    }

    #[tokio::test]
    #[should_panic]
    async fn send_on_stream_from_remote_without_notify() {
        //remote opens stream
        //we send on it
        let [mut p1, mut p2] = tcp_bound(10, None);
        let event = ProtocolEvent::OpenStream {
            sid: Sid::new(10),
            prio: 3u8,
            promises: Promises::ORDERED,
            guaranteed_bandwidth: 1_000_000,
        };
        p1.0.send(event).await.unwrap();
        let _ = p2.1.recv().await.unwrap();
        let event = ProtocolEvent::Message {
            sid: Sid::new(10),
            mid: 0,
            buffer: Arc::new(MessageBuffer {
                data: vec![188u8; 600],
            }),
        };
        p2.0.send(event.clone()).await.unwrap();
        p2.0.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = p1.1.recv().await.unwrap();
        assert_eq!(event, e);
    }

    #[tokio::test]
    async fn send_on_stream_from_remote() {
        //remote opens stream
        //we send on it
        let [mut p1, mut p2] = tcp_bound(10, None);
        let event = ProtocolEvent::OpenStream {
            sid: Sid::new(10),
            prio: 3u8,
            promises: Promises::ORDERED,
            guaranteed_bandwidth: 1_000_000,
        };
        p1.0.send(event).await.unwrap();
        let e = p2.1.recv().await.unwrap();
        p2.0.notify_from_recv(e);
        let event = ProtocolEvent::Message {
            sid: Sid::new(10),
            mid: 0,
            buffer: Arc::new(MessageBuffer {
                data: vec![188u8; 600],
            }),
        };
        p2.0.send(event.clone()).await.unwrap();
        p2.0.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = p1.1.recv().await.unwrap();
        assert_eq!(event, e);
    }
}
