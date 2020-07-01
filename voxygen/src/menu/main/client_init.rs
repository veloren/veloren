use client::{error::Error as ClientError, Client, error::NetworkError};
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::{
    net::ToSocketAddrs,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

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
}
impl ClientInit {
    #[allow(clippy::op_ref)] // TODO: Pending review in #587
    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    pub fn new(
        connection_args: (String, u16, bool),
        username: String,
        view_distance: Option<u32>,
        password: String,
    ) -> Self {
        let (server_address, default_port, prefer_ipv6) = connection_args;

        let (tx, rx) = unbounded();
        let (trust_tx, trust_rx) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel2 = Arc::clone(&cancel);

        thread::spawn(move || {
            // Parse ip address or resolves hostname.
            // Note: if you use an ipv6 address, the number after the last colon will be
            // used as the port unless you use [] around the address.
            match server_address
                .to_socket_addrs()
                .or((server_address.as_ref(), default_port).to_socket_addrs())
            {
                Ok(socket_address) => {
                    let (first_addrs, second_addrs) =
                        socket_address.partition::<Vec<_>, _>(|a| a.is_ipv6() == prefer_ipv6);

                    let mut last_err = None;

                    'tries: for _ in 0..960 + 1 {
                        // 300 Seconds
                        if cancel2.load(Ordering::Relaxed) {
                            break;
                        }
                        for socket_addr in
                            first_addrs.clone().into_iter().chain(second_addrs.clone())
                        {
                            match Client::new(socket_addr, view_distance) {
                                Ok(mut client) => {
                                    if let Err(err) =
                                        client.register(username, password, |auth_server| {
                                            let _ = tx
                                                .send(Msg::IsAuthTrusted(auth_server.to_string()));
                                            trust_rx
                                                .recv()
                                                .map(|AuthTrust(server, trust)| {
                                                    trust && &server == auth_server
                                                })
                                                .unwrap_or(false)
                                        })
                                    {
                                        last_err = Some(Error::ClientError(err));
                                        break 'tries;
                                    }
                                    let _ = tx.send(Msg::Done(Ok(client)));
                                    return;
                                },
                                Err(err) => {
                                    match err {
                                        ClientError::NetworkErr(NetworkError::ListenFailed(..)) => {},
                                        // Non-connection error, stop attempts
                                        err => {
                                            last_err = Some(Error::ClientError(err));
                                            break 'tries;
                                        },
                                    }
                                },
                            }
                        }
                        thread::sleep(Duration::from_secs(5));
                    }
                    // Parsing/host name resolution successful but no connection succeeded.
                    let _ = tx.send(Msg::Done(Err(last_err.unwrap_or(Error::NoAddress))));
                },
                Err(err) => {
                    // Error parsing input string or error resolving host name.
                    let _ = tx.send(Msg::Done(Err(Error::BadAddress(err))));
                },
            }
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
