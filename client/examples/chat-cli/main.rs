#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]

use client_i18n::LocalizationHandle;
use common::{clock::Clock, comp};
use std::{
    io,
    sync::{mpsc, Arc},
    thread,
    time::Duration,
};
use tokio::runtime::Runtime;
use tracing::{error, info};
use veloren_client::{addr::ConnectionArgs, Client, Event};
use voxygen_i18n_helpers::localize_chat_message;

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
    common_frontend::init_stdout(None);

    info!("loading localisation");

    let localisation = LocalizationHandle::load_expect("en");

    info!("Starting chat-cli...");

    // Set up an fps clock.
    let mut clock = Clock::new(Duration::from_secs_f64(1.0 / TPS as f64));

    println!("Enter your username");
    let username = read_input();

    println!("Enter the server address");
    let server_addr = read_input();

    println!("Enter your password");
    let password = read_input();

    let runtime = Arc::new(Runtime::new().unwrap());
    let runtime2 = Arc::clone(&runtime);
    let addr = ConnectionArgs::Tcp {
        prefer_ipv6: false,
        hostname: server_addr,
    };

    // Create a client.
    let mut client = runtime
        .block_on(Client::new(
            addr,
            runtime2,
            &mut None,
            &username,
            &password,
            |provider| provider == "https://auth.veloren.net",
        ))
        .expect("Failed to create client instance");

    println!("Server info: {:?}", client.server_info());

    let mut player_printed = false;

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

        let events = match client.tick(comp::ControllerInputs::default(), clock.dt(), |_| {}) {
            Ok(events) => events,
            Err(err) => {
                error!("Error: {:?}", err);
                break;
            },
        };

        const SHOW_NAME: bool = false;
        for event in events {
            match event {
                Event::Chat(m) => println!(
                    "{}",
                    localize_chat_message(
                        m,
                        |msg| client.lookup_msg_context(msg),
                        &localisation.read(),
                        SHOW_NAME,
                    )
                    .message
                ),
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
        clock.tick();

        if !player_printed {
            println!("Players online: {:?}", client.players().collect::<Vec<_>>());
            player_printed = true;
        }
    }
}
