use crate::api::{ConnectAddr, NetworkConnectError};
use async_trait::async_trait;
use bytes::BytesMut;
use futures_util::FutureExt;
#[cfg(feature = "quic")]
use futures_util::StreamExt;
use hashbrown::HashMap;
use network_protocol::{
    Bandwidth, Cid, InitProtocolError, MpscMsg, MpscRecvProtocol, MpscSendProtocol, Pid,
    ProtocolError, ProtocolEvent, ProtocolMetricCache, ProtocolMetrics, Sid, TcpRecvProtocol,
    TcpSendProtocol, UnreliableDrain, UnreliableSink,
};
#[cfg(feature = "quic")]
use network_protocol::{QuicDataFormat, QuicDataFormatStream, QuicRecvProtocol, QuicSendProtocol};
use std::{
    io,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net,
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    select,
    sync::{mpsc, oneshot, Mutex},
};
use tracing::{error, info, trace, warn};

#[allow(clippy::large_enum_variant)]
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
    Quic(QuicRecvProtocol<QuicSink>),
}

lazy_static::lazy_static! {
    pub(crate) static ref MPSC_POOL: Mutex<HashMap<u64, mpsc::UnboundedSender<C2cMpscConnect>>> = {
        Mutex::new(HashMap::new())
    };
}

pub(crate) type C2cMpscConnect = (
    mpsc::Sender<MpscMsg>,
    oneshot::Sender<mpsc::Sender<MpscMsg>>,
);
pub(crate) type C2sProtocol = (Protocols, ConnectAddr, Cid);

impl Protocols {
    const MPSC_CHANNEL_BOUND: usize = 1000;

    pub(crate) async fn with_tcp_connect(
        addr: SocketAddr,
        metrics: ProtocolMetricCache,
    ) -> Result<Self, NetworkConnectError> {
        let stream = net::TcpStream::connect(addr)
            .await
            .and_then(|s| {
                s.set_nodelay(true)?;
                Ok(s)
            })
            .map_err(NetworkConnectError::Io)?;
        info!(
            "Connecting Tcp to: {}",
            stream.peer_addr().map_err(NetworkConnectError::Io)?
        );
        Ok(Self::new_tcp(stream, metrics))
    }

    pub(crate) async fn with_tcp_listen(
        addr: SocketAddr,
        cids: Arc<AtomicU64>,
        metrics: Arc<ProtocolMetrics>,
        s2s_stop_listening_r: oneshot::Receiver<()>,
        c2s_protocol_s: mpsc::UnboundedSender<C2sProtocol>,
    ) -> io::Result<()> {
        use socket2::{Domain, Socket, Type};
        let domain = Domain::for_address(addr);
        let socket2_socket = Socket::new(domain, Type::STREAM, None)?;
        if domain == Domain::IPV6 {
            socket2_socket.set_only_v6(true)?
        }
        socket2_socket.set_nonblocking(true)?; // Needed by Tokio
        // See https://docs.rs/tokio/latest/tokio/net/struct.TcpSocket.html
        #[cfg(not(windows))]
        socket2_socket.set_reuse_address(true)?;
        let socket2_addr = addr.into();
        socket2_socket.bind(&socket2_addr)?;
        socket2_socket.listen(1024)?;
        let std_listener: std::net::TcpListener = socket2_socket.into();
        let listener = net::TcpListener::from_std(std_listener)?;
        trace!(?addr, "Tcp Listener bound");
        let mut end_receiver = s2s_stop_listening_r.fuse();
        tokio::spawn(async move {
            while let Some(data) = select! {
                    next = listener.accept().fuse() => Some(next),
                    _ = &mut end_receiver => None,
            } {
                let (stream, remote_addr) = match data {
                    Ok((s, p)) => (s, p),
                    Err(e) => {
                        trace!(?e, "TcpStream Error, ignoring connection attempt");
                        continue;
                    },
                };
                if let Err(e) = stream.set_nodelay(true) {
                    warn!(
                        ?e,
                        "Failed to set TCP_NODELAY, client may have degraded latency"
                    );
                }
                let cid = cids.fetch_add(1, Ordering::Relaxed);
                info!(?remote_addr, ?cid, "Accepting Tcp from");
                let metrics = ProtocolMetricCache::new(&cid.to_string(), Arc::clone(&metrics));
                let _ = c2s_protocol_s.send((
                    Self::new_tcp(stream, metrics.clone()),
                    ConnectAddr::Tcp(remote_addr),
                    cid,
                ));
            }
        });
        Ok(())
    }

