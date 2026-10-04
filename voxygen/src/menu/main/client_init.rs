use client::{
    Client, ClientInitStage, CredentialProvider, Credentials, ServerInfo,
    addr::ConnectionArgs,
    error::{Error as ClientError, NetworkConnectError, NetworkError},
};
use common_net::msg::ClientType;
use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::runtime;
use tracing::{trace, warn};

#[derive(Debug)]
#[expect(clippy::enum_variant_names)] //TODO: evaluate ClientError ends with Enum name
pub enum Error {
    ClientError {
        error: ClientError,
        mismatched_server_info: Option<ServerInfo>,
    },
    ClientCrashed,
    ServerNotFound,
}

#[expect(clippy::large_enum_variant)]
pub enum InitOutMsg {
    IsAuthTrusted(String),
    RequestCredentials(Option<String>),
    Done(Result<Client, Error>),
}

pub struct AuthResponseMsg(String, bool);
pub struct CredentialsResponseMsg(Option<String>, Result<Credentials, ()>);

struct UiCredentialProvider {
    singleplayer: bool,
    request_tx: Sender<InitOutMsg>,
    auth_response_rx: Receiver<AuthResponseMsg>,
    credentials_response_rx: Receiver<CredentialsResponseMsg>,
}

impl CredentialProvider for UiCredentialProvider {
    fn auth_trusted(&self, auth_provider: &Option<String>) -> bool {
        match auth_provider {
            None => true, //acccept servers without auth
            Some(auth_provider) => {
                let _ = self
                    .request_tx
                    .send(InitOutMsg::IsAuthTrusted(auth_provider.clone()));

                self.auth_response_rx
                    .recv()
                    .map(|AuthResponseMsg(server, trust)| trust && server == *auth_provider)
                    .unwrap_or(false)
            },
        }
    }

    async fn get_credentials(&self, auth_provider: &Option<String>) -> Result<Credentials, ()> {
        if self.singleplayer {
            return Ok(Credentials::Username("singleplayer".to_string()));
        }

        let _ = self
            .request_tx
            .send(InitOutMsg::RequestCredentials(auth_provider.clone()));

        let response = self.credentials_response_rx.recv().map_err(|_| ())?;
        if &response.0 == auth_provider {
            response.1
        } else {
            Err(())
        }
    }
}

// Used to asynchronously parse the server address, resolve host names,
// and create the client (which involves establishing a connection to the
// server).
pub struct ClientInit {
    rx: Receiver<InitOutMsg>,
    stage_rx: Receiver<ClientInitStage>,
    auth_tx: Sender<AuthResponseMsg>,
    credentials_tx: Sender<CredentialsResponseMsg>,
    cancel: Arc<AtomicBool>,
}
impl ClientInit {
    pub fn new(
        connection_args: ConnectionArgs,
        runtime: Arc<runtime::Runtime>,
        locale: Option<String>,
        config_dir: &Path,
        client_type: ClientType,
    ) -> Self {
        let (tx, rx) = unbounded();
        let (auth_tx, auth_rx) = unbounded();
        let (credentials_tx, credentials_rx) = unbounded();
        let (init_stage_tx, init_stage_rx) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel2 = Arc::clone(&cancel);

        let runtime2 = Arc::clone(&runtime);
        let config_dir = config_dir.to_path_buf();

        runtime.spawn(async move {
            let credential_provider = UiCredentialProvider {
                singleplayer: matches!(connection_args, ConnectionArgs::Mpsc(_)),
                request_tx: tx.clone(),
                auth_response_rx: auth_rx,
                credentials_response_rx: credentials_rx,
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
                    locale.clone(),
                    &credential_provider,
                    &|stage| {
                        let _ = init_stage_tx.send(stage);
                    },
                    crate::ecs::sys::add_local_systems,
                    config_dir.clone(),
                    client_type,
                )
                .await
                {
                    Ok(client) => {
                        let _ = tx.send(InitOutMsg::Done(Ok(client)));
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
            let _ = tx.send(InitOutMsg::Done(Err(
                last_err.unwrap_or(Error::ServerNotFound)
            )));

            // Safe drop runtime
            tokio::task::block_in_place(move || drop(runtime2));
        });

        ClientInit {
            rx,
            stage_rx: init_stage_rx,
            auth_tx,
            credentials_tx,
            cancel,
        }
    }

    /// Poll if the thread is complete.
    /// Returns None if the thread is still running, otherwise returns the
    /// Result of client creation.
    pub fn poll(&self) -> Option<InitOutMsg> {
        match self.rx.try_recv() {
            Ok(msg) => Some(msg),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(InitOutMsg::Done(Err(Error::ClientCrashed))),
        }
    }

    /// Poll for connection stage updates from the client
    pub fn stage_update(&self) -> Option<ClientInitStage> { self.stage_rx.try_recv().ok() }

    /// Answer a request, by giving the trust status for a requested auth_server
    pub fn auth_trust(&self, auth_server: String, trusted: bool) {
        let _ = self.auth_tx.send(AuthResponseMsg(auth_server, trusted));
    }

    /// Answer a request, by giving the credentials for a requested auth_server
    pub fn provide_credentials(
        &self,
        auth_server: Option<String>,
        credentials: Result<Credentials, ()>,
    ) {
        let _ = self
            .credentials_tx
            .send(CredentialsResponseMsg(auth_server, credentials));
    }

    pub fn cancel(&mut self) { self.cancel.store(true, Ordering::Relaxed); }
}

impl Drop for ClientInit {
    fn drop(&mut self) { self.cancel(); }
}
