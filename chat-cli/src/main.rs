#![deny(unsafe_code)]
#![allow(clippy::option_map_unit_fn)]

use client::{Client, Event};
use common::{clock::Clock, comp};
use std::{io, net::ToSocketAddrs, sync::mpsc, thread, time::Duration};
use tracing::{error, info};

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
    tracing_subscriber::fmt::init();

    info!("Starting chat-cli...");

    // Set up an fps clock.
    let mut clock = Clock::start();

    println!("Enter your username");
    let username = read_input();

    println!("Enter the server address");
    let server_addr = read_input();

    println!("Enter your password");
    let password = read_input();

    // Create a client.
    let mut client = Client::new(
        server_addr
            .to_socket_addrs()
            .expect("Invalid server address")
            .next()
            .unwrap(),
        None,
    )
    .expect("Failed to create client instance");

    println!("Server info: {:?}", client.server_info);

    println!("Players online: {:?}", client.get_players());

    client
        .register(username, password, |provider| {
            provider == "https://auth.veloren.net"
        })
        .unwrap();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        loop {
            let msg = read_input();
            tx.send(msg).unwrap();
        }
    });

    loop {
        for msg in rx.try_iter() {
            client.send_chat(msg)
        }

        let events = match client.tick(
            comp::ControllerInputs::default(),
            clock.get_last_delta(),
            |_| {},
        ) {
            Ok(events) => events,
            Err(err) => {
                error!("Error: {:?}", err);
                break;
            },
        };

        for event in events {
            match event {
                // TODO client is now responsible for formatting the `[{player_name}] {}`
                Event::Chat(m) => println!("{}", m.message),
                Event::Disconnect => {}, // TODO
                Event::DisconnectionNotification(time) => {
                    let message = match time {
                        0 => String::from("Goodbye!"),
                        _ => format!("Connection lost. Kicking in {} seconds", time),
                    };

                    println!("{}", message)
                },
                _ => {},
            }
        }

        // Clean up the server after a tick.
        client.cleanup();

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }
}
