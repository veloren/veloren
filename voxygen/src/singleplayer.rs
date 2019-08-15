use client::Client;
use common::clock::Clock;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use log::info;
use portpicker::pick_unused_port;
use server::{Event, Input, Server, ServerSettings};
use std::{
    net::SocketAddr,
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(feature = "discord")]
use crate::{discord, discord::DiscordUpdate};

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
    pub fn new(client: Option<&Client>) -> (Self, SocketAddr) {
        let (sender, receiver) = unbounded();

        let sock = SocketAddr::from((
            [127, 0, 0, 1],
            pick_unused_port().expect("Failed to find unused port!"),
        ));

        // Create server
        let server = Server::bind(sock.clone(), ServerSettings::singleplayer())
            .expect("Failed to create server instance!");

        let server = match client {
            Some(client) => server.with_thread_pool(client.thread_pool().clone()),
            None => server,
        };

        let thread = thread::spawn(move || {
            run_server(server, receiver);
        });

        (
            Singleplayer {
                _server_thread: thread,
                sender,
            },
            sock,
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

    #[cfg(feature = "discord")]
    {
        discord::send_all(vec![
            DiscordUpdate::Details("Singleplayer".into()),
            DiscordUpdate::State("Playing...".into()),
        ]);
    }

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
