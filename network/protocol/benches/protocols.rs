use async_channel::*;
use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::{sync::Arc, time::Duration};
use tokio::runtime::Runtime;
use veloren_network_protocol::{
    InitProtocol, MpscMsg, MpscRecvProtocol, MpscSendProtocol, Pid, Promises, ProtocolError,
    ProtocolEvent, ProtocolMetricCache, ProtocolMetrics, QuicDataFormat, QuicRecvProtocol,
    QuicSendProtocol, RecvProtocol, SendProtocol, Sid, TcpRecvProtocol, TcpSendProtocol,
    UnreliableDrain, UnreliableSink, _internal::OTFrame,
};

fn frame_serialize(frame: OTFrame, buffer: &mut BytesMut) { frame.write_bytes(buffer); }

async fn handshake<S, R>(p: [(S, R); 2])
where
    S: SendProtocol,
    R: RecvProtocol,
    (S, R): InitProtocol,
{
    let [mut p1, mut p2] = p;
    tokio::join!(
        async {
            p1.initialize(true, Pid::fake(2), 1337).await.unwrap();
            p1
        },
        async {
            p2.initialize(false, Pid::fake(3), 42).await.unwrap();
            p2
        }
    );
}

async fn send_msg<T: SendProtocol>(mut s: T, data: Bytes, cnt: usize) {
    let bandwidth = data.len() as u64 + 100;
    const SEC1: Duration = Duration::from_secs(1);

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
            data: data.clone(),
        })
        .await
        .unwrap();
        if i.rem_euclid(50) == 0 {
            s.flush(bandwidth * 50_u64, SEC1).await.unwrap();
        }
    }
    s.flush(bandwidth * 1000_u64, SEC1).await.unwrap();
}

async fn recv_msg<T: RecvProtocol>(mut r: T, cnt: usize) {
    r.recv().await.unwrap();

    for _ in 0..cnt {
        r.recv().await.unwrap();
    }
}

async fn send_and_recv_msg<S: SendProtocol, R: RecvProtocol>(
    p: [(S, R); 2],
    data: Bytes,
    cnt: usize,
) {
    let [p1, p2] = p;
    let (s, r) = (p1.0, p2.1);

    tokio::join!(send_msg(s, data, cnt), recv_msg(r, cnt));
}

fn rt() -> Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
}

fn criterion_util(c: &mut Criterion) {
    c.bench_function("mpsc_handshake", |b| {
        b.to_async(rt())
            .iter_with_setup(|| utils::ac_bound(10, None), handshake)
    });
    c.bench_function("frame_serialize_short", |b| {
        let mut buffer = BytesMut::with_capacity(1500);
        let frame = OTFrame::Data {
            mid: 65,
            data: Bytes::from(&b"hello_world"[..]),
        };
        b.iter_with_setup(
            || frame.clone(),
            |frame| frame_serialize(frame, &mut buffer),
        )
    });
}

fn criterion_mpsc(c: &mut Criterion) {
    let mut c = c.benchmark_group("mpsc");
    c.significance_level(0.1).sample_size(10);
    c.throughput(Throughput::Bytes(1000000000))
        .bench_function("1GB_in_10000_msg", |b| {
            let buffer = Bytes::from(&[155u8; 100_000][..]);
            b.to_async(rt()).iter_with_setup(
                || (buffer.clone(), utils::ac_bound(10, None)),
                |(b, p)| send_and_recv_msg(p, b, 10_000),
            )
        });
    c.throughput(Throughput::Elements(1000000))
        .bench_function("1000000_tiny_msg", |b| {
            let buffer = Bytes::from(&[3u8; 5][..]);
            b.to_async(rt()).iter_with_setup(
                || (buffer.clone(), utils::ac_bound(10, None)),
                |(b, p)| send_and_recv_msg(p, b, 1_000_000),
            )
        });
    c.finish();
}

