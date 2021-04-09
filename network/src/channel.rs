use async_trait::async_trait;
use bytes::BytesMut;
use network_protocol::{
    Bandwidth, Cid, InitProtocolError, MpscMsg, MpscRecvProtocol, MpscSendProtocol, Pid,
    ProtocolError, ProtocolEvent, ProtocolMetricCache, ProtocolMetrics, Sid, TcpRecvProtocol,
    TcpSendProtocol, UnreliableDrain, UnreliableSink,
};
#[cfg(feature = "quic")] use quinn::*;
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::mpsc,
};

#[derive(Debug)]
pub(crate) enum Protocols {
    Tcp((TcpSendProtocol<TcpDrain>, TcpRecvProtocol<TcpSink>)),
    Mpsc((MpscSendProtocol<MpscDrain>, MpscRecvProtocol<MpscSink>)),
    #[cfg(feature = "quic")]
    Quic((QuicSendProtocol<QuicDrain>, QuicRecvProtocol<QuicSink>)),
}

#[derive(Debug)]
pub(crate) enum SendProtocols {
    Tcp(TcpSendProtocol<TcpDrain>),
    Mpsc(MpscSendProtocol<MpscDrain>),
    #[cfg(feature = "quic")]
    Quic(QuicSendProtocol<QuicDrain>),
}

#[derive(Debug)]
pub(crate) enum RecvProtocols {
    Tcp(TcpRecvProtocol<TcpSink>),
    Mpsc(MpscRecvProtocol<MpscSink>),
    #[cfg(feature = "quic")]
    Quic(QuicSendProtocol<QuicDrain>),
}

impl Protocols {
    pub(crate) fn new_tcp(
        stream: tokio::net::TcpStream,
        cid: Cid,
        metrics: Arc<ProtocolMetrics>,
    ) -> Self {
        let (r, w) = stream.into_split();
        let metrics = ProtocolMetricCache::new(&cid.to_string(), metrics);

        let sp = TcpSendProtocol::new(TcpDrain { half: w }, metrics.clone());
        let rp = TcpRecvProtocol::new(
            TcpSink {
                half: r,
                buffer: BytesMut::new(),
            },
            metrics,
        );
        Protocols::Tcp((sp, rp))
    }

    pub(crate) fn new_mpsc(
        sender: mpsc::Sender<MpscMsg>,
        receiver: mpsc::Receiver<MpscMsg>,
        cid: Cid,
        metrics: Arc<ProtocolMetrics>,
    ) -> Self {
        let metrics = ProtocolMetricCache::new(&cid.to_string(), metrics);

        let sp = MpscSendProtocol::new(MpscDrain { sender }, metrics.clone());
        let rp = MpscRecvProtocol::new(MpscSink { receiver }, metrics);
        Protocols::Mpsc((sp, rp))
    }

    pub(crate) fn split(self) -> (SendProtocols, RecvProtocols) {
        match self {
            Protocols::Tcp((s, r)) => (SendProtocols::Tcp(s), RecvProtocols::Tcp(r)),
            Protocols::Mpsc((s, r)) => (SendProtocols::Mpsc(s), RecvProtocols::Mpsc(r)),
            #[cfg(feature = "quic")]
            Protocols::Quic((s, r)) => (SendProtocols::Quic(s), RecvProtocols::Quic(r)),
        }
    }
}

#[async_trait]
impl network_protocol::InitProtocol for Protocols {
    async fn initialize(
        &mut self,
        initializer: bool,
        local_pid: Pid,
        secret: u128,
    ) -> Result<(Pid, Sid, u128), InitProtocolError> {
        match self {
            Protocols::Tcp(p) => p.initialize(initializer, local_pid, secret).await,
            Protocols::Mpsc(p) => p.initialize(initializer, local_pid, secret).await,
            #[cfg(feature = "quic")]
            Protocols::Quic(p) => p.initialize(initializer, local_pid, secret).await,
        }
    }
}

#[async_trait]
impl network_protocol::SendProtocol for SendProtocols {
    fn notify_from_recv(&mut self, event: ProtocolEvent) {
        match self {
            SendProtocols::Tcp(s) => s.notify_from_recv(event),
            SendProtocols::Mpsc(s) => s.notify_from_recv(event),
            #[cfg(feature = "quic")]
            SendProtocols::Quic(s) => s.notify_from_recv(event),
        }
    }

    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError> {
        match self {
            SendProtocols::Tcp(s) => s.send(event).await,
            SendProtocols::Mpsc(s) => s.send(event).await,
            #[cfg(feature = "quic")]
            SendProtocols::Quic(s) => s.send(event).await,
        }
    }

    async fn flush(
        &mut self,
        bandwidth: Bandwidth,
        dt: Duration,
    ) -> Result<Bandwidth, ProtocolError> {
        match self {
            SendProtocols::Tcp(s) => s.flush(bandwidth, dt).await,
            SendProtocols::Mpsc(s) => s.flush(bandwidth, dt).await,
            #[cfg(feature = "quic")]
            SendProtocols::Quic(s) => s.flush(bandwidth, dt).await,
        }
    }
}

#[async_trait]
impl network_protocol::RecvProtocol for RecvProtocols {
    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError> {
        match self {
            RecvProtocols::Tcp(r) => r.recv().await,
            RecvProtocols::Mpsc(r) => r.recv().await,
            #[cfg(feature = "quic")]
            RecvProtocols::Quic(r) => r.recv().await,
        }
    }
}