    pub(crate) fn new_tcp(stream: net::TcpStream, metrics: ProtocolMetricCache) -> Self {
        let (r, w) = stream.into_split();
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

    pub(crate) async fn with_mpsc_connect(
        addr: u64,
        metrics: ProtocolMetricCache,
    ) -> Result<Self, NetworkConnectError> {
        let mpsc_s = MPSC_POOL
            .lock()
            .await
            .get(&addr)
            .ok_or_else(|| {
                NetworkConnectError::Io(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "no mpsc listen on this addr",
                ))
            })?
            .clone();
        let (remote_to_local_s, remote_to_local_r) = mpsc::channel(Self::MPSC_CHANNEL_BOUND);
        let (local_to_remote_oneshot_s, local_to_remote_oneshot_r) = oneshot::channel();
        mpsc_s
            .send((remote_to_local_s, local_to_remote_oneshot_s))
            .map_err(|_| {
                NetworkConnectError::Io(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "mpsc pipe broke during connect",
                ))
            })?;
        let local_to_remote_s = local_to_remote_oneshot_r
            .await
            .map_err(|e| NetworkConnectError::Io(io::Error::new(io::ErrorKind::BrokenPipe, e)))?;
        info!(?addr, "Connecting Mpsc");
        Ok(Self::new_mpsc(
            local_to_remote_s,
            remote_to_local_r,
            metrics,
        ))
    }

    pub(crate) async fn with_mpsc_listen(
        addr: u64,
        cids: Arc<AtomicU64>,
        metrics: Arc<ProtocolMetrics>,
        s2s_stop_listening_r: oneshot::Receiver<()>,
        c2s_protocol_s: mpsc::UnboundedSender<C2sProtocol>,
    ) -> io::Result<()> {
        let (mpsc_s, mut mpsc_r) = mpsc::unbounded_channel();
        MPSC_POOL.lock().await.insert(addr, mpsc_s);
        trace!(?addr, "Mpsc Listener bound");
        let mut end_receiver = s2s_stop_listening_r.fuse();
        tokio::spawn(async move {
            while let Some((local_to_remote_s, local_remote_to_local_s)) = select! {
                    next = mpsc_r.recv().fuse() => next,
                    _ = &mut end_receiver => None,
            } {
                let (remote_to_local_s, remote_to_local_r) =
                    mpsc::channel(Self::MPSC_CHANNEL_BOUND);
                if let Err(e) = local_remote_to_local_s.send(remote_to_local_s) {
                    error!(?e, "mpsc listen aborted");
                }

                let cid = cids.fetch_add(1, Ordering::Relaxed);
                info!(?addr, ?cid, "Accepting Mpsc from");
                let metrics = ProtocolMetricCache::new(&cid.to_string(), Arc::clone(&metrics));
                let _ = c2s_protocol_s.send((
                    Self::new_mpsc(local_to_remote_s, remote_to_local_r, metrics.clone()),
                    ConnectAddr::Mpsc(addr),
                    cid,
                ));
            }
            warn!("MpscStream Failed, stopping");
        });
        Ok(())
    }

    pub(crate) fn new_mpsc(
        sender: mpsc::Sender<MpscMsg>,
        receiver: mpsc::Receiver<MpscMsg>,
        metrics: ProtocolMetricCache,
    ) -> Self {
        let sp = MpscSendProtocol::new(MpscDrain { sender }, metrics.clone());
        let rp = MpscRecvProtocol::new(MpscSink { receiver }, metrics);
        Protocols::Mpsc((sp, rp))
    }

    #[cfg(feature = "quic")]
    pub(crate) async fn with_quic_connect(
        addr: SocketAddr,
        config: quinn::ClientConfig,
        name: String,
        metrics: ProtocolMetricCache,
    ) -> Result<Self, NetworkConnectError> {
        let config = config.clone();

        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

        let bindsock = match addr {
            SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
        };
        let endpoint = match quinn::Endpoint::client(bindsock) {
            Ok(e) => e,
            Err(e) => return Err(NetworkConnectError::Io(e)),
        };

        info!("Connecting Quic to: {}", &addr);
        let connecting = endpoint.connect_with(config, addr, &name).map_err(|e| {
            trace!(?e, "error setting up quic");
            NetworkConnectError::Io(io::Error::new(io::ErrorKind::ConnectionAborted, e))
        })?;
        let connection = connecting.await.map_err(|e| {
            trace!(?e, "error with quic connection");
            NetworkConnectError::Io(io::Error::new(io::ErrorKind::ConnectionAborted, e))
        })?;
        Self::new_quic(connection, false, metrics)
            .await
            .map_err(|e| {
                trace!(?e, "error with quic");
                NetworkConnectError::Io(io::Error::new(io::ErrorKind::ConnectionAborted, e))
            })
    }

    #[cfg(feature = "quic")]
    pub(crate) async fn with_quic_listen(
        addr: SocketAddr,
        server_config: quinn::ServerConfig,
        cids: Arc<AtomicU64>,
        metrics: Arc<ProtocolMetrics>,
        s2s_stop_listening_r: oneshot::Receiver<()>,
        c2s_protocol_s: mpsc::UnboundedSender<C2sProtocol>,
    ) -> io::Result<()> {
        let (_endpoint, mut listener) = match quinn::Endpoint::server(server_config, addr) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        trace!(?addr, "Quic Listener bound");
        let mut end_receiver = s2s_stop_listening_r.fuse();
        tokio::spawn(async move {
            while let Some(Some(connecting)) = select! {
                next = listener.next().fuse() => Some(next),
                _ = &mut end_receiver => None,
            } {
                let remote_addr = connecting.remote_address();
                let connection = match connecting.await {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::debug!(?e, ?remote_addr, "skipping connection attempt");
                        continue;
                    },
                };

                let cid = cids.fetch_add(1, Ordering::Relaxed);
                info!(?remote_addr, ?cid, "Accepting Quic from");
                let metrics = ProtocolMetricCache::new(&cid.to_string(), Arc::clone(&metrics));
                match Protocols::new_quic(connection, true, metrics).await {
                    Ok(quic) => {
                        // TODO: we cannot guess the client hostname in quic server here.
                        // though we need it for the certificate to be validated, in the future
                        // this will either go away with new auth, or we have to do something like
                        // a reverse DNS lookup
                        let connect_addr = ConnectAddr::Quic(
                            addr,
                            quinn::ClientConfig::with_native_roots(),
                            "TODO_remote_hostname".to_string(),
                        );
                        let _ = c2s_protocol_s.send((quic, connect_addr, cid));
                    },
                    Err(e) => {
                        trace!(?e, "failed to start quic");
                        continue;
                    },
                }
            }
        });
        Ok(())
    }

    #[cfg(feature = "quic")]
    pub(crate) async fn new_quic(
        mut connection: quinn::NewConnection,
        listen: bool,
        metrics: ProtocolMetricCache,
    ) -> Result<Self, quinn::ConnectionError> {
        let (sendstream, recvstream) = if listen {
            connection.connection.open_bi().await?
        } else {
            connection
                .bi_streams
                .next()
                .await
                .ok_or(quinn::ConnectionError::LocallyClosed)??
        };
        let (recvstreams_s, recvstreams_r) = mpsc::unbounded_channel();
        let streams_s_clone = recvstreams_s.clone();
        let (sendstreams_s, sendstreams_r) = mpsc::unbounded_channel();
        let sp = QuicSendProtocol::new(
            QuicDrain {
                con: connection.connection.clone(),
                main: sendstream,
                reliables: HashMap::new(),
                recvstreams_s: streams_s_clone,
                sendstreams_r,
            },
            metrics.clone(),
        );
        spawn_new(recvstream, None, &recvstreams_s);
        let rp = QuicRecvProtocol::new(
            QuicSink {
                con: connection.connection,
                bi: connection.bi_streams,
                recvstreams_r,
                recvstreams_s,
                sendstreams_s,
            },
            metrics,
        );
        Ok(Protocols::Quic((sp, rp)))
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
    type CustomErr = ProtocolsError;

    async fn initialize(
        &mut self,
        initializer: bool,
        local_pid: Pid,
        secret: u128,
    ) -> Result<(Pid, Sid, u128), InitProtocolError<Self::CustomErr>> {
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
    type CustomErr = ProtocolsError;

    fn notify_from_recv(&mut self, event: ProtocolEvent) {
        match self {
            SendProtocols::Tcp(s) => s.notify_from_recv(event),
            SendProtocols::Mpsc(s) => s.notify_from_recv(event),
            #[cfg(feature = "quic")]
            SendProtocols::Quic(s) => s.notify_from_recv(event),
        }
    }

    async fn send(&mut self, event: ProtocolEvent) -> Result<(), ProtocolError<Self::CustomErr>> {
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
    ) -> Result<Bandwidth, ProtocolError<Self::CustomErr>> {
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
    type CustomErr = ProtocolsError;

    async fn recv(&mut self) -> Result<ProtocolEvent, ProtocolError<Self::CustomErr>> {
        match self {
            RecvProtocols::Tcp(r) => r.recv().await,
            RecvProtocols::Mpsc(r) => r.recv().await,
            #[cfg(feature = "quic")]
            RecvProtocols::Quic(r) => r.recv().await,
        }
    }
}

#[derive(Debug)]
pub enum MpscError {
    Send(mpsc::error::SendError<MpscMsg>),
    Recv,
}

#[cfg(feature = "quic")]
#[derive(Debug)]
pub enum QuicError {
    Send(io::Error),
    Connection(quinn::ConnectionError),
    Write(quinn::WriteError),
    Read(quinn::ReadError),
    InternalMpsc,
}

/// Error types for Protocols
#[derive(Debug)]
pub enum ProtocolsError {
    Tcp(io::Error),
    Udp(io::Error),
    #[cfg(feature = "quic")]
    Quic(QuicError),
    Mpsc(MpscError),
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
    type CustomErr = ProtocolsError;
    type DataFormat = BytesMut;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError<Self::CustomErr>> {
        self.half
            .write_all(&data)
            .await
            .map_err(|e| ProtocolError::Custom(ProtocolsError::Tcp(e)))
    }
}

#[async_trait]
impl UnreliableSink for TcpSink {
    type CustomErr = ProtocolsError;
    type DataFormat = BytesMut;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError<Self::CustomErr>> {
        if self.buffer.capacity() < 1500 {
            self.buffer.reserve(1500 * 4); // reserve multiple, so that we alloc less often
        }
        match self.half.read_buf(&mut self.buffer).await {
            Ok(0) => Err(ProtocolError::Custom(ProtocolsError::Tcp(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "read returned 0 bytes",
            )))),
            Ok(_) => Ok(self.buffer.split()),
            Err(e) => Err(ProtocolError::Custom(ProtocolsError::Tcp(e))),
        }
    }
}

///////////////////////////////////////
//// MPSC
#[derive(Debug)]
pub struct MpscDrain {
    sender: mpsc::Sender<MpscMsg>,
}

#[derive(Debug)]
pub struct MpscSink {
    receiver: mpsc::Receiver<MpscMsg>,
}

#[async_trait]
impl UnreliableDrain for MpscDrain {
    type CustomErr = ProtocolsError;
    type DataFormat = MpscMsg;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError<Self::CustomErr>> {
        self.sender
            .send(data)
            .await
            .map_err(|e| ProtocolError::Custom(ProtocolsError::Mpsc(MpscError::Send(e))))
    }
}

#[async_trait]
impl UnreliableSink for MpscSink {
    type CustomErr = ProtocolsError;
    type DataFormat = MpscMsg;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError<Self::CustomErr>> {
        self.receiver
            .recv()
            .await
            .ok_or(ProtocolError::Custom(ProtocolsError::Mpsc(MpscError::Recv)))
    }
}

///////////////////////////////////////
//// QUIC
#[cfg(feature = "quic")]
type QuicStream = (
    BytesMut,
    Result<Option<usize>, quinn::ReadError>,
    quinn::RecvStream,
    Option<Sid>,
);

#[cfg(feature = "quic")]
#[derive(Debug)]
pub struct QuicDrain {
    con: quinn::Connection,
    main: quinn::SendStream,
    reliables: HashMap<Sid, quinn::SendStream>,
    recvstreams_s: mpsc::UnboundedSender<QuicStream>,
    sendstreams_r: mpsc::UnboundedReceiver<quinn::SendStream>,
}

#[cfg(feature = "quic")]
#[derive(Debug)]
pub struct QuicSink {
    #[allow(dead_code)]
    con: quinn::Connection,
    bi: quinn::IncomingBiStreams,
    recvstreams_r: mpsc::UnboundedReceiver<QuicStream>,
    recvstreams_s: mpsc::UnboundedSender<QuicStream>,
    sendstreams_s: mpsc::UnboundedSender<quinn::SendStream>,
}

#[cfg(feature = "quic")]
fn spawn_new(
    mut recvstream: quinn::RecvStream,
    sid: Option<Sid>,
    streams_s: &mpsc::UnboundedSender<QuicStream>,
) {
    let streams_s_clone = streams_s.clone();
    tokio::spawn(async move {
        let mut buffer = BytesMut::new();
        buffer.resize(1500, 0u8);
        let r = recvstream.read(&mut buffer).await;
        let _ = streams_s_clone.send((buffer, r, recvstream, sid));
    });
}

#[cfg(feature = "quic")]
#[async_trait]
impl UnreliableDrain for QuicDrain {
    type CustomErr = ProtocolsError;
    type DataFormat = QuicDataFormat;

    async fn send(&mut self, data: Self::DataFormat) -> Result<(), ProtocolError<Self::CustomErr>> {
        match data.stream {
            QuicDataFormatStream::Main => self.main.write_all(&data.data).await,
            QuicDataFormatStream::Unreliable => unimplemented!(),
            QuicDataFormatStream::Reliable(sid) => {
                use hashbrown::hash_map::Entry;
                //tracing::trace!(?sid, "Reliable");
                match self.reliables.entry(sid) {
                    Entry::Occupied(mut occupied) => occupied.get_mut().write_all(&data.data).await,
                    Entry::Vacant(vacant) => {
                        // IF the buffer is empty this was created locally and WE are allowed to
                        // open_bi(), if not, we NEED to block on sendstreams_r
                        if data.data.is_empty() {
                            let (mut sendstream, recvstream) =
                                self.con.open_bi().await.map_err(|e| {
                                    ProtocolError::Custom(ProtocolsError::Quic(
                                        QuicError::Connection(e),
                                    ))
                                })?;
                            // send SID as first msg
                            sendstream.write_u64(sid.get_u64()).await.map_err(|e| {
                                ProtocolError::Custom(ProtocolsError::Quic(QuicError::Send(e)))
                            })?;
                            spawn_new(recvstream, Some(sid), &self.recvstreams_s);
                            vacant.insert(sendstream).write_all(&data.data).await
                        } else {
                            let sendstream =
                                self.sendstreams_r
                                    .recv()
                                    .await
                                    .ok_or(ProtocolError::Custom(ProtocolsError::Quic(
                                        QuicError::InternalMpsc,
                                    )))?;
                            vacant.insert(sendstream).write_all(&data.data).await
                        }
                    },
                }
            },
        }
        .map_err(|e| ProtocolError::Custom(ProtocolsError::Quic(QuicError::Write(e))))
    }
}

#[cfg(feature = "quic")]
#[async_trait]
impl UnreliableSink for QuicSink {
    type CustomErr = ProtocolsError;
    type DataFormat = QuicDataFormat;

    async fn recv(&mut self) -> Result<Self::DataFormat, ProtocolError<Self::CustomErr>> {
        let (mut buffer, result, mut recvstream, id) = loop {
            use futures_util::FutureExt;
            // first handle all bi streams!
            let (a, b) = select! {
                biased;
                Some(n) = self.bi.next().fuse() => (Some(n), None),
                Some(n) = self.recvstreams_r.recv().fuse() => (None, Some(n)),
            };

            if let Some(remote_stream) = a {
                let (sendstream, mut recvstream) = remote_stream.map_err(|e| {
                    ProtocolError::Custom(ProtocolsError::Quic(QuicError::Connection(e)))
                })?;
                let sid = match recvstream.read_u64().await {
                    Ok(u64::MAX) => None, //unreliable
                    Ok(sid) => Some(Sid::new(sid)),
                    Err(_) => return Err(ProtocolError::Violated),
                };
                if self.sendstreams_s.send(sendstream).is_err() {
                    return Err(ProtocolError::Custom(ProtocolsError::Quic(
                        QuicError::InternalMpsc,
                    )));
                }
                spawn_new(recvstream, sid, &self.recvstreams_s);
            }

            if let Some(data) = b {
                break data;
            }
        };

        let r = match result {
            Ok(Some(0)) => Err(ProtocolError::Custom(ProtocolsError::Quic(
                QuicError::Send(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "read returned 0 bytes",
                )),
            ))),
            Ok(Some(n)) => Ok(QuicDataFormat {
                stream: match id {
                    Some(id) => QuicDataFormatStream::Reliable(id),
                    None => QuicDataFormatStream::Main,
                },
                data: buffer.split_to(n),
            }),
            Ok(None) => Err(ProtocolError::Custom(ProtocolsError::Quic(
                QuicError::Send(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "read returned None",
                )),
            ))),
            Err(e) => Err(ProtocolError::Custom(ProtocolsError::Quic(
                QuicError::Read(e),
            ))),
        }?;

        let streams_s_clone = self.recvstreams_s.clone();
        tokio::spawn(async move {
            buffer.resize(1500, 0u8);
            let r = recvstream.read(&mut buffer).await;
            let _ = streams_s_clone.send((buffer, r, recvstream, id));
        });
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use network_protocol::{Promises, ProtocolMetrics, RecvProtocol, SendProtocol};
    use std::sync::Arc;
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
        let metrics = ProtocolMetricCache::new("0", Arc::new(ProtocolMetrics::new().unwrap()));
        let client = Protocols::new_tcp(client, metrics.clone());
        let server = Protocols::new_tcp(server, metrics);
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
        let metrics = ProtocolMetricCache::new("0", Arc::new(ProtocolMetrics::new().unwrap()));
        let client = Protocols::new_tcp(client, metrics.clone());
        let server = Protocols::new_tcp(server, metrics);
        let (s, _) = client.split();
        let (_, mut r) = server.split();
        let e = tokio::spawn(async move { r.recv().await });
        drop(s);
        let e = e.await.unwrap();
        assert!(e.is_err());
        assert!(matches!(e, Err(..)));
        let e = e.unwrap_err();
        assert!(matches!(e, ProtocolError::Custom(..)));
        assert!(matches!(e, ProtocolError::Custom(ProtocolsError::Tcp(_))));
        match e {
            ProtocolError::Custom(ProtocolsError::Tcp(e)) => {
                assert_eq!(e.kind(), io::ErrorKind::BrokenPipe)
            },
            _ => panic!("invalid error"),
        }
    }
}
