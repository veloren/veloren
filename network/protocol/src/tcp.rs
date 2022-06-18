use crate::{
    error::ProtocolError,
    event::ProtocolEvent,
    frame::{ITFrame, InitFrame, OTFrame},
    handshake::{ReliableDrain, ReliableSink},
    message::{ITMessage, ALLOC_BLOCK},
    metrics::{ProtocolMetricCache, RemoveReason},
    prio::PrioManager,
    types::{Bandwidth, Mid, Promises, Sid},
    RecvProtocol, SendProtocol, UnreliableDrain, UnreliableSink,
};
use async_trait::async_trait;
use bytes::BytesMut;
use hashbrown::HashMap;
use std::time::{Duration, Instant};
use tracing::info;
#[cfg(feature = "trace_pedantic")]
use tracing::trace;

/// TCP implementation of [`SendProtocol`]
///
/// [`SendProtocol`]: crate::SendProtocol
#[derive(Debug)]
pub struct TcpSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    buffer: BytesMut,
    store: PrioManager,
    next_mid: Mid,
    closing_streams: Vec<Sid>,
    notify_closing_streams: Vec<Sid>,
    pending_shutdown: bool,
    drain: D,
    #[allow(dead_code)]
    last: Instant,
    metrics: ProtocolMetricCache,
}

/// TCP implementation of [`RecvProtocol`]
///
/// [`RecvProtocol`]: crate::RecvProtocol
#[derive(Debug)]
pub struct TcpRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    buffer: BytesMut,
    itmsg_allocator: BytesMut,
    incoming: HashMap<Mid, ITMessage>,
    sink: S,
    metrics: ProtocolMetricCache,
}

impl<D> TcpSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    pub fn new(drain: D, metrics: ProtocolMetricCache) -> Self {
        Self {
            buffer: BytesMut::new(),
            store: PrioManager::new(metrics.clone()),
            next_mid: 0u64,
            closing_streams: vec![],
            notify_closing_streams: vec![],
            pending_shutdown: false,
            drain,
            last: Instant::now(),
            metrics,
        }
    }

    /// returns all promises that this Protocol can take care of
    /// If you open a Stream anyway, unsupported promises are ignored.
    pub fn supported_promises() -> Promises {
        Promises::ORDERED
            | Promises::CONSISTENCY
            | Promises::GUARANTEED_DELIVERY
            | Promises::COMPRESSED
    }
}

impl<S> TcpRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    pub fn new(sink: S, metrics: ProtocolMetricCache) -> Self {
        Self {
            buffer: BytesMut::new(),
            itmsg_allocator: BytesMut::with_capacity(ALLOC_BLOCK),
            incoming: HashMap::new(),
            sink,
            metrics,
        }
    }
}

#[async_trait]
impl<D> SendProtocol for TcpSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    type CustomErr = D::CustomErr;

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

    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError<Self::CustomErr>> {
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
                event.to_frame().write_bytes(&mut self.buffer);
                self.drain.send(self.buffer.split()).await?;
            },
            ProtocolEvent::CloseStream { sid } => {
                if self.store.try_close_stream(sid) {
                    event.to_frame().write_bytes(&mut self.buffer);
                    self.drain.send(self.buffer.split()).await?;
                } else {
                    #[cfg(feature = "trace_pedantic")]
                    trace!(?sid, "hold back close stream");
                    self.closing_streams.push(sid);
                }
            },
            ProtocolEvent::Shutdown => {
                if self.store.is_empty() {
                    event.to_frame().write_bytes(&mut self.buffer);
                    self.drain.send(self.buffer.split()).await?;
                } else {
                    #[cfg(feature = "trace_pedantic")]
                    trace!("hold back shutdown");
                    self.pending_shutdown = true;
                }
            },
            ProtocolEvent::Message { data, sid } => {
                self.metrics.smsg_ib(sid, data.len() as u64);
                self.store.add(data, self.next_mid, sid);
                self.next_mid += 1;
            },
        }
        Ok(())
    }

    async fn flush(
        &mut self,
        bandwidth: Bandwidth,
        dt: Duration,
    ) -> Result</* actual */ Bandwidth, ProtocolError<Self::CustomErr>> {
        let (frames, total_bytes) = self.store.grab(bandwidth, dt);
        self.buffer.reserve(total_bytes as usize);
        let mut data_frames = 0;
        let mut data_bandwidth = 0;
        for (_, frame) in frames {
            if let OTFrame::Data { mid: _, data } = &frame {
                data_bandwidth += data.len();
                data_frames += 1;
            }
            frame.write_bytes(&mut self.buffer);
        }
        self.drain.send(self.buffer.split()).await?;
        self.metrics
            .sdata_frames_b(data_frames, data_bandwidth as u64);

        let mut finished_streams = vec![];
        for (i, &sid) in self.closing_streams.iter().enumerate() {
            if self.store.try_close_stream(sid) {
                #[cfg(feature = "trace_pedantic")]
                trace!(?sid, "close stream, as it's now empty");
                OTFrame::CloseStream { sid }.write_bytes(&mut self.buffer);
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
            OTFrame::Shutdown {}.write_bytes(&mut self.buffer);
            self.drain.send(self.buffer.split()).await?;
            self.pending_shutdown = false;
        }
        Ok(data_bandwidth as u64)
    }
}