///////////////////////////////////////
//// TCP
#[derive(Debug)]
pub struct TcpDrain {
    half: OwnedWriteHalf,
}

#[derive(Debug)]
pub struct TcpSink {
    half: OwnedReadHalf,
    buffer: BytesMut,
}

#[async_trait]
impl UnreliableDrain for TcpDrain {
    type DataFormat = BytesMut;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
        match self.half.write_all(&data).await {
            Ok(()) => Ok(()),
            Err(_) => Err(ProtocolError::Closed),
        }
    }
}

#[async_trait]
impl UnreliableSink for TcpSink {
    type DataFormat = BytesMut;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
        self.buffer.resize(1500, 0u8);
        match self.half.read(&mut self.buffer).await {
            Ok(0) => Err(ProtocolError::Closed),
            Ok(n) => Ok(self.buffer.split_to(n)),
            Err(_) => Err(ProtocolError::Closed),
        }
    }
}

///////////////////////////////////////
//// MPSC
#[derive(Debug)]
pub struct MpscDrain {
    sender: tokio::sync::mpsc::Sender<MpscMsg>,
}

#[derive(Debug)]
pub struct MpscSink {
    receiver: tokio::sync::mpsc::Receiver<MpscMsg>,
}

#[async_trait]
impl UnreliableDrain for MpscDrain {
    type DataFormat = MpscMsg;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
        self.sender
            .send(data)
            .await
            .map_err(|_| ProtocolError::Closed)
    }
}

#[async_trait]
impl UnreliableSink for MpscSink {
    type DataFormat = MpscMsg;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
        self.receiver.recv().await.ok_or(ProtocolError::Closed)
    }
}

///////////////////////////////////////
//// QUIC
#[derive(Debug)]
pub struct QuicDrain {
    half: OwnedWriteHalf,
}

#[derive(Debug)]
pub struct QuicSink {
    half: OwnedReadHalf,
    buffer: BytesMut,
}

#[async_trait]
impl UnreliableDrain for QuicDrain {
    type DataFormat = BytesMut;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
        match self.half.write_all(&data).await {
            Ok(()) => Ok(()),
            Err(_) => Err(ProtocolError::Closed),
        }
    }
}

#[async_trait]
impl UnreliableSink for QuicSink {
    type DataFormat = BytesMut;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
        self.buffer.resize(1500, 0u8);
        match self.half.read(&mut self.buffer).await {
            Ok(0) => Err(ProtocolError::Closed),
            Ok(n) => Ok(self.buffer.split_to(n)),
            Err(_) => Err(ProtocolError::Closed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use network_protocol::{Promises, RecvProtocol, SendProtocol};
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn tokio_sinks() {
        let listener = TcpListener::bind("127.0.0.1:5000").await.unwrap();
        let r1 = tokio::spawn(async move {
            let (server, _) = listener.accept().await.unwrap();
            (listener, server)
        });
        let client = TcpStream::connect("127.0.0.1:5000").await.unwrap();
        let (_listener, server) = r1.await.unwrap();
        let metrics = Arc::new(ProtocolMetrics::new().unwrap());
        let client = Protocols::new_tcp(client, 0, Arc::clone(&metrics));
        let server = Protocols::new_tcp(server, 0, Arc::clone(&metrics));
        let (mut s, _) = client.split();
        let (_, mut r) = server.split();
        let event = ProtocolEvent::OpenStream {
            sid: Sid::new(1),
            prio: 4u8,
            promises: Promises::GUARANTEED_DELIVERY,
            guaranteed_bandwidth: 1_000,
        };
        s.send(event.clone()).await.unwrap();
        s.send(ProtocolEvent::Message {
            sid: Sid::new(1),
            data: Bytes::from(&[8u8; 8][..]),
        })
        .await
        .unwrap();
        s.flush(1_000_000, Duration::from_secs(1)).await.unwrap();
        drop(s); // recv must work even after shutdown of send!
        tokio::time::sleep(Duration::from_secs(1)).await;
        let res = r.recv().await;
        match res {
            Ok(ProtocolEvent::OpenStream {
                sid,
                prio,
                promises,
                guaranteed_bandwidth: _,
            }) => {
                assert_eq!(sid, Sid::new(1));
                assert_eq!(prio, 4u8);
                assert_eq!(promises, Promises::GUARANTEED_DELIVERY);
            },
            _ => {
                panic!("wrong type {:?}", res);
            },
        }
        r.recv().await.unwrap();
    }

    #[tokio::test]
    async fn tokio_sink_stop_after_drop() {
        let listener = TcpListener::bind("127.0.0.1:5001").await.unwrap();
        let r1 = tokio::spawn(async move {
            let (server, _) = listener.accept().await.unwrap();
            (listener, server)
        });
        let client = TcpStream::connect("127.0.0.1:5001").await.unwrap();
        let (_listener, server) = r1.await.unwrap();
        let metrics = Arc::new(ProtocolMetrics::new().unwrap());
        let client = Protocols::new_tcp(client, 0, Arc::clone(&metrics));
        let server = Protocols::new_tcp(server, 0, Arc::clone(&metrics));
        let (s, _) = client.split();
        let (_, mut r) = server.split();
        let e = tokio::spawn(async move { r.recv().await });
        drop(s);
        let e = e.await.unwrap();
        assert!(e.is_err());
        assert_eq!(e.unwrap_err(), ProtocolError::Closed);
    }
}