fn criterion_tcp(c: &mut Criterion) {
    let mut c = c.benchmark_group("tcp");
    c.significance_level(0.1).sample_size(10);
    c.throughput(Throughput::Bytes(1000000000))
        .bench_function("1GB_in_10000_msg", |b| {
            let buf = Bytes::from(&[155u8; 100_000][..]);
            b.to_async(rt()).iter_with_setup(
                || (buf.clone(), utils::tcp_bound(10000, None)),
                |(b, p)| send_and_recv_msg(p, b, 10_000),
            )
        });
    c.throughput(Throughput::Elements(1000000))
        .bench_function("1000000_tiny_msg", |b| {
            let buf = Bytes::from(&[3u8; 5][..]);
            b.to_async(rt()).iter_with_setup(
                || (buf.clone(), utils::tcp_bound(10000, None)),
                |(b, p)| send_and_recv_msg(p, b, 1_000_000),
            )
        });
    c.finish();
}

fn criterion_quic(c: &mut Criterion) {
    let mut c = c.benchmark_group("quic");
    c.significance_level(0.1).sample_size(10);
    c.throughput(Throughput::Bytes(1000000000))
        .bench_function("1GB_in_10000_msg", |b| {
            let buf = Bytes::from(&[155u8; 100_000][..]);
            b.to_async(rt()).iter_with_setup(
                || (buf.clone(), utils::quic_bound(10000, None)),
                |(b, p)| send_and_recv_msg(p, b, 10_000),
            )
        });
    c.throughput(Throughput::Elements(1000000))
        .bench_function("1000000_tiny_msg", |b| {
            let buf = Bytes::from(&[3u8; 5][..]);
            b.to_async(rt()).iter_with_setup(
                || (buf.clone(), utils::quic_bound(10000, None)),
                |(b, p)| send_and_recv_msg(p, b, 1_000_000),
            )
        });
    c.finish();
}

criterion_group!(
    benches,
    criterion_util,
    criterion_mpsc,
    criterion_tcp,
    criterion_quic
);
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
    ) -> [(MpscSendProtocol<ACDrain>, MpscRecvProtocol<ACSink>); 2] {
        let (s1, r1) = bounded(cap);
        let (s2, r2) = bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("mpsc", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                MpscSendProtocol::new(ACDrain { sender: s1 }, m.clone()),
                MpscRecvProtocol::new(ACSink { receiver: r2 }, m.clone()),
            ),
            (
                MpscSendProtocol::new(ACDrain { sender: s2 }, m.clone()),
                MpscRecvProtocol::new(ACSink { receiver: r1 }, m),
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

    pub struct QuicDrain {
        pub sender: Sender<QuicDataFormat>,
    }

    pub struct QuicSink {
        pub receiver: Receiver<QuicDataFormat>,
    }

    /// emulate Quic protocol on Channels
    pub fn quic_bound(
        cap: usize,
        metrics: Option<ProtocolMetricCache>,
    ) -> [(QuicSendProtocol<QuicDrain>, QuicRecvProtocol<QuicSink>); 2] {
        let (s1, r1) = bounded(cap);
        let (s2, r2) = bounded(cap);
        let m = metrics.unwrap_or_else(|| {
            ProtocolMetricCache::new("quic", Arc::new(ProtocolMetrics::new().unwrap()))
        });
        [
            (
                QuicSendProtocol::new(QuicDrain { sender: s1 }, m.clone()),
                QuicRecvProtocol::new(QuicSink { receiver: r2 }, m.clone()),
            ),
            (
                QuicSendProtocol::new(QuicDrain { sender: s2 }, m.clone()),
                QuicRecvProtocol::new(QuicSink { receiver: r1 }, m),
            ),
        ]
    }

    #[async_trait]
    impl UnreliableDrain for ACDrain {
        type CustomErr = ();
        type DataFormat = MpscMsg;

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
    impl UnreliableSink for ACSink {
        type CustomErr = ();
        type DataFormat = MpscMsg;

        async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError<Self::CustomErr>> {
            self.receiver
                .recv()
                .await
                .map_err(|_| ProtocolError::Custom(()))
        }
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

    #[async_trait]
    impl UnreliableDrain for QuicDrain {
        type CustomErr = ();
        type DataFormat = QuicDataFormat;

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