#[async_trait]
impl<S> RecvProtocol for TcpRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    type CustomErr = S::CustomErr;

    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError<Self::CustomErr>> {
        'outer: loop {
            loop {
                match ITFrame::read_frame(&mut self.buffer) {
                    Ok(Some(frame)) => {
                        #[cfg(feature = "trace_pedantic")]
                        trace!(?frame, "recv");
                        match frame {
                            ITFrame::Shutdown => break 'outer Ok(ProtocolEvent::Shutdown),
                            ITFrame::OpenStream {
                                sid,
                                prio,
                                promises,
                                guaranteed_bandwidth,
                            } => {
                                break 'outer Ok(ProtocolEvent::OpenStream {
                                    sid,
                                    prio: prio.min(crate::types::HIGHEST_PRIO),
                                    promises,
                                    guaranteed_bandwidth,
                                });
                            },
                            ITFrame::CloseStream { sid } => {
                                break 'outer Ok(ProtocolEvent::CloseStream { sid });
                            },
                            ITFrame::DataHeader { sid, mid, length } => {
                                let m = ITMessage::new(sid, length, &mut self.itmsg_allocator);
                                self.metrics.rmsg_ib(sid, length);
                                self.incoming.insert(mid, m);
                            },
                            ITFrame::Data { mid, data } => {
                                self.metrics.rdata_frames_b(data.len() as u64);
                                let m = match self.incoming.get_mut(&mid) {
                                    Some(m) => m,
                                    None => {
                                        info!(
                                            ?mid,
                                            "protocol violation by remote side: send Data before \
                                             Header"
                                        );
                                        break 'outer Err(ProtocolError::Violated);
                                    },
                                };
                                m.data.extend_from_slice(&data);
                                if m.data.len() == m.length as usize {
                                    // finished, yay
                                    let m = self.incoming.remove(&mid).unwrap();
                                    self.metrics.rmsg_ob(
                                        m.sid,
                                        RemoveReason::Finished,
                                        m.data.len() as u64,
                                    );
                                    break 'outer Ok(ProtocolEvent::Message {
                                        sid: m.sid,
                                        data: m.data.freeze(),
                                    });
                                }
                            },
                        };
                    },
                    Ok(None) => break, //inner => read more data
                    Err(()) => return Err(ProtocolError::Violated),
                }
            }
            let chunk = self.sink.recv().await?;
            if self.buffer.is_empty() {
                self.buffer = chunk;
            } else {
                self.buffer.extend_from_slice(&chunk);
            }
        }
    }
}

#[async_trait]
impl<D> ReliableDrain for TcpSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = BytesMut>,
{
    type CustomErr = D::CustomErr;

    async fn send(&mut self, frame: InitFrame) -> Result<(), ProtocolError<Self::CustomErr>> {
        let mut buffer = BytesMut::with_capacity(500);
        frame.write_bytes(&mut buffer);
        self.drain.send(buffer).await
    }
}

#[async_trait]
impl<S> ReliableSink for TcpRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = BytesMut>,
{
    type CustomErr = S::CustomErr;

    async fn recv(&mut self) -> Result<InitFrame, ProtocolError<Self::CustomErr>> {
        while self.buffer.len() < 100 {
            let chunk = self.sink.recv().await?;
            self.buffer.extend_from_slice(&chunk);
            if let Some(frame) = InitFrame::read_frame(&mut self.buffer) {
                return Ok(frame);
            }
        }
        Err(ProtocolError::Violated)
    }
}

