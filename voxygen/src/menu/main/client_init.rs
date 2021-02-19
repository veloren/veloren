use client::{
    error::{Error as ClientError, NetworkError},
    Client,
};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{net::lookup_host, runtime};
use tracing::{trace, warn};

#[derive(Debug)]
pub enum Error {
    // Error parsing input string or error resolving host name.
    BadAddress(std::io::Error),
    // Parsing/host name resolution successful but there was an error within the client.
    ClientError(ClientError),
    // Parsing yielded an empty iterator (specifically to_socket_addrs()).
    NoAddress,
    ClientCrashed,
}

#[allow(clippy::large_enum_variant)] // TODO: Pending review in #587
pub enum Msg {
    IsAuthTrusted(String),
    Done(Result<Client, Error>),
}

pub struct AuthTrust(String, bool);

// Used to asynchronously parse the server address, resolve host names,
// and create the client (which involves establishing a connection to the
// server).
pub struct ClientInit {
    rx: Receiver<Msg>,
    trust_tx: Sender<AuthTrust>,
    cancel: Arc<AtomicBool>,
    _runtime: Arc<runtime::Runtime>,
}
impl ClientInit {
    #[allow(clippy::op_ref)] // TODO: Pending review in #587
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    pub fn new(
        connection_args: (String, u16, bool),
        username: String,
        view_distance: Option<u32>,
        password: String,
        runtime: Option<Arc<runtime::Runtime>>,
    ) -> Self {
        let (server_address, port, prefer_ipv6) = connection_args;

        let (tx, rx) = unbounded();
        let (trust_tx, trust_rx) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel2 = Arc::clone(&cancel);

        let runtime = runtime.unwrap_or_else(|| {
            let cores = num_cpus::get();
            Arc::new(
                runtime::Builder::new_multi_thread()
                    .enable_all()
                    .worker_threads(if cores > 4 { cores - 1 } else { cores })
                    .thread_name_fn(|| {
                        static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                        let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                        format!("tokio-voxygen-{}", id)
                    })
                    .build()
                    .unwrap(),
            )
        });
        let runtime2 = Arc::clone(&runtime);

        runtime.spawn(async move {
            let addresses = match Self::resolve(server_address, port, prefer_ipv6).await {
                Ok(a) => a,
                Err(e) => {
                    let _ = tx.send(Msg::Done(Err(Error::BadAddress(e))));
                    return;
                },
            };
            let mut last_err = None;

            const FOUR_MINUTES_RETRIES: u64 = 48;
            'tries: for _ in 0..FOUR_MINUTES_RETRIES {
                if cancel2.load(Ordering::Relaxed) {
                    break;
                }
                for socket_addr in &addresses {
                    match Client::new(*socket_addr, view_distance, Arc::clone(&runtime2)).await {
                        Ok(mut client) => {
                            if let Err(e) = client
                                .register(username, password, |auth_server| {
                                    let _ = tx.send(Msg::IsAuthTrusted(auth_server.to_string()));
                                    trust_rx
                                        .recv()
                                        .map(|AuthTrust(server, trust)| {
                                            trust && &server == auth_server
                                        })
                                        .unwrap_or(false)
                                })
                                .await
                            {
                                last_err = Some(Error::ClientError(e));
                                break 'tries;
                            }
                            let _ = tx.send(Msg::Done(Ok(client)));
                            return;
                        },
                        Err(ClientError::NetworkErr(NetworkError::ConnectFailed(e))) => {
                            if e.kind() == std::io::ErrorKind::PermissionDenied {
                                warn!(?e, "Cannot connect to server: Incompatible version");
                                last_err = Some(Error::ClientError(ClientError::NetworkErr(
                                    NetworkError::ConnectFailed(e),
                                )));
                                break 'tries;
                            } else {
                                warn!(?e, "Failed to connect to the server. Retrying...");
                            }
                        },
                        Err(e) => {
                            trace!(?e, "Aborting server connection attempt");
                            last_err = Some(Error::ClientError(e));
                            break 'tries;
                        },
                    }
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }

            // Parsing/host name resolution successful but no connection succeeded.
            let _ = tx.send(Msg::Done(Err(last_err.unwrap_or(Error::NoAddress))));
        });

        ClientInit {
            rx,
            trust_tx,
            cancel,
            _runtime: runtime,
        }
    }

    /// Parse ip address or resolves hostname.
    /// Note: if you use an ipv6 address, the number after the last colon will
    /// be used as the port unless you use [] around the address.
    async fn resolve(
        server_address: String,
        port: u16,
        prefer_ipv6: bool,
    ) -> Result<Vec<SocketAddr>, std::io::Error> {
        // 1. try if server_address already contains a port
        if let Ok(addr) = server_address.parse::<SocketAddr>() {
            warn!("please don't add port directly to server_address");
            return Ok(vec![addr]);
        }

        // 2, try server_address and port
        if let Ok(addr) = format!("{}:{}", server_address, port).parse::<SocketAddr>() {
            return Ok(vec![addr]);
        }

        // 3. do DNS call
        let (mut first_addrs, mut second_addrs) = match lookup_host(server_address).await {
            Ok(s) => s.partition::<Vec<_>, _>(|a| a.is_ipv6() == prefer_ipv6),
            Err(e) => {
                return Err(e);
            },
        };

        Ok(
            std::iter::Iterator::chain(first_addrs.drain(..), second_addrs.drain(..))
                .map(|mut addr| {
                    addr.set_port(port);
                    addr
                })
                .collect(),
        )
    }

    /// Poll if the thread is complete.
    /// Returns None if the thread is still running, otherwise returns the
    /// Result of client creation.
    pub fn poll(&self) -> Option<Msg> {
        match self.rx.try_recv() {
            Ok(msg) => Some(msg),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Msg::Done(Err(Error::ClientCrashed))),
        }
    }

    /// Report trust status of auth server
    pub fn auth_trust(&self, auth_server: String, trusted: bool) {
        let _ = self.trust_tx.send(AuthTrust(auth_server, trusted));
    }

    pub fn cancel(&mut self) { self.cancel.store(true, Ordering::Relaxed); }
}

impl Drop for ClientInit {
    fn drop(&mut self) { self.cancel(); }
}
