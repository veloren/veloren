use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::{Duration, Instant},
};

use tokio::{net::UdpSocket, time::timeout};

use crate::proto::{Ping, QueryServerRequest, QueryServerResponse, ServerInfo, VELOREN_HEADER};

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
    pub async fn server_info(&self) -> Result<ServerInfo, QueryClientError> {
        self.send_query(QueryServerRequest::ServerInfo(Default::default()))
            .await
            .and_then(|(response, _)| {
                if let QueryServerResponse::ServerInfo(info) = response {
                    Ok(info)
                } else {
                    Err(QueryClientError::InvalidResponse)
                }
            })
    }

    pub async fn ping(&self) -> Result<Duration, QueryClientError> {
        self.send_query(QueryServerRequest::Ping(Ping))
            .await
            .map(|(_, elapsed)| elapsed)
    }

    async fn send_query(
        &self,
        request: QueryServerRequest,
    ) -> Result<(QueryServerResponse, Duration), QueryClientError> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0)).await?;

        let mut pipeline = crate::create_pipeline();

        let mut buf = VELOREN_HEADER.to_vec();

        let mut cursor = io::Cursor::new(&mut buf);
        cursor.set_position(VELOREN_HEADER.len() as u64);
        pipeline.send_to(&mut cursor, &request)?;

        let query_sent = Instant::now();
        socket.send_to(buf.as_slice(), self.addr).await?;

        let mut buf = vec![0; 1500];
        let _ = timeout(Duration::from_secs(2), socket.recv_from(&mut buf))
            .await
            .map_err(|_| QueryClientError::Timeout)?
            .map_err(|_| QueryClientError::Timeout)?;

        let mut pipeline = crate::create_pipeline();

        let packet: QueryServerResponse = pipeline.receive_from(&mut io::Cursor::new(&mut buf))?;

        Ok((packet, query_sent.elapsed()))
    }
}

impl From<tokio::io::Error> for QueryClientError {
    fn from(value: tokio::io::Error) -> Self { Self::Io(value) }
}

impl From<protocol::Error> for QueryClientError {
    fn from(value: protocol::Error) -> Self { Self::Protocol(value) }
}
