use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, Instant},
};

use protocol::Parcel;
use tokio::{net::UdpSocket, time::timeout};
use tracing::trace;

use crate::proto::{
    QueryServerRequest, QueryServerResponse, RawQueryServerRequest, RawQueryServerResponse,
    ServerInfo, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, VELOREN_HEADER,
};

const MAX_REQUEST_RETRIES: usize = 5;

#[derive(Debug)]
pub enum QueryClientError {
    Io(tokio::io::Error),
    Protocol(protocol::Error),
    InvalidResponse,
    Timeout,
    ChallengeFailed,
}

pub struct QueryClient {
    pub addr: SocketAddr,
    p: u64,
}

impl QueryClient {
    pub fn new(addr: SocketAddr) -> Self { Self { addr, p: 0 } }

    pub async fn server_info(&mut self) -> Result<(ServerInfo, Duration), QueryClientError> {
        self.send_query(QueryServerRequest::ServerInfo(Default::default()))
            .await
            .and_then(|(response, duration)| {
                #[allow(irrefutable_let_patterns)] // TODO: remove when more variants are added
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
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;

        let mut tries = 0;
        while tries < MAX_REQUEST_RETRIES {
            tries += 1;
            let mut buf = Vec::with_capacity(VELOREN_HEADER.len() + 2 + MAX_REQUEST_SIZE);

            buf.extend(VELOREN_HEADER);
            // 2 extra bytes for version information, currently unused
            buf.extend([0; 2]);
            buf.extend(<RawQueryServerRequest as Parcel>::raw_bytes(
                &RawQueryServerRequest { p: self.p, request },
                &Default::default(),
            )?);

            let query_sent = Instant::now();
            socket.send_to(&buf, self.addr).await?;

            let mut buf = vec![0; MAX_RESPONSE_SIZE];
            let _ = timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
                .await
                .map_err(|_| QueryClientError::Timeout)?
                .map_err(|_| QueryClientError::Timeout)?;

            let packet = <RawQueryServerResponse as Parcel>::read(
                &mut io::Cursor::new(buf),
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
