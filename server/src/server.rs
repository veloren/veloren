// Standard
use std::thread;
use std::sync::mpsc;

// Library
use bifrost::{Dispatcher, Relay, event};

// Local
use init::init_server;
use server_context::ServerContext;

pub struct Server {
    relay: Relay<ServerContext>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Server {
    pub fn new() -> Server {
        let (tx, rx) = mpsc::channel::<Relay<ServerContext>>();

        let handle = thread::spawn(move || {
            let mut world = ServerContext::new();
            let mut dispatcher = Dispatcher::<ServerContext>::new(&mut world);
            let relay = dispatcher.create_relay();
            tx.send(relay).unwrap();

            dispatcher.run();
        });

        Server {
            relay: rx.recv().unwrap(),
            handle: Some(handle),
        }
    }

    pub fn start(&self) {
        self.relay.send(
            event(init_server)
        );
    }

    pub fn stop(&mut self) {
        self.relay.stop();
        self.handle.take().map(|handle| handle.join());
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.handle.take().map(|handle| handle.join());
    }
}
