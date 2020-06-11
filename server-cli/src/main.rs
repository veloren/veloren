#![deny(unsafe_code)]

use common::clock::Clock;
use log::info;
use server::{Event, Input, Server, ServerSettings};
use std::time::Duration;

const TPS: u64 = 30;

#[allow(clippy::redundant_pattern_matching)] // TODO: Pending review in #587
fn main() {
    // Init logging
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    info!("Starting server...");

    // Set up an fps clock
    let mut clock = Clock::start();

    // Load settings
    let settings = ServerSettings::load();
    let metrics_port = &settings.metrics_address.port();

    // Create server
    let mut server = Server::new(settings).expect("Failed to create server instance!");

    info!("Server is ready to accept connections.");
    info!("Metrics port: {}", metrics_port);

    loop {
        let events = server
            .tick(Input::default(), clock.get_last_delta())
            .expect("Failed to tick server");

        for event in events {
            match event {
                Event::ClientConnected { entity: _ } => info!("Client connected!"),
                Event::ClientDisconnected { entity: _ } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
