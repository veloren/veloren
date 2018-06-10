use bifrost::{Dispatcher, Relay, event};
use init::init_server;
use std::sync::mpsc;
use std::thread;
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
