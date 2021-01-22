use async_trait::async_trait;
use network_protocol::{
    InitProtocolError, MpscMsg, MpscRecvProtcol, MpscSendProtcol, Pid, ProtocolError,
    ProtocolEvent, ProtocolMetricCache, ProtocolMetrics, Sid, TcpRecvProtcol, TcpSendProtcol,
    UnreliableDrain, UnreliableSink,
};
#[cfg(feature = "metrics")] use std::sync::Arc;
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    sync::mpsc,
};

#[derive(Debug)]
pub(crate) enum Protocols {
    Tcp((TcpSendProtcol<TcpDrain>, TcpRecvProtcol<TcpSink>)),
    Mpsc((MpscSendProtcol<MpscDrain>, MpscRecvProtcol<MpscSink>)),
}

#[derive(Debug)]
pub(crate) enum SendProtocols {
    Tcp(TcpSendProtcol<TcpDrain>),
    Mpsc(MpscSendProtcol<MpscDrain>),
}

#[derive(Debug)]
pub(crate) enum RecvProtocols {
    Tcp(TcpRecvProtcol<TcpSink>),
    Mpsc(MpscRecvProtcol<MpscSink>),
}

impl Protocols {
    pub(crate) fn new_tcp(stream: tokio::net::TcpStream) -> Self {
        let (r, w) = stream.into_split();
        #[cfg(feature = "metrics")]
        let metrics = ProtocolMetricCache::new(
            "foooobaaaarrrrrrrr",
            Arc::new(ProtocolMetrics::new().unwrap()),
        );
        #[cfg(not(feature = "metrics"))]
        let metrics = ProtocolMetricCache {};

        let sp = TcpSendProtcol::new(TcpDrain { half: w }, metrics.clone());
        let rp = TcpRecvProtcol::new(TcpSink { half: r }, metrics.clone());
        Protocols::Tcp((sp, rp))
    }

    pub(crate) fn new_mpsc(
        sender: mpsc::Sender<MpscMsg>,
        receiver: mpsc::Receiver<MpscMsg>,
    ) -> Self {
        #[cfg(feature = "metrics")]
        let metrics =
            ProtocolMetricCache::new("mppppsssscccc", Arc::new(ProtocolMetrics::new().unwrap()));
        #[cfg(not(feature = "metrics"))]
        let metrics = ProtocolMetricCache {};

        let sp = MpscSendProtcol::new(MpscDrain { sender }, metrics.clone());
        let rp = MpscRecvProtcol::new(MpscSink { receiver }, metrics.clone());
        Protocols::Mpsc((sp, rp))
    }

    pub(crate) fn split(self) -> (SendProtocols, RecvProtocols) {
        match self {
            Protocols::Tcp((s, r)) => (SendProtocols::Tcp(s), RecvProtocols::Tcp(r)),
            Protocols::Mpsc((s, r)) => (SendProtocols::Mpsc(s), RecvProtocols::Mpsc(r)),
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
        }
    }
}

#[async_trait]
impl network_protocol::SendProtocol for SendProtocols {
    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError> {
        match self {
            SendProtocols::Tcp(s) => s.send(event).await,
            SendProtocols::Mpsc(s) => s.send(event).await,
        }
    }

    async fn flush(&mut self, bandwidth: u64, dt: Duration) -> Result<(), ProtocolError> {
        match self {
            SendProtocols::Tcp(s) => s.flush(bandwidth, dt).await,
            SendProtocols::Mpsc(s) => s.flush(bandwidth, dt).await,
        }
    }
}

#[async_trait]
impl network_protocol::RecvProtocol for RecvProtocols {
    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError> {
        match self {
            RecvProtocols::Tcp(r) => r.recv().await,
            RecvProtocols::Mpsc(r) => r.recv().await,
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
}

#[async_trait]
impl UnreliableDrain for TcpDrain {
    type DataFormat = Vec<u8>;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError> {
        //self.half.recv
        match self.half.write_all(&data).await {
            Ok(()) => Ok(()),
            Err(_) => Err(ProtocolError::Closed),
        }
    }
}

#[async_trait]
impl UnreliableSink for TcpSink {
    type DataFormat = Vec<u8>;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError> {
        let mut data = vec![0u8; 1500];
        match self.half.read(&mut data).await {
            Ok(n) => {
                data.truncate(n);
                Ok(data)
            },
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

#[cfg(test)]
mod tests {
    use super::*;
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
        let client = Protocols::new_tcp(client);
        let server = Protocols::new_tcp(server);
        let (mut s, _) = client.split();
        let (_, mut r) = server.split();
        let event = ProtocolEvent::OpenStream {
            sid: Sid::new(1),
            prio: 4u8,
            promises: Promises::GUARANTEED_DELIVERY,
            guaranteed_bandwidth: 1_000,
        };
        s.send(event.clone()).await.unwrap();
        let r = r.recv().await;
        match r {
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
                panic!("wrong type {:?}", r);
            },
        }
    }
}
