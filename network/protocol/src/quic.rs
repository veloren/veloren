use crate::{
    error::ProtocolError,
    event::ProtocolEvent,
    frame::{ITFrame, InitFrame, OTFrame},
    handshake::{ReliableDrain, ReliableSink},
    message::{ITMessage, ALLOC_BLOCK},
    metrics::{ProtocolMetricCache, RemoveReason},
    prio::PrioManager,
    types::{Bandwidth, Mid, Promises, Sid},
    util::SortedVec,
    RecvProtocol, SendProtocol, UnreliableDrain, UnreliableSink,
};
use async_trait::async_trait;
use bytes::BytesMut;
use hashbrown::HashMap;
use std::time::{Duration, Instant};
use tracing::info;
#[cfg(feature = "trace_pedantic")]
use tracing::trace;

#[derive(PartialEq, Eq)]
pub enum QuicDataFormatStream {
    Main,
    Reliable(Sid),
    Unreliable,
}

pub struct QuicDataFormat {
    pub stream: QuicDataFormatStream,
    pub data: BytesMut,
}

impl QuicDataFormat {
    fn with_main(buffer: &mut BytesMut) -> Self {
        Self {
            stream: QuicDataFormatStream::Main,
            data: buffer.split(),
        }
    }

    fn with_reliable(buffer: &mut BytesMut, sid: Sid) -> Self {
        Self {
            stream: QuicDataFormatStream::Reliable(sid),
            data: buffer.split(),
        }
    }

    fn with_unreliable(frame: OTFrame) -> Self {
        let mut buffer = BytesMut::new();
        frame.write_bytes(&mut buffer);
        Self {
            stream: QuicDataFormatStream::Unreliable,
            data: buffer,
        }
    }
}

/// QUIC implementation of [`SendProtocol`]
///
/// [`SendProtocol`]: crate::SendProtocol
#[derive(Debug)]
pub struct QuicSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = QuicDataFormat>,
{
    main_buffer: BytesMut,
    reliable_buffers: SortedVec<Sid, BytesMut>,
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

/// QUIC implementation of [`RecvProtocol`]
///
/// [`RecvProtocol`]: crate::RecvProtocol
#[derive(Debug)]
pub struct QuicRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = QuicDataFormat>,
{
    main_buffer: BytesMut,
    unreliable_buffer: BytesMut,
    reliable_buffers: SortedVec<Sid, BytesMut>,
    pending_reliable_buffers: Vec<(Sid, BytesMut)>,
    itmsg_allocator: BytesMut,
    incoming: HashMap<Mid, ITMessage>,
    sink: S,
    metrics: ProtocolMetricCache,
}

fn is_reliable(p: &Promises) -> bool {
    p.contains(Promises::ORDERED)
        || p.contains(Promises::CONSISTENCY)
        || p.contains(Promises::GUARANTEED_DELIVERY)
}

impl<D> QuicSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = QuicDataFormat>,
{
    pub fn new(drain: D, metrics: ProtocolMetricCache) -> Self {
        Self {
            main_buffer: BytesMut::new(),
            reliable_buffers: SortedVec::default(),
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
            | Promises::ENCRYPTED
    }
}

impl<S> QuicRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = QuicDataFormat>,
{
    pub fn new(sink: S, metrics: ProtocolMetricCache) -> Self {
        Self {
            main_buffer: BytesMut::new(),
            unreliable_buffer: BytesMut::new(),
            reliable_buffers: SortedVec::default(),
            pending_reliable_buffers: vec![],
            itmsg_allocator: BytesMut::with_capacity(ALLOC_BLOCK),
            incoming: HashMap::new(),
            sink,
            metrics,
        }
    }

