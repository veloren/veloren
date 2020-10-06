use client::Client;
use common::clock::Clock;
use crossbeam::channel::{bounded, unbounded, Receiver, Sender, TryRecvError};
use server::{Error as ServerError, Event, Input, Server};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tracing::{info, warn};

const TPS: u64 = 30;

enum Msg {
    Stop,
}

/// Used to start and stop the background thread running the server
/// when in singleplayer mode.
pub struct Singleplayer {
    _server_thread: JoinHandle<()>,
    sender: Sender<Msg>,
    pub receiver: Receiver<Result<(), ServerError>>,
    // Wether the server is stopped or not
    paused: Arc<AtomicBool>,
}

impl Singleplayer {
    pub fn new(client: Option<&Client>) -> (Self, server::Settings) {
        let (sender, receiver) = unbounded();

        // Determine folder to save server data in
        let server_data_dir = {
            let mut path = common::userdata_dir_workspace!();
            path.push("singleplayer");
            path
        };

        // Create server
        let settings = server::Settings::singleplayer(&server_data_dir);
        let editable_settings = server::EditableSettings::singleplayer(&server_data_dir);

        let thread_pool = client.map(|c| c.thread_pool().clone());
        let settings2 = settings.clone();

        let paused = Arc::new(AtomicBool::new(false));
        let paused1 = Arc::clone(&paused);

        let (result_sender, result_receiver) = bounded(1);

        let thread = thread::spawn(move || {
            let mut server = None;
            if let Err(e) = result_sender.send(
                match Server::new(settings2, editable_settings, &server_data_dir) {
                    Ok(s) => {
                        server = Some(s);
                        Ok(())
                    },
                    Err(e) => Err(e),
                },
            ) {
                warn!(
                    ?e,
                    "Failed to send singleplayer server initialization result. Most likely the \
                     channel was closed by cancelling server creation. Stopping Server"
                );
                return;
            };

            let server = match server {
                Some(s) => s,
                None => return,
            };

            let server = match thread_pool {
                Some(pool) => server.with_thread_pool(pool),
                None => server,
            };

            run_server(server, receiver, paused1);
        });

        (
            Singleplayer {
                _server_thread: thread,
                sender,
                receiver: result_receiver,
                paused,
            },
            settings,
        )
    }

    /// Returns wether or not the server is paused
    pub fn is_paused(&self) -> bool { self.paused.load(Ordering::SeqCst) }

    /// Pauses if true is passed and unpauses if false (Does nothing if in that
    /// state already)
    pub fn pause(&self, state: bool) { self.paused.store(state, Ordering::SeqCst); }
}

impl Drop for Singleplayer {
    fn drop(&mut self) {
        // Ignore the result
        let _ = self.sender.send(Msg::Stop);
    }
}

fn run_server(mut server: Server, rec: Receiver<Msg>, paused: Arc<AtomicBool>) {
    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::start();

    loop {
        // Check any event such as stopping and pausing
        match rec.try_recv() {
            Ok(msg) => match msg {
                Msg::Stop => break,
            },
            Err(err) => match err {
                TryRecvError::Empty => (),
                TryRecvError::Disconnected => break,
            },
        }

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));

        // Skip updating the server if it's paused
        if paused.load(Ordering::SeqCst) && server.number_of_players() < 2 {
            continue;
        } else if server.number_of_players() > 1 {
            paused.store(false, Ordering::SeqCst);
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
