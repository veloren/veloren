use client::{error::Error as ClientError, Client};
use common::comp;
use std::{
    sync::mpsc::{channel, Receiver, TryRecvError},
    thread::{self, JoinHandle},
};

#[derive(Debug)]
pub enum Error {
    // Error parsing input string or error resolving host name
    BadAddress(std::io::Error),
    // Parsing yielded an empty iterator (specifically to_socket_addrs())
    NoAddress,
    // Parsing/host name resolution successful but could not connect
    ConnectionFailed(ClientError),
}

// Used to asynchronusly parse the server address, resolve host names, and create the client (which involves establishing a connection to the server)
pub struct ClientInit {
    rx: Receiver<Result<Client, Error>>,
}
impl ClientInit {
    pub fn new(
        connection_args: (String, u16, bool),
        client_args: (comp::Player, Option<comp::Character>, Option<comp::Animation>, u64),
    ) -> Self {
        let (server_address, default_port, prefer_ipv6) = connection_args;
        let (player, character, animation, view_distance) = client_args;

        let (tx, rx) = channel();

        let handle = Some(thread::spawn(move || {
            use std::net::ToSocketAddrs;
            // Parses ip address or resolves hostname
            // Note: if you use an ipv6 address the number after the last colon will be used as the port unless you use [] around the address
            match server_address
                .to_socket_addrs()
                .or((server_address.as_ref(), default_port).to_socket_addrs())
            {
                Ok(socket_adders) => {
                    let (first_addrs, second_addrs) =
                        socket_adders.partition::<Vec<_>, _>(|a| a.is_ipv6() == prefer_ipv6);

                    let mut last_err = None;

                    for socket_addr in first_addrs.into_iter().chain(second_addrs) {
                        match Client::new(socket_addr, player.clone(), character, view_distance) {
                            Ok(client) => {
                                let _ = tx.send(Ok(client));
                                return;
                            }
                            Err(err) => {
                                match err {
                                    // assume connection failed and try next address
                                    ClientError::Network(_) => {
                                        last_err = Some(Error::ConnectionFailed(err))
                                    }
                                    // TODO: handle error?
                                    _ => panic!(
                                        "Unexpected non-network error when creating client: {:?}",
                                        err
                                    ),
                                }
                            }
                        }
                    }
                    // Parsing/host name resolution successful but no connection succeeded
                    let _ = tx.send(Err(last_err.unwrap_or(Error::NoAddress)));
                }
                Err(err) => {
                    // Error parsing input string or error resolving host name
                    let _ = tx.send(Err(Error::BadAddress(err)));
                }
            }
        }));

        ClientInit { rx }
    }
    // Returns None is the thread is still running
    // Otherwise returns the Result of client creation
    pub fn poll(&self) -> Option<Result<Client, Error>> {
        match self.rx.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => panic!("Thread panicked or already finished"),
        }
    }
}
