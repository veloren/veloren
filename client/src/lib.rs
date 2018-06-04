#![feature(nll)]

extern crate network;
extern crate region;

mod client;

// Reexports
pub use network::ClientMode as ClientMode;
pub use region::Volume as Volume;

use std::thread;
use std::sync::{Mutex, Arc};
use std::net::ToSocketAddrs;

use client::Client;

// Errors that may occur within this crate
#[derive(Debug)]
pub enum Error {
    NetworkErr(network::Error),
}

impl From<network::Error> for Error {
    fn from(e: network::Error) -> Error {
        Error::NetworkErr(e)
    }
}

// A thread-safe client handle
pub struct ClientHandle {
    client: Arc<Mutex<Client>>,
}

impl ClientHandle {
    // Create a new client from a set of parameters and return a handle to it
    pub fn new<T: ToSocketAddrs, U: ToSocketAddrs>(mode: ClientMode, alias: &str, bind_addr: T, remote_addr: U) -> Result<ClientHandle, Error> {
        Ok(ClientHandle {
            client: Arc::new(Mutex::new(match client::Client::new(mode, alias, bind_addr, remote_addr) {
                Ok(c) => c,
                Err(e) => return Err(e),
            })),
        })
    }

    pub fn run(&mut self) {
        let client_ref = self.client.clone();
        thread::spawn(move || {
            let conn = client_ref.lock().unwrap().conn();
            while client_ref.lock().unwrap().running() {
                match conn.recv() {
                    Ok(data) => client_ref.lock().unwrap().handle_packet(data.1),
                    Err(e) => println!("[WARNING] Receive error: {:?}", e),
                }
            }
        });
    }

    pub fn set_chat_callback<F: 'static + Fn(&str, &str) + Send>(&self, f: F) {
        self.client.lock().unwrap().set_chat_callback(f);
    }

    pub fn send_chat_msg(&self, msg: &str) -> Result<(), Error> {
        self.client.lock().unwrap().send_chat_msg(msg)
    }

        pub fn send_command(&self, cmd: &str) -> Result<(), Error> {
        self.client.lock().unwrap().send_command(cmd)
    }
}
