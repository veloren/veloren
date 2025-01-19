use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
    time::{Duration, Instant},
};

use protocol::Parcel;
use tokio::{net::UdpSocket, time::timeout};
use tracing::trace;

use crate::proto::{
    MAX_RESPONSE_SIZE, QueryServerRequest, QueryServerResponse, RawQueryServerRequest,
    RawQueryServerResponse, ServerInfo,
};

// This must be at least 2 for the client to get a value for the `p` field.
const MAX_REQUEST_RETRIES: usize = 5;

#[derive(Debug)]
pub enum QueryClientError {
    Io(tokio::io::Error),
    Protocol(protocol::Error),
    InvalidResponse,
    Timeout,
    ChallengeFailed,
}

struct ClientInitData {
    p: u64,
    #[expect(dead_code)]
    server_max_version: u16,
}

/// The `p` field has to be requested from the server each time this client is
/// constructed, if possible reuse this!
pub struct QueryClient {
    pub addr: SocketAddr,
    init: Option<ClientInitData>,
}

impl QueryClient {
    pub fn new(addr: SocketAddr) -> Self { Self { addr, init: None } }

    pub async fn server_info(&mut self) -> Result<(ServerInfo, Duration), QueryClientError> {
        self.send_query(QueryServerRequest::ServerInfo)
            .await
            .and_then(|(response, duration)| {
                #[expect(irrefutable_let_patterns)]
                if let QueryServerResponse::ServerInfo(info) = response {
                    Ok((info, duration))
                } else {
                    Err(QueryClientError::InvalidResponse)
                }
            })
    }

    async fn send_query(
        &mut self,
        request: QueryServerRequest,
    ) -> Result<(QueryServerResponse, Duration), QueryClientError> {
        let socket = UdpSocket::bind(if self.addr.is_ipv4() {
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        } else {
            SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, 0, 0, 0))
        })
        .await?;

        for _ in 0..MAX_REQUEST_RETRIES {
            let request = if let Some(init) = &self.init {
                // TODO: Use the maximum version supported by both the client and server once
                // new protocol versions are added
                RawQueryServerRequest { p: init.p, request }
            } else {
                // TODO: Use the legacy version here once new protocol versions are added
                RawQueryServerRequest {
                    p: 0,
                    request: QueryServerRequest::Init,
                }
            };
            let buf = request.serialize()?;
            let query_sent = Instant::now();
            socket.send_to(&buf, self.addr).await?;
            let mut buf = vec![0; MAX_RESPONSE_SIZE];
            let (buf_len, _) = timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
                .await
                .map_err(|_| QueryClientError::Timeout)??;

            if buf_len <= 2 {
                Err(QueryClientError::InvalidResponse)?
            }

            let packet = <RawQueryServerResponse as Parcel>::read(
                &mut io::Cursor::new(&buf[..buf_len]),
                &Default::default(),
            )?;

            match packet {
                RawQueryServerResponse::Response(response) => {
                    return Ok((response, query_sent.elapsed()));
                },
                RawQueryServerResponse::Init(init) => {
                    trace!(?init, "Resetting p");
                    self.init = Some(ClientInitData {
                        p: init.p,
                        server_max_version: init.max_supported_version,
                    });
                },
            }
        }

        Err(QueryClientError::ChallengeFailed)
    }
}

impl From<tokio::io::Error> for QueryClientError {
    fn from(value: tokio::io::Error) -> Self { Self::Io(value) }
}

impl From<protocol::Error> for QueryClientError {
    fn from(value: protocol::Error) -> Self { Self::Protocol(value) }
}