    async fn recv_into_stream(
        &mut self,
    ) -> Result<QuicDataFormatStream, ProtocolError<S::CustomErr>> {
        let chunk = self.sink.recv().await?;
        let buffer = match chunk.stream {
            QuicDataFormatStream::Main => &mut self.main_buffer,
            QuicDataFormatStream::Unreliable => &mut self.unreliable_buffer,
            QuicDataFormatStream::Reliable(id) => {
                match self.reliable_buffers.get_mut(&id) {
                    Some(buffer) => buffer,
                    None => {
                        self.pending_reliable_buffers.push((id, BytesMut::new()));
                        //Violated but will never happen
                        &mut self
                            .pending_reliable_buffers
                            .last_mut()
                            .ok_or(ProtocolError::Violated)?
                            .1
                    },
                }
            },
        };
        if buffer.is_empty() {
            *buffer = chunk.data
        } else {
            buffer.extend_from_slice(&chunk.data)
        }
        Ok(chunk.stream)
    }
}

#[async_trait]
impl<D> SendProtocol for QuicSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = QuicDataFormat>,
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
                if is_reliable(&promises) {
                    self.reliable_buffers.insert(sid, BytesMut::new());
                }
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
                if is_reliable(&promises) {
                    self.reliable_buffers.insert(sid, BytesMut::new());
                    //Send a empty message to notify local drain of stream
                    self.drain
                        .send(QuicDataFormat::with_reliable(&mut BytesMut::new(), sid))
                        .await?;
                }
                event.to_frame().write_bytes(&mut self.main_buffer);
                self.drain
                    .send(QuicDataFormat::with_main(&mut self.main_buffer))
                    .await?;
            },
            ProtocolEvent::CloseStream { sid } => {
                if self.store.try_close_stream(sid) {
                    let _ = self.reliable_buffers.delete(&sid); //delete if it was reliable
                    event.to_frame().write_bytes(&mut self.main_buffer);
                    self.drain
                        .send(QuicDataFormat::with_main(&mut self.main_buffer))
                        .await?;
                } else {
                    #[cfg(feature = "trace_pedantic")]
                    trace!(?sid, "hold back close stream");
                    self.closing_streams.push(sid);
                }
            },
            ProtocolEvent::Shutdown => {
                if self.store.is_empty() {
                    event.to_frame().write_bytes(&mut self.main_buffer);
                    self.drain
                        .send(QuicDataFormat::with_main(&mut self.main_buffer))
                        .await?;
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
        let (frames, _) = self.store.grab(bandwidth, dt);
        //Todo: optimize reserve
        let mut data_frames = 0;
        let mut data_bandwidth = 0;
        for (sid, frame) in frames {
            if let OTFrame::Data { mid: _, data } = &frame {
                data_bandwidth += data.len();
                data_frames += 1;
            }
            match self.reliable_buffers.get_mut(&sid) {
                Some(buffer) => frame.write_bytes(buffer),
                None => {
                    self.drain
                        .send(QuicDataFormat::with_unreliable(frame))
                        .await?
                },
            }
        }
        for (sid, buffer) in self.reliable_buffers.data.iter_mut() {
            if !buffer.is_empty() {
                self.drain
                    .send(QuicDataFormat::with_reliable(buffer, *sid))
                    .await?;
            }
        }
        self.metrics
            .sdata_frames_b(data_frames, data_bandwidth as u64);

        let mut finished_streams = vec![];
        for (i, &sid) in self.closing_streams.iter().enumerate() {
            if self.store.try_close_stream(sid) {
                #[cfg(feature = "trace_pedantic")]
                trace!(?sid, "close stream, as it's now empty");
                OTFrame::CloseStream { sid }.write_bytes(&mut self.main_buffer);
                self.drain
                    .send(QuicDataFormat::with_main(&mut self.main_buffer))
                    .await?;
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
            OTFrame::Shutdown {}.write_bytes(&mut self.main_buffer);
            self.drain
                .send(QuicDataFormat::with_main(&mut self.main_buffer))
                .await?;
            self.pending_shutdown = false;
        }
        Ok(data_bandwidth as u64)
    }
}

#[async_trait]
impl<S> RecvProtocol for QuicRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = QuicDataFormat>,
{
    type CustomErr = S::CustomErr;

    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError<Self::CustomErr>> {
        'outer: loop {
            match ITFrame::read_frame(&mut self.main_buffer) {
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
                            if is_reliable(&promises) {
                                self.reliable_buffers.insert(sid, BytesMut::new());
                            }
                            break 'outer Ok(ProtocolEvent::OpenStream {
                                sid,
                                prio: prio.min(crate::types::HIGHEST_PRIO),
                                promises,
                                guaranteed_bandwidth,
                            });
                        },
                        ITFrame::CloseStream { sid } => {
                            //FIXME: defer close!
                            //let _ = self.reliable_buffers.delete(sid); // if it was reliable
                            break 'outer Ok(ProtocolEvent::CloseStream { sid });
                        },
                        _ => break 'outer Err(ProtocolError::Violated),
                    };
                },
                Ok(None) => {},
                Err(()) => return Err(ProtocolError::Violated),
            }

            // try to order pending
            let mut pending_violated = false;
            let mut reliable = vec![];
            self.pending_reliable_buffers.drain_filter(|(_, buffer)| {
                // try to get Sid without touching buffer
                let mut testbuffer = buffer.clone();
                match ITFrame::read_frame(&mut testbuffer) {
                    Ok(Some(ITFrame::DataHeader {
                        sid,
                        mid: _,
                        length: _,
                    })) => {
                        reliable.push((sid, buffer.clone()));
                        true
                    },
                    Ok(Some(_)) | Err(_) => {
                        pending_violated = true;
                        true
                    },
                    Ok(None) => false,
                }
            });

            if pending_violated {
                break 'outer Err(ProtocolError::Violated);
            }
            for (sid, buffer) in reliable.into_iter() {
                self.reliable_buffers.insert(sid, buffer)
            }

            let mut iter = self
                .reliable_buffers
                .data
                .iter_mut()
                .map(|(_, b)| (b, true))
                .collect::<Vec<_>>();
            iter.push((&mut self.unreliable_buffer, false));

            for (buffer, reliable) in iter {
                loop {
                    match ITFrame::read_frame(buffer) {
                        Ok(Some(frame)) => {
                            #[cfg(feature = "trace_pedantic")]
                            trace!(?frame, "recv");
                            match frame {
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
                                            if reliable {
                                                info!(
                                                    ?mid,
                                                    "protocol violation by remote side: send Data \
                                                     before Header"
                                                );
                                                break 'outer Err(ProtocolError::Violated);
                                            } else {
                                                //TODO: cleanup old messages from time to time
                                                continue;
                                            }
                                        },
                                    };
                                    m.data.extend_from_slice(&data);
                                    if m.data.len() == m.length as usize {
                                        // finished, yay
                                        let m = self
                                            .incoming
                                            .remove(&mid)
                                            .ok_or(ProtocolError::Violated)?;
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
                                _ => break 'outer Err(ProtocolError::Violated),
                            };
                        },
                        Ok(None) => break, //inner => read more data
                        Err(()) => return Err(ProtocolError::Violated),
                    }
                }
            }

            self.recv_into_stream().await?;
        }
    }
}

