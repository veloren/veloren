use std::time::Duration;
use log::info;
use server::{Input, Event, Server};
use common::clock::Clock;
use std::{
    thread,
    thread::JoinHandle
};
use std::sync::mpsc::{
    channel, Receiver, Sender
};

const TPS: u64 = 30;

enum Msg {
    Stop,
}

pub struct Singleplayer {
    server_thread: JoinHandle<()>,
    sender: Sender<Msg>,
}

impl Singleplayer {
    pub fn new() -> Self {
        let (sender, reciever) = channel();
        let thread = thread::spawn(move || {
            run_server(reciever);
        });
        Singleplayer {
            server_thread: thread,
            sender,
        }
    }
}

impl Drop for Singleplayer {
    fn drop(&mut self) {
        self.sender.send(Msg::Stop);
    }
}

fn run_server(rec: Receiver<Msg>) {
    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::new();

    // Create server
    let mut server = Server::new()
        .expect("Failed to create server instance");

    loop {
        let events = server.tick(Input::default(), clock.get_last_delta())
            .expect("Failed to tick server");

        for event in events {
            match event {
                Event::ClientConnected { entity } => info!("Client connected!"),
                Event::ClientDisconnected { entity } => info!("Client disconnected!"),
                Event::Chat { entity, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick
        server.cleanup();

        match rec.try_recv() {
            Ok(msg) => break,
            Err(err) => match err {
                Empty => (),
                Disconnected => break,
            },
        }

        // Wait for the next tick
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
