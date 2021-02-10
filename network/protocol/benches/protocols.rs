use async_channel::*;
use async_trait::async_trait;
use bytes::BytesMut;
use criterion::{criterion_group, criterion_main, Criterion};
use std::{sync::Arc, time::Duration};
use veloren_network_protocol::{
    InitProtocol, MessageBuffer, MpscMsg, MpscRecvProtcol, MpscSendProtcol, Pid, Promises,
    ProtocolError, ProtocolEvent, ProtocolMetricCache, ProtocolMetrics, RecvProtocol, SendProtocol,
    Sid, TcpRecvProtcol, TcpSendProtcol, UnreliableDrain, UnreliableSink, _internal::Frame,
};

fn frame_serialize(frame: Frame, buffer: &mut BytesMut) { frame.to_bytes(buffer); }

async fn mpsc_msg(buffer: Arc<MessageBuffer>) {
    // Arrrg, need to include constructor here
    let [p1, p2] = utils::ac_bound(10, None);
    let (mut s, mut r) = (p1.0, p2.1);
    s.send(ProtocolEvent::Message {
        sid: Sid::new(12),
        mid: 0,
        buffer,
    })
    .await
    .unwrap();
    r.recv().await.unwrap();
}

async fn mpsc_handshake() {
    let [mut p1, mut p2] = utils::ac_bound(10, None);
    let r1 = tokio::spawn(async move {
        p1.initialize(true, Pid::fake(2), 1337).await.unwrap();
        p1
    });
    let r2 = tokio::spawn(async move {
        p2.initialize(false, Pid::fake(3), 42).await.unwrap();
        p2
    });
    let (r1, r2) = tokio::join!(r1, r2);
    r1.unwrap();
    r2.unwrap();
}

async fn tcp_msg(buffer: Arc<MessageBuffer>, cnt: usize) {
    let [p1, p2] = utils::tcp_bound(10000, None); /*10kbit*/
    let (mut s, mut r) = (p1.0, p2.1);

    let buffer = Arc::clone(&buffer);
    let bandwidth = buffer.data.len() as u64 + 1000;

    let r1 = tokio::spawn(async move {
        s.send(ProtocolEvent::OpenStream {
            sid: Sid::new(12),
            prio: 0,
            promises: Promises::ORDERED,
            guaranteed_bandwidth: 100_000,
        })
        .await
        .unwrap();

        for i in 0..cnt {
            s.send(ProtocolEvent::Message {
                sid: Sid::new(12),
                mid: i as u64,
                buffer: Arc::clone(&buffer),
            })
            .await
            .unwrap();
            s.flush(bandwidth, Duration::from_secs(1)).await.unwrap();
        }
    });
    let r2 = tokio::spawn(async move {
        r.recv().await.unwrap();

        for _ in 0..cnt {
            r.recv().await.unwrap();
        }
    });
    let (r1, r2) = tokio::join!(r1, r2);
    r1.unwrap();
    r2.unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = || {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
    };

    c.bench_function("mpsc_short_msg", |b| {
        let buffer = Arc::new(MessageBuffer {
            data: b"hello_world".to_vec(),
        });
        b.to_async(rt()).iter(|| mpsc_msg(Arc::clone(&buffer)))
    });
    c.bench_function("mpsc_long_msg", |b| {
        let buffer = Arc::new(MessageBuffer {
            data: vec![150u8; 500_000],
        });
        b.to_async(rt()).iter(|| mpsc_msg(Arc::clone(&buffer)))
    });
    c.bench_function("mpsc_handshake", |b| {
        b.to_async(rt()).iter(|| mpsc_handshake())
    });

    let mut buffer = BytesMut::with_capacity(1500);

    c.bench_function("frame_serialize_short", |b| {
        let frame = Frame::Data {
            mid: 65,
            start: 89u64,
            data: b"hello_world".to_vec(),
        };
        b.iter(|| frame_serialize(frame.clone(), &mut buffer))
    });

    c.bench_function("tcp_short_msg", |b| {
        let buffer = Arc::new(MessageBuffer {
            data: b"hello_world".to_vec(),
        });
        b.to_async(rt()).iter(|| tcp_msg(Arc::clone(&buffer), 1))
    });
    c.bench_function("tcp_1GB_in_10000_msg", |b| {
        let buffer = Arc::new(MessageBuffer {
            data: vec![155u8; 100_000],
        });
        b.to_async(rt())
            .iter(|| tcp_msg(Arc::clone(&buffer), 10_000))
    });
    c.bench_function("tcp_1000000_tiny_msg", |b| {
        let buffer = Arc::new(MessageBuffer { data: vec![3u8; 5] });
        b.to_async(rt())
            .iter(|| tcp_msg(Arc::clone(&buffer), 1_000_000))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

mod utils {
    use super::*;

    pub struct ACDrain {
        sender: Sender<MpscMsg>,
    }

    pub struct ACSink {
        receiver: Receiver<MpscMsg>,
    }

    pub fn ac_bound(
        cap: usize,
        metrics: Option<ProtocolMetricCache>,
    ) -> [(MpscSendProtcol<ACDrain>, MpscRecvProtcol<ACSink>); 2] {
        let (s1, r1) = async_channel::bounded(cap);
        let (s2, r2) = async_channel::bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("mpsc", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                MpscSendProtcol::new(ACDrain { sender: s1 }, m.clone()),
                MpscRecvProtcol::new(ACSink { receiver: r2 }, m.clone()),
            ),
            (
                MpscSendProtcol::new(ACDrain { sender: s2 }, m.clone()),
                MpscRecvProtcol::new(ACSink { receiver: r1 }, m.clone()),
            ),
        ]
    }

    pub struct TcpDrain {
        sender: Sender<BytesMut>,
    }

    pub struct TcpSink {
        receiver: Receiver<BytesMut>,
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
    impl UnreliableDrain for ACDrain {
        type DataFormat = MpscMsg;

        async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
            self.sender
                .send(data)
                .await
                .map_err(|_| ProtocolError::Closed)
        }
    }

    #[async_trait]
    impl UnreliableSink for ACSink {
        type DataFormat = MpscMsg;

        async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
            self.receiver
                .recv()
                .await
                .map_err(|_| ProtocolError::Closed)
        }
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
