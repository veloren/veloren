use std::time::Duration;
use log::info;
use client::{Input, Client, Event};
use common::clock::Clock;

const FPS: u64 = 60;

fn main() {
    // Init logging
    pretty_env_logger::init();

    info!("Starting chat-cli...");

    // Set up an fps clock
    let mut clock = Clock::new();

    // Create client
    let mut client = Client::new(([127, 0, 0, 1], 59003))
        .expect("Failed to create client instance");

    client.send_chat("Hello!".to_string());

    loop {
        let events = match client.tick(Input::default(), clock.get_last_delta()) {
            Ok(events) => events,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            },
        };

        for event in events {
            match event {
                Event::Chat(msg) => println!("[chat] {}", msg),
            }
        }

        // Clean up the server after a tick
        client.cleanup();

        // Wait for the next tick
        clock.tick(Duration::from_millis(1000 / FPS));
    }
}
