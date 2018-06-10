use bifrost::{Dispatcher, Relay};
use bifrost::event::event;
use init::init_server;
use std::sync::mpsc;
use std::thread;
use world_context::World;

pub struct Server {
    relay: Relay<World>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Server {
    pub fn new() -> Server {
        let (tx, rx) = mpsc::channel::<Relay<World>>();

        let handle = thread::spawn(move || {
            let mut world = World::new();
            let mut dispatcher = Dispatcher::<World>::new(&mut world);
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
