use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, Instant},
};

use protocol::Parcel;
use tokio::{net::UdpSocket, time::timeout};

use crate::proto::{
    QueryServerRequest, QueryServerResponse, ServerInfo, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE,
    VELOREN_HEADER,
};

#[derive(Debug)]
pub enum QueryClientError {
    Io(tokio::io::Error),
    Protocol(protocol::Error),
    InvalidResponse,
    Timeout,
}

pub struct QueryClient {
    pub addr: SocketAddr,
}

impl QueryClient {
    pub async fn server_info(&self) -> Result<(ServerInfo, Duration), QueryClientError> {
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
        &self,
        request: QueryServerRequest,
    ) -> Result<(QueryServerResponse, Duration), QueryClientError> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;

        let mut buf = Vec::with_capacity(VELOREN_HEADER.len() + 2 + MAX_REQUEST_SIZE);
        buf.extend(VELOREN_HEADER);
        // 2 extra bytes for version information, currently unused
        buf.extend([0; 2]);
        buf.extend(<QueryServerRequest as Parcel>::raw_bytes(
            &request,
            &Default::default(),
        )?);

        let query_sent = Instant::now();
        socket.send_to(&buf, self.addr).await?;

        let mut buf = vec![0; MAX_RESPONSE_SIZE];
        let _ = timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
            .await
            .map_err(|_| QueryClientError::Timeout)?
            .map_err(|_| QueryClientError::Timeout)?;

        let packet =
            <QueryServerResponse as Parcel>::read(&mut io::Cursor::new(buf), &Default::default())?;

        Ok((packet, query_sent.elapsed()))
    }
}

impl From<tokio::io::Error> for QueryClientError {
    fn from(value: tokio::io::Error) -> Self { Self::Io(value) }
}

impl From<protocol::Error> for QueryClientError {
    fn from(value: protocol::Error) -> Self { Self::Protocol(value) }
}
