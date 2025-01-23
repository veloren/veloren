#[allow(deprecated)] use std::hash::SipHasher;
use std::{
    hash::{Hash, Hasher},
    io::{self, ErrorKind},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use protocol::Parcel;
use rand::{Rng, thread_rng};
use tokio::{net::UdpSocket, sync::watch};
use tracing::{debug, error, trace};

use crate::{
    proto::{
        Init, MAX_REQUEST_SIZE, MAX_RESPONSE_SIZE, QueryServerRequest, QueryServerResponse,
        RawQueryServerRequest, RawQueryServerResponse, ServerInfo, VELOREN_HEADER, VERSION,
    },
    ratelimit::{RateLimiter, ReducedIpAddr},
};

const SECRET_REGEN_INTERNVAL: Duration = Duration::from_secs(300);

pub struct QueryServer {
    addr: SocketAddr,
    server_info: watch::Receiver<ServerInfo>,
    settings: protocol::Settings,
    ratelimit: RateLimiter,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Metrics {
    pub received_packets: u32,
    pub dropped_packets: u32,
    pub invalid_packets: u32,
    pub proccessing_errors: u32,
    pub info_requests: u32,
    pub init_requests: u32,
    pub sent_responses: u32,
    pub failed_responses: u32,
    pub timed_out_responses: u32,
    pub ratelimited: u32,
}

impl QueryServer {
    pub fn new(addr: SocketAddr, server_info: watch::Receiver<ServerInfo>, ratelimit: u16) -> Self {
        Self {
            addr,
            server_info,
            ratelimit: RateLimiter::new(ratelimit),
            settings: Default::default(),
        }
    }

    /// This produces TRACE level logs for any packet received on the assigned
    /// port. To prevent potentially unfettered log spam, disable the TRACE
    /// level for this crate (when outside of debugging contexts).
    ///
    /// NOTE: TRACE and DEBUG levels are disabled by default for this crate when
    /// using `veloren-common-frontend`.
    pub async fn run(&mut self, metrics: Arc<Mutex<Metrics>>) -> Result<(), tokio::io::Error> {
        let mut socket = UdpSocket::bind(self.addr).await?;

        let gen_secret = || {
            let mut rng = thread_rng();
            (rng.gen::<u64>(), rng.gen::<u64>())
        };
        let mut secrets = gen_secret();
        let mut last_secret_refresh = Instant::now();

        let mut buf = Box::new([0; MAX_REQUEST_SIZE]);
        loop {
            let (len, remote_addr) = match socket.recv_from(&mut *buf).await {
                Ok(v) => v,
                Err(e) if e.kind() == ErrorKind::NotConnected => {
                    error!(
                        ?e,
                        "Query server connection was closed, re-binding to socket..."
                    );
                    socket = UdpSocket::bind(self.addr).await?;
                    continue;
                },
                err => {
                    debug!(?err, "Error while receiving from query server socket");
                    continue;
                },
            };

            let mut new_metrics = Metrics {
                received_packets: 1,
                ..Default::default()
            };

            let raw_msg_buf = &buf[..len];
            let msg_buf = if Self::validate_datagram(raw_msg_buf) {
                // Require 2 extra bytes for version (currently unused)
                &raw_msg_buf[2..(raw_msg_buf.len() - VELOREN_HEADER.len())]
            } else {
                new_metrics.dropped_packets += 1;
                continue;
            };

            self.process_datagram(msg_buf, remote_addr, secrets, &mut new_metrics, &socket)
                .await;

            // Update metrics at the end of eath packet
            if let Ok(mut metrics) = metrics.lock() {
                *metrics += new_metrics;
            }

            {
                let now = Instant::now();
                if now.duration_since(last_secret_refresh) > SECRET_REGEN_INTERNVAL {
                    last_secret_refresh = now;
                    secrets = gen_secret();
                }

                self.ratelimit.maintain(now);
            }
        }
    }

    // Header must be discarded after this validation passes
    fn validate_datagram(data: &[u8]) -> bool {
        let len = data.len();
        // Require 2 extra bytes for version (currently unused)
        if len < MAX_RESPONSE_SIZE.max(VELOREN_HEADER.len() + 2) {
            trace!(?len, "Datagram too short");
            false
        } else if len > MAX_REQUEST_SIZE {
            trace!(?len, "Datagram too large");
            false
        } else if data[(len - VELOREN_HEADER.len())..] != VELOREN_HEADER {
            trace!(?len, "Datagram header invalid");
            false
        // TODO: Allow lower versions once proper versioning is added.
        } else if u16::from_ne_bytes(data[..2].try_into().unwrap()) != VERSION {
            trace!(
                "Datagram has invalid version {:?}, current {VERSION:?}",
                &data[..2]
            );
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
        metrics: &mut Metrics,
        socket: &UdpSocket,
    ) {
        let Ok(RawQueryServerRequest {
            p: client_p,
            request,
        }) =
            <RawQueryServerRequest as Parcel>::read(&mut io::Cursor::new(datagram), &self.settings)
        else {
            metrics.invalid_packets += 1;
            return;
        };

        trace!(?request, "Received packet");

        #[allow(deprecated)]
        let real_p = {
            // Use SipHash-2-4 to compute the `p` value from a server specific
            // secret and the client's address.
            //
            // This is used to verify that packets are from an entity that can
            // receive packets at the given address.
            //
            // Only use the first 64 bits from Ipv6 addresses since the latter
            // 64 bits can change very frequently (as much as for every
            // request).
            let mut hasher = SipHasher::new_with_keys(secrets.0, secrets.1);
            ReducedIpAddr::from(remote.ip()).hash(&mut hasher);
            hasher.finish()
        };

        if real_p != client_p {
            Self::send_response(
                RawQueryServerResponse::Init(Init {
                    p: real_p,
                    max_supported_version: VERSION,
                }),
                remote,
                socket,
                metrics,
            )
            .await;

            return;
        }

        if !self.ratelimit.can_request(remote.ip().into()) {
            trace!("Ratelimited request");
            metrics.ratelimited += 1;
            return;
        }

        match request {
            QueryServerRequest::Init => {
                metrics.init_requests += 1;
                Self::send_response(
                    RawQueryServerResponse::Init(Init {
                        p: real_p,
                        max_supported_version: VERSION,
                    }),
                    remote,
                    socket,
                    metrics,
                )
                .await;
            },
            QueryServerRequest::ServerInfo => {
                metrics.info_requests += 1;
                let server_info = *self.server_info.borrow();
                Self::send_response(
                    RawQueryServerResponse::Response(QueryServerResponse::ServerInfo(server_info)),
                    remote,
                    socket,
                    metrics,
                )
                .await;
            },
        }
    }

    async fn send_response(
        response: RawQueryServerResponse,
        addr: SocketAddr,
        socket: &UdpSocket,
        metrics: &mut Metrics,
    ) {
        // TODO: Once more versions are added, send the packet in the same version as
        // the request here.
        match <RawQueryServerResponse as Parcel>::raw_bytes(&response, &Default::default()) {
            Ok(data) => {
                if data.len() > MAX_RESPONSE_SIZE {
                    error!(
                        ?MAX_RESPONSE_SIZE,
                        "Attempted to send a response larger than the maximum allowed size (size: \
                         {}, response: {response:?})",
                        data.len()
                    );
                    #[cfg(debug_assertions)]
                    panic!(
                        "Attempted to send a response larger than the maximum allowed size (size: \
                         {}, max: {}, response: {response:?})",
                        data.len(),
                        MAX_RESPONSE_SIZE
                    );
                }

                match socket.send_to(&data, addr).await {
                    Ok(_) => {
                        metrics.sent_responses += 1;
                    },
                    Err(err) => {
                        metrics.failed_responses += 1;
                        debug!(?err, "Failed to send query server response");
                    },
                }
            },
            Err(error) => {
                trace!(?error, "Failed to serialize response");
                #[cfg(debug_assertions)]
                panic!("Serializing response failed: {error:?} ({response:?})");
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
            init_requests,
            sent_responses,
            failed_responses,
            timed_out_responses,
            ratelimited,
        }: Self,
    ) {
        self.received_packets += received_packets;
        self.dropped_packets += dropped_packets;
        self.invalid_packets += invalid_packets;
        self.proccessing_errors += proccessing_errors;
        self.info_requests += info_requests;
        self.init_requests += init_requests;
        self.sent_responses += sent_responses;
        self.failed_responses += failed_responses;
        self.timed_out_responses += timed_out_responses;
        self.ratelimited += ratelimited;
    }
}

impl Metrics {
    /// Resets all metrics to 0 and returns previous ones
    ///
    /// Used by the consumer of the metrics.
    pub fn reset(&mut self) -> Self { std::mem::take(self) }
}