#[async_trait]
impl<D> ReliableDrain for QuicSendProtocol<D>
where
    D: UnreliableDrain<DataFormat = QuicDataFormat>,
{
    type CustomErr = D::CustomErr;

    async fn send(&mut self, frame: InitFrame) -> Result<(), ProtocolError<Self::CustomErr>> {
        self.main_buffer.reserve(500);
        frame.write_bytes(&mut self.main_buffer);
        self.drain
            .send(QuicDataFormat::with_main(&mut self.main_buffer))
            .await
    }
}

#[async_trait]
impl<S> ReliableSink for QuicRecvProtocol<S>
where
    S: UnreliableSink<DataFormat = QuicDataFormat>,
{
    type CustomErr = S::CustomErr;

    async fn recv(&mut self) -> Result<InitFrame, ProtocolError<Self::CustomErr>> {
        while self.main_buffer.len() < 100 {
            if self.recv_into_stream().await? == QuicDataFormatStream::Main {
                if let Some(frame) = InitFrame::read_frame(&mut self.main_buffer) {
                    return Ok(frame);
                }
            }
        }
        Err(ProtocolError::Violated)
    }
}

#[cfg(test)]
mod test_utils {
    //Quic protocol based on Channel
    use super::*;
    use crate::metrics::{ProtocolMetricCache, ProtocolMetrics};
    use async_channel::*;
    use std::sync::Arc;

