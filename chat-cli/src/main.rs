use client::{Client, Event};
use common::{clock::Clock, comp};
use log::{error, info};
use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const TPS: u64 = 10; // Low value is okay, just reading messages.

fn read_input() -> String {
    let mut buffer = String::new();

    io::stdin()
        .read_line(&mut buffer)
        .expect("Failed to read input");

    buffer.trim().to_string()
}

fn main() {
    // Initialize logging.
    pretty_env_logger::init();

    info!("Starting chat-cli...");

    // Set up an fps clock.
    let mut clock = Clock::new();

    println!("Enter your username");
    let mut username = read_input();

    // Create a client.
    let mut client =
        Client::new(([127, 0, 0, 1], 59003), None).expect("Failed to create client instance");

    println!("Server info: {:?}", client.server_info);

    println!("Players online: {:?}", client.get_players());

    client.register(comp::Player::new(username, None));

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || loop {
        let msg = read_input();
        tx.send(msg).unwrap();
    });

    loop {
        for msg in rx.try_iter() {
            client.send_chat(msg)
        }

        let events = match client.tick(comp::Controller::default(), clock.get_last_delta()) {
            Ok(events) => events,
            Err(err) => {
                error!("Error: {:?}", err);
                break;
            }
        };

        for event in events {
            match event {
                Event::Chat(msg) => println!("{}", msg),
                Event::Disconnect => {} // TODO
            }
        }
        // Clean up the server after a tick.
        client.cleanup();

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
