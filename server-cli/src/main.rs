use std::time::Duration;
use log::info;
use server::{Input, Event, Server};
use common::clock::Clock;

const FPS: u64 = 60;

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
                Event::ClientConnected { ecs_entity } => println!("Client connected!"),
                Event::ClientDisconnected { ecs_entity } => println!("Client disconnected!"),
                Event::Chat { msg, .. } => println!("[chat] {}", msg),
            }
        }

        // Clean up the server after a tick
        server.cleanup();

        // Wait for the next tick
        clock.tick(Duration::from_millis(1000 / FPS));
    }
}