    pub struct QuicDrain {
        pub sender: Sender<QuicDataFormat>,
        pub drop_ratio: f32,
    }

    pub struct QuicSink {
        pub receiver: Receiver<QuicDataFormat>,
    }

    /// emulate Quic protocol on Channels
    pub fn quic_bound(
        cap: usize,
        drop_ratio: f32,
        metrics: Option<ProtocolMetricCache>,
    ) -> [(QuicSendProtocol<QuicDrain>, QuicRecvProtocol<QuicSink>); 2] {
        let (s1, r1) = bounded(cap);
        let (s2, r2) = bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("quic", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                QuicSendProtocol::new(
                    QuicDrain {
                        sender: s1,
                        drop_ratio,
                    },
                    m.clone(),
                ),
                QuicRecvProtocol::new(QuicSink { receiver: r2 }, m.clone()),
            ),
            (
                QuicSendProtocol::new(
                    QuicDrain {
                        sender: s2,
                        drop_ratio,
                    },
                    m.clone(),
                ),
                QuicRecvProtocol::new(QuicSink { receiver: r1 }, m),
            ),
        ]
    }

    #[async_trait]
    impl UnreliableDrain for QuicDrain {
        type CustomErr = ();
        type DataFormat = QuicDataFormat;

        async fn send(
            &mut self,
            data: Self::DataFormat,
        ) -> Result<(), ProtocolError<Self::CustomErr>> {
            use rand::Rng;
            if matches!(data.stream, QuicDataFormatStream::Unreliable)
                && rand::thread_rng().gen::<f32>() < self.drop_ratio
            {
                return Ok(());
            }
            self.sender
                .send(data)
                .await
                .map_err(|_| ProtocolError::Custom(()))
        }
    }

    #[async_trait]
    impl UnreliableSink for QuicSink {
        type CustomErr = ();
        type DataFormat = QuicDataFormat;

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
        quic::{test_utils::*, QuicDataFormat},
        types::{Pid, Promises, Sid, STREAM_ID_OFFSET1, STREAM_ID_OFFSET2},
        InitProtocol, ProtocolEvent, RecvProtocol, SendProtocol,
    };
    use bytes::{Bytes, BytesMut};
    use std::{sync::Arc, time::Duration};

    #[tokio::test]
    async fn handshake_all_good() {
        let [mut p1, mut p2] = quic_bound(10, 0.5, None);
        let r1 = tokio::spawn(async move { p1.initialize(true, Pid::fake(2), 1337).await });
        let r2 = tokio::spawn(async move { p2.initialize(false, Pid::fake(3), 42).await });
        let (r1, r2) = tokio::join!(r1, r2);
        assert_eq!(r1.unwrap(), Ok((Pid::fake(3), STREAM_ID_OFFSET1, 42)));
        assert_eq!(r2.unwrap(), Ok((Pid::fake(2), STREAM_ID_OFFSET2, 1337)));
    }

    #[tokio::test]
    async fn open_stream() {
        let [p1, p2] = quic_bound(10, 0.5, None);
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
        let [p1, p2] = quic_bound(10, 0.5, None);
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
            ProtocolMetricCache::new("long_quic", Arc::new(ProtocolMetrics::new().unwrap()));
        let sid = Sid::new(1);
        let [p1, p2] = quic_bound(10000, 0.5, Some(metrics.clone()));
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED | Promises::ORDERED,
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
        let [p1, p2] = quic_bound(10000, 0.5, None);
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED | Promises::ORDERED,
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
        let [p1, p2] = quic_bound(10000, 0.5, None);
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED | Promises::ORDERED,
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
        let [p1, p2] = quic_bound(10000, 0.5, None);
        let (mut s, mut r) = (p1.0, p2.1);
        let event = ProtocolEvent::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED | Promises::ORDERED,
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
        let m = ProtocolMetricCache::new("quic", Arc::new(ProtocolMetrics::new().unwrap()));
        let mut r = super::QuicRecvProtocol::new(QuicSink { receiver: r }, m.clone());

        const DATA1: &[u8; 69] =
            b"We need to make sure that its okay to send OPEN_STREAM and DATA_HEAD ";
        const DATA2: &[u8; 95] = b"in one chunk and (DATA and CLOSE_STREAM) in the second chunk. and then keep the connection open";
        let mut bytes = BytesMut::with_capacity(1500);
        OTFrame::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED | Promises::ORDERED,
            guaranteed_bandwidth: 1_000_000,
        }
        .write_bytes(&mut bytes);
        s.send(QuicDataFormat::with_main(&mut bytes)).await.unwrap();

        OTFrame::DataHeader {
            mid: 99,
            sid,
            length: (DATA1.len() + DATA2.len()) as u64,
        }
        .write_bytes(&mut bytes);
        s.send(QuicDataFormat::with_reliable(&mut bytes, sid))
            .await
            .unwrap();

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
        s.send(QuicDataFormat::with_reliable(&mut bytes, sid))
            .await
            .unwrap();

        OTFrame::CloseStream { sid }.write_bytes(&mut bytes);
        s.send(QuicDataFormat::with_main(&mut bytes)).await.unwrap();

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
        let m = ProtocolMetricCache::new("quic", Arc::new(ProtocolMetrics::new().unwrap()));
        let mut r = super::QuicRecvProtocol::new(QuicSink { receiver: r }, m.clone());

        let mut bytes = BytesMut::with_capacity(1500);
        OTFrame::OpenStream {
            sid,
            prio: 5u8,
            promises: Promises::COMPRESSED,
            guaranteed_bandwidth: 1_000_000,
        }
        .write_bytes(&mut bytes);
        s.send(QuicDataFormat::with_main(&mut bytes)).await.unwrap();
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
        let [mut p1, mut p2] = quic_bound(10, 0.5, None);
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
        let [mut p1, mut p2] = quic_bound(10, 0.5, None);
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

    #[tokio::test]
    async fn unrealiable_test() {
        const MIN_CHECK: usize = 10;
        const COUNT: usize = 10_000;
        //We send COUNT msg with 50% of be send each. we check that >= MIN_CHECK && !=
        // COUNT reach their target

        let [mut p1, mut p2] = quic_bound(
            COUNT * 2 - 1, /* 2 times as it is HEADER + DATA but -1 as we want to see not all
                            * succeed */
            0.5,
            None,
        );
        let event = ProtocolEvent::OpenStream {
            sid: Sid::new(1337),
            prio: 3u8,
            promises: Promises::empty(), /* on purpose! */
            guaranteed_bandwidth: 1_000_000,
        };
        p1.0.send(event).await.unwrap();
        let e = p2.1.recv().await.unwrap();
        p2.0.notify_from_recv(e);
        let event = ProtocolEvent::Message {
            sid: Sid::new(1337),
            data: Bytes::from(&[188u8; 600][..]),
        };
        for _ in 0..COUNT {
            p2.0.send(event.clone()).await.unwrap();
        }
        p2.0.flush(1_000_000_000, Duration::from_secs(1))
            .await
            .unwrap();
        for _ in 0..COUNT {
            p2.0.send(event.clone()).await.unwrap();
        }
        for _ in 0..MIN_CHECK {
            let e = p1.1.recv().await.unwrap();
            assert_eq!(event, e);
        }
    }
}
