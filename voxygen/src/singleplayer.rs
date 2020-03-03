use client::Client;
use common::clock::Clock;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::info;
use server::{Event, Input, Server, ServerSettings};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::{self, JoinHandle},
    time::Duration,
};

const TPS: u64 = 30;

enum Msg {
    Stop,
    Pause(bool),
}

/// Used to start and stop the background thread running the server
/// when in singleplayer mode.
pub struct Singleplayer {
    _server_thread: JoinHandle<()>,
    sender: Sender<Msg>,
    // Wether the server is stopped or not
    paused: AtomicBool,
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
                paused: AtomicBool::new(false),
            },
            settings,
        )
    }

    /// Returns wether or not the server is paused
    pub fn is_paused(&self) -> bool { self.paused.load(Ordering::Relaxed) }

    /// Pauses if true is passed and unpauses if false (Does nothing if in that
    /// state already)
    pub fn pause(&self, state: bool) {
        self.paused.load(Ordering::SeqCst);
        let _ = self.sender.send(Msg::Pause(state));
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
    let mut paused = false;

    loop {
        // Check any event such as stopping and pausing
        match rec.try_recv() {
            Ok(msg) => match msg {
                Msg::Stop => break,
                Msg::Pause(val) => paused = val,
            },
            Err(err) => match err {
                TryRecvError::Empty => (),
                TryRecvError::Disconnected => break,
            },
        }

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));

        // Skip updating the server if it's paused
        if paused {
            continue;
        }

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
    }
}