#[cfg(test)]
mod test_utils {
    //TCP protocol based on Channel
    use super::*;
    use crate::metrics::{ProtocolMetricCache, ProtocolMetrics};
    use async_channel::*;
    use std::sync::Arc;

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
    ) -> [(TcpSendProtocol<TcpDrain>, TcpRecvProtocol<TcpSink>); 2] {
        let (s1, r1) = bounded(cap);
        let (s2, r2) = bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("tcp", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                TcpSendProtocol::new(TcpDrain { sender: s1 }, m.clone()),
                TcpRecvProtocol::new(TcpSink { receiver: r2 }, m.clone()),
            ),
            (
                TcpSendProtocol::new(TcpDrain { sender: s2 }, m.clone()),
                TcpRecvProtocol::new(TcpSink { receiver: r1 }, m),
            ),
        ]
    }

    #[async_trait]
    impl UnreliableDrain for TcpDrain {
        type CustomErr = ();
        type DataFormat = BytesMut;

        async fn send(
            &mut self,
            data: Self::DataFormat,
        ) -> Result<(), ProtocolError<Self::CustomErr>> {
            self.sender
                .send(data)
                .await
                .map_err(|_| ProtocolError::Custom(()))
        }
    }

    #[async_trait]
    impl UnreliableSink for TcpSink {
        type CustomErr = ();
        type DataFormat = BytesMut;

        async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError<Self::CustomErr>> {
            self.receiver
                .recv()
                .await
                .map_err(|_| ProtocolError::Custom(()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        error::ProtocolError,
        frame::OTFrame,
        metrics::{ProtocolMetricCache, ProtocolMetrics, RemoveReason},
        tcp::test_utils::*,
        types::{Pid, Promises, Sid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2},
        InitProtocol, ProtocolEvent, RecvProtocol, SendProtocol,
    };
    use bytes::{Bytes, BytesMut};
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
            prio: 0u8,
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
            data: Bytes::from(&[188u8; 600][..]),
        };
        s.send(event.clone()).await.unwrap();
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = r.recv().await.unwrap();
        assert_eq!(event, e);
        // 2nd short message
        let event = ProtocolEvent::Message {
            sid: Sid::new(10),
            data: Bytes::from(&[7u8; 30][..]),
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
            data: Bytes::from(&[99u8; 500_000][..]),
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
            data: Bytes::from(&[99u8; 500_000][..]),
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
            data: Bytes::from(&[99u8; 500_000][..]),
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
    async fn msg_finishes_after_drop() {
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
        let event = ProtocolEvent::Message {
            sid,
            data: Bytes::from(&[99u8; 500_000][..]),
        };
        s.send(event).await.unwrap();
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let event = ProtocolEvent::Message {
            sid,
            data: Bytes::from(&[100u8; 500_000][..]),
        };
        s.send(event).await.unwrap();
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        drop(s);
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::OpenStream { .. }));
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::Message { .. }));
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::Message { .. }));
    }

    #[tokio::test]
    async fn header_and_data_in_seperate_msg() {
        let sid = Sid::new(1);
        let (s, r) = async_channel::bounded(10);
        let m = ProtocolMetricCache::new("tcp", Arc::new(ProtocolMetrics::new().unwrap()));
        let mut r = super::TcpRecvProtocol::new(TcpSink { receiver: r }, m.clone());

        const DATA1: &[u8; 69] =
            b"We need to make sure that its okay to send OPEN_STREAM and DATA_HEAD ";
        const DATA2: &[u8; 95] = b"in one chunk and (DATA and CLOSE_STREAM) in the second chunk. and then keep the connection open";
        let mut bytes = BytesMut::with_capacity(1500);
        OTFrame::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED,
            guaranteed_bandwidth: 1_000_000,
        }
        .write_bytes(&mut bytes);
        OTFrame::DataHeader {
            mid: 99,
            sid,
            length: (DATA1.len() + DATA2.len()) as u64,
        }
        .write_bytes(&mut bytes);
        s.send(bytes.split()).await.unwrap();

        OTFrame::Data {
            mid: 99,
            data: Bytes::from(&DATA1[..]),
        }
        .write_bytes(&mut bytes);
        OTFrame::Data {
            mid: 99,
            data: Bytes::from(&DATA2[..]),
        }
        .write_bytes(&mut bytes);
        OTFrame::CloseStream { sid }.write_bytes(&mut bytes);
        s.send(bytes.split()).await.unwrap();

        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::OpenStream { .. }));

        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::Message { .. }));

        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::CloseStream { .. }));
    }

    #[tokio::test]
    async fn drop_sink_while_recv() {
        let sid = Sid::new(1);
        let (s, r) = async_channel::bounded(10);
        let m = ProtocolMetricCache::new("tcp", Arc::new(ProtocolMetrics::new().unwrap()));
        let mut r = super::TcpRecvProtocol::new(TcpSink { receiver: r }, m.clone());

        let mut bytes = BytesMut::with_capacity(1500);
        OTFrame::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED,
            guaranteed_bandwidth: 1_000_000,
        }
        .write_bytes(&mut bytes);
        s.send(bytes.split()).await.unwrap();
        let e = r.recv().await.unwrap();
        assert!(matches!(e, ProtocolEvent::OpenStream { .. }));

        let e = tokio::spawn(async move { r.recv().await });
        drop(s);

        let e = e.await.unwrap();
        assert_eq!(e, Err(ProtocolError::Custom(())));
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
            data: Bytes::from(&[188u8; 600][..]),
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
            data: Bytes::from(&[188u8; 600][..]),
        };
        p2.0.send(event.clone()).await.unwrap();
        p2.0.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        let e = p1.1.recv().await.unwrap();
        assert_eq!(event, e);
    }
}
