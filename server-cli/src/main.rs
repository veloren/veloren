#![deny(unsafe_code)]

mod tui_runner;
mod tuilog;

#[macro_use] extern crate lazy_static;

use crate::{
    tui_runner::{Message, Tui},
    tuilog::TuiLog,
};
use common::clock::Clock;
use server::{Event, Input, Server, ServerSettings};
use tracing::{info, Level};
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};

use clap::{App, Arg};
use std::{io, sync::mpsc, time::Duration};

const TPS: u64 = 30;
const RUST_LOG_ENV: &str = "RUST_LOG";

lazy_static! {
    static ref LOG: TuiLog<'static> = TuiLog::default();
}

fn main() -> io::Result<()> {
    let matches = App::new("Veloren server cli")
        .version(
            format!(
                "{}-{}",
                env!("CARGO_PKG_VERSION"),
                common::util::GIT_HASH.to_string()
            )
            .as_str(),
        )
        .author("The veloren devs <https://gitlab.com/veloren/veloren>")
        .about("The veloren server cli provides an easy to use interface to start a veloren server")
        .arg(
            Arg::with_name("basic")
                .short("b")
                .long("basic")
                .help("Disables the tui")
                .takes_value(false),
        )
        .get_matches();

    let basic = matches.is_present("basic");
    let (mut tui, msg_r) = Tui::new();

    // Init logging
    let filter = match std::env::var_os(RUST_LOG_ENV).map(|s| s.into_string()) {
        Some(Ok(env)) => {
            let mut filter = EnvFilter::new("veloren_world::sim=info")
                .add_directive("veloren_world::civ=info".parse().unwrap())
                .add_directive(LevelFilter::INFO.into());
            for s in env.split(',').into_iter() {
                match s.parse() {
                    Ok(d) => filter = filter.add_directive(d),
                    Err(err) => println!("WARN ignoring log directive: `{}`: {}", s, err),
                };
            }
            filter
        },
        _ => EnvFilter::from_env(RUST_LOG_ENV)
            .add_directive("veloren_world::sim=info".parse().unwrap())
            .add_directive("veloren_world::civ=info".parse().unwrap())
            .add_directive(LevelFilter::INFO.into()),
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_env_filter(filter);

    if basic {
        subscriber.init();
    } else {
        subscriber.with_writer(|| LOG.clone()).init();
    }

    tui.run(basic);

    info!("Starting server...");

    // Set up an fps clock
    let mut clock = Clock::start();

    // Load settings
    let settings = ServerSettings::load();
    let server_port = &settings.gameserver_address.port();
    let metrics_port = &settings.metrics_address.port();
    // Create server
    let mut server = Server::new(settings).expect("Failed to create server instance!");

    info!("Server is ready to accept connections.");
    info!(?metrics_port, "starting metrics at port");
    info!(?server_port, "starting server at port");

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

        match msg_r.try_recv() {
            Ok(msg) => match msg {
                Message::Quit => {
                    info!("Closing the server");
                    break;
                },
            },
            Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => {},
        };

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }

    Ok(())
}
