#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(bool_to_option)]

mod logging;
mod shutdown_coordinator;
mod tui_runner;
mod tuilog;

use crate::{
    shutdown_coordinator::ShutdownCoordinator,
    tui_runner::{Message, Tui},
};
use clap::{App, Arg};
use common::clock::Clock;
use server::{DataDir, Event, Input, Server, ServerSettings};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use signal_hook::SIGUSR1;
use std::{
    io,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Duration,
};
use tracing::info;

const TPS: u64 = 30;

fn main() -> io::Result<()> {
    let matches = App::new("Veloren server cli")
        .version(common::util::DISPLAY_VERSION_LONG.as_str())
        .author("The veloren devs <https://gitlab.com/veloren/veloren>")
        .about("The veloren server cli provides an easy to use interface to start a veloren server")
        .args(&[
            Arg::with_name("basic")
                .short("b")
                .long("basic")
                .help("Disables the tui")
                .takes_value(false),
            Arg::with_name("interactive")
                .short("i")
                .long("interactive")
                .help("Enables command input for basic mode")
                .takes_value(false),
            Arg::with_name("no-auth")
                .long("no-auth")
                .help("Runs without auth enabled"),
        ])
        .get_matches();

    let basic = matches.is_present("basic");
    let interactive = matches.is_present("interactive");
    let no_auth = matches.is_present("no-auth");

    let sigusr1_signal = Arc::new(AtomicBool::new(false));

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let _ = signal_hook::flag::register(SIGUSR1, Arc::clone(&sigusr1_signal));

    logging::init(basic);

    // Panic hook to ensure that console mode is set back correctly if in non-basic
    // mode
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        Tui::shutdown(basic);
        hook(info);
    }));

    let tui = (!basic || interactive).then(|| Tui::run(basic));

    info!("Starting server...");

    // Set up an fps clock
    let mut clock = Clock::start();

    // Determine folder to save server data in
    let server_data_dir = DataDir::from({
        let mut path = common::userdata_dir_workspace!();
        path.push(server::DEFAULT_DATA_DIR_NAME);
        path
    });

    // Load settings
    let mut settings = ServerSettings::load(server_data_dir.as_ref());

    if no_auth {
        settings.auth_server_address = None;
    }

    let server_port = &settings.gameserver_address.port();
    let metrics_port = &settings.metrics_address.port();
    // Create server
    let mut server =
        Server::new(settings, server_data_dir).expect("Failed to create server instance!");

    info!(
        ?server_port,
        ?metrics_port,
        "Server is ready to accept connections."
    );

    let mut shutdown_coordinator = ShutdownCoordinator::new(Arc::clone(&sigusr1_signal));

    loop {
        // Terminate the server if instructed to do so by the shutdown coordinator
        if shutdown_coordinator.check(&mut server) {
            break;
        }

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
        #[cfg(feature = "tracy")]
        common::util::tracy_client::finish_continuous_frame!();

        if let Some(tui) = tui.as_ref() {
            match tui.msg_r.try_recv() {
                Ok(msg) => match msg {
                    Message::AbortShutdown => shutdown_coordinator.abort_shutdown(&mut server),
                    Message::Shutdown { grace_period } => {
                        // TODO: The TUI parser doesn't support quoted strings so it is not
                        // currently possible to provide a shutdown reason
                        // from the console.
                        let message = "The server is shutting down".to_owned();
                        shutdown_coordinator.initiate_shutdown(&mut server, grace_period, message);
                    },
                    Message::Quit => {
                        info!("Closing the server");
                        break;
                    },
                },
                Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => {},
            }
        }

        // Wait for the next tick.
        clock.tick(Duration::from_millis(1000 / TPS));
    }

    Ok(())
}
