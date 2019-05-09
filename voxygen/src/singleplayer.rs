use std::{
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
    time::Duration,
    thread::{self, JoinHandle},
    net::SocketAddr,
};
use log::info;
use portpicker::pick_unused_port;
use common::clock::Clock;
use server::{Event, Input, Server};

const TPS: u64 = 30;

enum Msg {
    Stop,
}

/// Used to start and stop the background thread running the server
/// when in singleplayer mode.
pub struct Singleplayer {
    server_thread: JoinHandle<()>,
    sender: Sender<Msg>,
}

impl Singleplayer {
    pub fn new() -> (Self, SocketAddr) {
        let (sender, reciever) = channel();

        let sock = SocketAddr::from(([127,0,0,1], pick_unused_port()
            .expect("Failed to find unused port")));

        let sock2 = sock.clone();
        let thread = thread::spawn(move || {
            run_server(sock2, reciever);
        });

        (Singleplayer {
            server_thread: thread,
            sender,
        }, sock)
    }
}

impl Drop for Singleplayer {
    fn drop(&mut self) {
        self.sender.send(Msg::Stop);
    }
}

fn run_server(sock: SocketAddr, rec: Receiver<Msg>) {
    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::new();

    // Create server
    let mut server = Server::bind(sock).expect("Failed to create server instance");

    loop {
        let events = server
            .tick(Input::default(), clock.get_last_delta())
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
                TryRecvError::Empty => (),
                TryRecvError::Disconnected => break,
            },
        }

        // Wait for the next tick
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
