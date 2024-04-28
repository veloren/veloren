#[allow(deprecated)] use std::hash::SipHasher;
use std::{
    future::Future,
    hash::{Hash, Hasher},
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::Arc,
    time::{Duration, Instant},
};

use protocol::Parcel;
use rand::{thread_rng, Rng};
use tokio::{
    net::UdpSocket,
    sync::{watch, RwLock},
    time::timeout,
};
use tracing::{debug, trace};

use crate::proto::{
    QueryServerRequest, QueryServerResponse, RawQueryServerRequest, RawQueryServerResponse,
    ServerInfo, MAX_REQUEST_SIZE, VELOREN_HEADER,
};

const RESPONSE_SEND_TIMEOUT: Duration = Duration::from_secs(2);
const SECRET_REGEN_INTERNVAL: Duration = Duration::from_secs(60);

pub struct QueryServer {
    pub addr: SocketAddr,
    server_info: watch::Receiver<ServerInfo>,
    settings: protocol::Settings,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Metrics {
    pub received_packets: u32,
    pub dropped_packets: u32,
    pub invalid_packets: u32,
    pub proccessing_errors: u32,
    pub info_requests: u32,
    pub sent_responses: u32,
    pub failed_responses: u32,
    pub timed_out_responses: u32,
}

impl QueryServer {
    pub fn new(addr: SocketAddr, server_info: watch::Receiver<ServerInfo>) -> Self {
        Self {
            addr,
            server_info,
            settings: Default::default(),
        }
    }

    pub async fn run(&mut self, metrics: Arc<RwLock<Metrics>>) -> Result<(), tokio::io::Error> {
        let socket = UdpSocket::bind(self.addr).await?;

        let gen_secret = || {
            let mut rng = thread_rng();
            (rng.gen::<u64>(), rng.gen::<u64>())
        };
        let mut secrets = gen_secret();
        let mut last_secret_refresh = Instant::now();

        let mut buf = Box::new([0; MAX_REQUEST_SIZE]);
        loop {
            *buf = [0; MAX_REQUEST_SIZE];

            let Ok((len, remote_addr)) = socket.recv_from(&mut *buf).await.inspect_err(|err| {
                debug!("Error while receiving from query server socket: {err:?}")
            }) else {
                continue;
            };

            let mut new_metrics = Metrics {
                received_packets: 1,
                ..Default::default()
            };

            let raw_msg_buf = &buf[..len];
            let msg_buf = if Self::validate_datagram(raw_msg_buf) {
                // Require 2 extra bytes for version (currently unused)
                &raw_msg_buf[(VELOREN_HEADER.len() + 2)..]
            } else {
                new_metrics.dropped_packets += 1;
                continue;
            };

            if let Err(error) = self
                .process_datagram(
                    msg_buf,
                    remote_addr,
                    secrets,
                    (&mut new_metrics, Arc::clone(&metrics)),
                )
                .await
            {
                debug!(?error, "Error while processing datagram");
            }

            // Update metrics at the end of eath packet
            *metrics.write().await += new_metrics;

            {
                let now = Instant::now();
                if now.duration_since(last_secret_refresh) > SECRET_REGEN_INTERNVAL {
                    last_secret_refresh = now;
                    secrets = gen_secret();
                }
            }
        }
    }

    // Header must be discarded after this validation passes
    fn validate_datagram(data: &[u8]) -> bool {
        let len = data.len();
        // Require 2 extra bytes for version (currently unused)
        if len < VELOREN_HEADER.len() + 3 {
            trace!(?len, "Datagram too short");
            false
        } else if data[0..VELOREN_HEADER.len()] != VELOREN_HEADER {
            trace!(?len, "Datagram header invalid");
            false
        } else {
            true
        }
    }

    async fn process_datagram(
        &mut self,
        datagram: &[u8],
        remote: SocketAddr,
        secrets: (u64, u64),
        (new_metrics, metrics): (&mut Metrics, Arc<RwLock<Metrics>>),
    ) -> Result<(), tokio::io::Error> {
        let Ok(RawQueryServerRequest {
            p: client_p,
            request,
        }) =
            <RawQueryServerRequest as Parcel>::read(&mut io::Cursor::new(datagram), &self.settings)
        else {
            new_metrics.invalid_packets += 1;
            return Ok(());
        };

        trace!(?request, "Received packet");

        #[allow(deprecated)]
        let real_p = {
            let mut hasher = SipHasher::new_with_keys(secrets.0, secrets.1);
            remote.ip().hash(&mut hasher);
            hasher.finish()
        };

        async fn timed<'a, F: Future<Output = O> + 'a, O>(
            fut: F,
            metrics: &'a Arc<RwLock<Metrics>>,
        ) -> Option<O> {
            if let Ok(res) = timeout(RESPONSE_SEND_TIMEOUT, fut).await {
                Some(res)
            } else {
                metrics.write().await.timed_out_responses += 1;
                None
            }
        }

        if real_p != client_p {
            tokio::task::spawn(async move {
                timed(
                    Self::send_response(RawQueryServerResponse::P(real_p), remote, &metrics),
                    &metrics,
                )
                .await;
            });

            return Ok(());
        }

        match request {
            QueryServerRequest::ServerInfo(_) => {
                new_metrics.info_requests += 1;
                let server_info = *self.server_info.borrow();
                tokio::task::spawn(async move {
                    timed(
                        Self::send_response(
                            RawQueryServerResponse::Response(QueryServerResponse::ServerInfo(
                                server_info,
                            )),
                            remote,
                            &metrics,
                        ),
                        &metrics,
                    )
                    .await;
                });
            },
        }

        Ok(())
    }

    async fn send_response(
        response: RawQueryServerResponse,
        addr: SocketAddr,
        metrics: &Arc<RwLock<Metrics>>,
    ) {
        let Ok(socket) =
            UdpSocket::bind(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))).await
        else {
            debug!("Failed to create response socket");
            return;
        };

        let buf = if let Ok(data) =
            <RawQueryServerResponse as Parcel>::raw_bytes(&response, &Default::default())
        {
            data
        } else {
            Vec::new()
        };
        match socket.send_to(&buf, addr).await {
            Ok(_) => {
                metrics.write().await.sent_responses += 1;
            },
            Err(err) => {
                metrics.write().await.failed_responses += 1;
                debug!(?err, "Failed to send query server response");
            },
        }
    }
}

impl std::ops::AddAssign for Metrics {
    fn add_assign(
        &mut self,
        Self {
            received_packets,
            dropped_packets,
            invalid_packets,
            proccessing_errors,
            info_requests,
            sent_responses,
            failed_responses,
            timed_out_responses,
        }: Self,
    ) {
        self.received_packets += received_packets;
        self.dropped_packets += dropped_packets;
        self.invalid_packets += invalid_packets;
        self.proccessing_errors += proccessing_errors;
        self.info_requests += info_requests;
        self.sent_responses += sent_responses;
        self.failed_responses += failed_responses;
        self.timed_out_responses += timed_out_responses;
    }
}

impl Metrics {
    /// Resets all metrics to 0 and returns previous ones
    pub fn reset(&mut self) -> Self { std::mem::take(self) }
}
