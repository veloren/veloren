use client::{error::Error as ClientError, Client};
use common::{comp, net::PostError};
use crossbeam::channel::{unbounded, Receiver, TryRecvError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{net::ToSocketAddrs, thread, time::Duration};

#[derive(Debug)]
pub enum Error {
    // Error parsing input string or error resolving host name.
    BadAddress(std::io::Error),
    // Parsing/host name resolution successful but could not connect.
    #[allow(dead_code)]
    ConnectionFailed(ClientError),
    // Parsing yielded an empty iterator (specifically to_socket_addrs()).
    NoAddress,
    InvalidAuth,
    ClientCrashed,
    ServerIsFull,
}

// Used to asynchronously parse the server address, resolve host names,
// and create the client (which involves establishing a connection to the server).
pub struct ClientInit {
    rx: Receiver<Result<Client, Error>>,
    cancel: Arc<AtomicBool>,
}
impl ClientInit {
    pub fn new(
        connection_args: (String, u16, bool),
        player: comp::Player,
        password: String,
    ) -> Self {
        let (server_address, default_port, prefer_ipv6) = connection_args;

        let (tx, rx) = unbounded();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel2 = Arc::clone(&cancel);

        thread::spawn(move || {
            // Parse ip address or resolves hostname.
            // Note: if you use an ipv6 address, the number after the last colon will be used
            // as the port unless you use [] around the address.
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
                            match Client::new(socket_addr, player.view_distance) {
                                Ok(mut client) => {
                                    if let Err(ClientError::InvalidAuth) =
                                        client.register(player.clone(), password.clone())
                                    {
                                        last_err = Some(Error::InvalidAuth);
                                        break;
                                    }
                                    //client.register(player, password);
                                    let _ = tx.send(Ok(client));
                                    return;
                                }
                                Err(err) => {
                                    match err {
                                        ClientError::Network(PostError::Bincode(_)) => {
                                            last_err = Some(Error::ConnectionFailed(err));
                                            break 'tries;
                                        }
                                        // Assume the connection failed and try again soon
                                        ClientError::Network(_) => {}
                                        ClientError::TooManyPlayers => {
                                            last_err = Some(Error::ServerIsFull);
                                            break 'tries;
                                        }
                                        ClientError::InvalidAuth => {
                                            last_err = Some(Error::InvalidAuth);
                                            break 'tries;
                                        }
                                        // TODO: Handle errors?
                                        _ => panic!(
                                        "Unexpected non-network error when creating client: {:?}",
                                        err
                                    ),
                                    }
                                }
                            }
                        }
                        thread::sleep(Duration::from_secs(5));
                    }
                    // Parsing/host name resolution successful but no connection succeeded.
                    let _ = tx.send(Err(last_err.unwrap_or(Error::NoAddress)));
                }
                Err(err) => {
                    // Error parsing input string or error resolving host name.
                    let _ = tx.send(Err(Error::BadAddress(err)));
                }
            }
        });

        ClientInit { rx, cancel }
    }
    /// Poll if the thread is complete.
    /// Returns None if the thread is still running, otherwise returns the Result of client creation.
    pub fn poll(&self) -> Option<Result<Client, Error>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(Error::ClientCrashed)),
        }
    }
    pub fn cancel(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

impl Drop for ClientInit {
    fn drop(&mut self) {
        self.cancel();
    }
}
