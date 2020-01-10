use client::Client;
use common::clock::Clock;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::info;
use server::{Event, Input, Server, ServerSettings};
use std::{
    thread::{self, JoinHandle},
    time::Duration,
};

const TPS: u64 = 30;

enum Msg {
    Stop,
}

/// Used to start and stop the background thread running the server
/// when in singleplayer mode.
pub struct Singleplayer {
    _server_thread: JoinHandle<()>,
    sender: Sender<Msg>,
}

impl Singleplayer {
    pub fn new(client: Option<&Client>) -> (Self, ServerSettings) {
        let (sender, receiver) = unbounded();

        // Create server
        let settings = ServerSettings::singleplayer();

        let thread_pool = client.map(|c| c.thread_pool().clone());
        let settings2 = settings.clone();

        let thread = thread::spawn(move || {
            let server = Server::new(settings2).expect("Failed to create server instance!");

            let server = match thread_pool {
                Some(pool) => server.with_thread_pool(pool),
                None => server,
            };

            run_server(server, receiver);
        });

        (
            Singleplayer {
                _server_thread: thread,
                sender,
            },
            settings,
        )
    }
}

impl Drop for Singleplayer {
    fn drop(&mut self) {
        // Ignore the result
        let _ = self.sender.send(Msg::Stop);
    }
}

fn run_server(mut server: Server, rec: Receiver<Msg>) {
    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::start();

    loop {
        let events = server
            .tick(Input::default(), clock.get_last_delta())
            .expect("Failed to tick server!");

        for event in events {
            match event {
                Event::ClientConnected { .. } => info!("Client connected!"),
                Event::ClientDisconnected { .. } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();

        match rec.try_recv() {
            Ok(_msg) => break,
            Err(err) => match err {
                TryRecvError::Empty => (),
                TryRecvError::Disconnected => break,
            },
        }

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
