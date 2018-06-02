#![feature(nll)]

extern crate network;
extern crate region;

mod client;

// Reexports
pub use client::ClientMode as ClientMode;
pub use region::Volume as Volume;

use std::thread;
use std::sync::{Mutex, Arc};
use std::net::ToSocketAddrs;

use client::Client;

#[derive(Debug)]
pub enum Error {
    ConnectionErr,
}

pub struct ClientHandle {
    client: Arc<Mutex<Client>>,
}

impl ClientHandle {
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
                let data = conn.recv();
                client_ref.lock().unwrap().handle_packet(data);
            }
        });
    }

    pub fn set_chat_callback<F: 'static + Fn(&str, &str) + Send>(&self, f: F) {
        self.client.lock().unwrap().set_chat_callback(f);
    }

    pub fn send_chat_msg(&self, msg: &str) -> bool {
        self.client.lock().unwrap().send_chat_msg(msg)
    }
}
