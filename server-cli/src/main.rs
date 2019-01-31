// Standard
use std::time::Duration;

// Library
use log::info;

// Project
use server::{self, Server};
use common::clock::Clock;

const FPS: u64 = 60;

fn main() {
    // Init logging
    pretty_env_logger::init();

    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::new();

    // Create server
    let mut server = Server::new();

    loop {
        server.tick(server::Input {}, clock.get_last_delta())
            .expect("Failed to tick server");

        // Clean up the server after a tick
        server.cleanup();

        // Wait for the next tick
        clock.tick(Duration::from_millis(1000 / FPS));
    }
}
