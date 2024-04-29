use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, Instant},
};

use protocol::Parcel;
use tokio::{net::UdpSocket, time::timeout};
use tracing::{trace, warn};

use crate::proto::{
    QueryServerRequest, QueryServerResponse, RawQueryServerRequest, RawQueryServerResponse,
    ServerInfo, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, VELOREN_HEADER, VERSION,
};

// This must be at least 2 for the client to get a value for the `p` field.
const MAX_REQUEST_RETRIES: usize = 5;

#[derive(Debug)]
pub enum QueryClientError {
    Io(tokio::io::Error),
    Protocol(protocol::Error),
    InvalidResponse,
    InvalidVersion,
    Timeout,
    ChallengeFailed,
    RequestTooLarge,
}

/// The `p` field has to be requested from the server each time this client is
/// constructed, if possible reuse this!
pub struct QueryClient {
    pub addr: SocketAddr,
    p: u64,
}

impl QueryClient {
    pub fn new(addr: SocketAddr) -> Self { Self { addr, p: 0 } }

    pub async fn server_info(&mut self) -> Result<(ServerInfo, Duration), QueryClientError> {
        self.send_query(QueryServerRequest::ServerInfo)
            .await
            .and_then(|(response, duration)| {
                if let QueryServerResponse::ServerInfo(info) = response {
                    Ok((info, duration))
                } else {
                    Err(QueryClientError::InvalidResponse)
                }
            })
    }

    pub async fn ping(&mut self) -> Result<Duration, QueryClientError> {
        self.send_query(QueryServerRequest::Ping)
            .await
            .and_then(|(response, duration)| {
                if let QueryServerResponse::Pong = response {
                    Ok(duration)
                } else {
                    Err(QueryClientError::InvalidResponse)
                }
            })
    }

    async fn send_query(
        &mut self,
        request: QueryServerRequest,
    ) -> Result<(QueryServerResponse, Duration), QueryClientError> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;

        for _ in 0..MAX_REQUEST_RETRIES {
            let mut buf = Vec::with_capacity(VELOREN_HEADER.len() + 2 + MAX_REQUEST_SIZE);

            // 2 extra bytes for version information, currently unused
            buf.extend(VERSION.to_le_bytes());
            buf.extend({
                let request_data = <RawQueryServerRequest as Parcel>::raw_bytes(
                    &RawQueryServerRequest { p: self.p, request },
                    &Default::default(),
                )?;
                if request_data.len() > MAX_REQUEST_SIZE {
                    warn!(
                        ?request,
                        ?MAX_REQUEST_SIZE,
                        "Attempted to send request larger than the max size ({})",
                        request_data.len()
                    );
                    Err(QueryClientError::RequestTooLarge)?
                }
                request_data
            });
            buf.resize(2 + MAX_RESPONSE_SIZE, 0);
            buf.extend(VELOREN_HEADER);

            let query_sent = Instant::now();
            socket.send_to(&buf, self.addr).await?;

            let mut buf = vec![0; MAX_RESPONSE_SIZE];
            let (buf_len, _) = timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
                .await
                .map_err(|_| QueryClientError::Timeout)??;

            if buf_len <= 2 {
                Err(QueryClientError::InvalidResponse)?
            }

            // FIXME: Allow lower versions once proper versioning is added.
            if u16::from_le_bytes(buf[..2].try_into().unwrap()) != VERSION {
                Err(QueryClientError::InvalidVersion)?
            }

            let packet = <RawQueryServerResponse as Parcel>::read(
                // TODO: Remove this padding once version information is added to packets
                &mut io::Cursor::new(&buf[2..buf_len]),
                &Default::default(),
            )?;

            match packet {
                RawQueryServerResponse::Response(response) => {
                    return Ok((response, query_sent.elapsed()));
                },
                RawQueryServerResponse::P(p) => {
                    trace!(?p, "Resetting p");
                    self.p = p
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
