use client::{
    addr::ConnectionArgs,
    error::{Error as ClientError, NetworkConnectError, NetworkError},
    Client, ServerInfo,
};
use common::consts::MIN_RECOMMENDED_TOKIO_THREADS;
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::runtime;
use tracing::{trace, warn};

#[derive(Debug)]
pub enum Error {
    ClientError {
        error: ClientError,
        mismatched_server_info: Option<ServerInfo>,
    },
    ClientCrashed,
    ServerNotFound,
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
}
impl ClientInit {
    #[allow(clippy::op_ref)] // TODO: Pending review in #587
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    pub fn new(
        connection_args: ConnectionArgs,
        username: String,
        password: String,
        runtime: Option<Arc<runtime::Runtime>>,
    ) -> Self {
        let (tx, rx) = unbounded();
        let (trust_tx, trust_rx) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel2 = Arc::clone(&cancel);

        let runtime = runtime.unwrap_or_else(|| {
            // TODO: evaluate std::thread::available_concurrency as a num_cpus replacement
            let cores = num_cpus::get();
            Arc::new(
                runtime::Builder::new_multi_thread()
                    .enable_all()
                    .worker_threads((cores / 4).max(MIN_RECOMMENDED_TOKIO_THREADS))
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
            let trust_fn = |auth_server: &str| {
                let _ = tx.send(Msg::IsAuthTrusted(auth_server.to_string()));
                trust_rx
                    .recv()
                    .map(|AuthTrust(server, trust)| trust && &server == auth_server)
                    .unwrap_or(false)
            };

            let mut last_err = None;

            const FOUR_MINUTES_RETRIES: u64 = 48;
            'tries: for _ in 0..FOUR_MINUTES_RETRIES {
                if cancel2.load(Ordering::Relaxed) {
                    break;
                }
                let mut mismatched_server_info = None;
                match Client::new(
                    connection_args.clone(),
                    Arc::clone(&runtime2),
                    &mut mismatched_server_info,
                )
                .await
                {
                    Ok(mut client) => {
                        if let Err(e) = client.register(username, password, trust_fn).await {
                            last_err = Some(Error::ClientError {
                                error: e,
                                mismatched_server_info: None,
                            });
                            break 'tries;
                        }
                        let _ = tx.send(Msg::Done(Ok(client)));
                        tokio::task::block_in_place(move || drop(runtime2));
                        return;
                    },
                    Err(ClientError::NetworkErr(NetworkError::ConnectFailed(
                        NetworkConnectError::Io(e),
                    ))) => {
                        warn!(?e, "Failed to connect to the server. Retrying...");
                    },
                    Err(e) => {
                        trace!(?e, "Aborting server connection attempt");
                        last_err = Some(Error::ClientError {
                            error: e,
                            mismatched_server_info,
                        });
                        break 'tries;
                    },
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }

            // Parsing/host name resolution successful but no connection succeeded
            // If last_err is None this typically means there was no server up at the input
            // address and all the attempts timed out.
            let _ = tx.send(Msg::Done(Err(last_err.unwrap_or(Error::ServerNotFound))));

            // Safe drop runtime
            tokio::task::block_in_place(move || drop(runtime2));
        });

        ClientInit {
            rx,
            trust_tx,
            cancel,
        }
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
