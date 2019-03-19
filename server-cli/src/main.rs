use std::time::Duration;
use log::info;
use server::{Input, Event, Server};
use common::clock::Clock;

const TPS: u64 = 30;

fn main() {
    // Init logging
    pretty_env_logger::init();

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
                Event::ClientConnected { ecs_entity } => info!("Client connected!"),
                Event::ClientDisconnected { ecs_entity } => info!("Client disconnected!"),
                Event::Chat { ecs_entity, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick
        server.cleanup();

        // Wait for the next tick
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
