#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(bool_to_option)]

mod shutdown_coordinator;
mod tui_runner;
mod tuilog;

#[macro_use] extern crate lazy_static;

use crate::{
    shutdown_coordinator::ShutdownCoordinator,
    tui_runner::{Message, Tui},
    tuilog::TuiLog,
};
use common::clock::Clock;
use server::{Event, Input, Server, ServerSettings};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use signal_hook::SIGUSR1;
use tracing::{info, Level};
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};
#[cfg(feature = "tracy")]
use tracing_subscriber::{layer::SubscriberExt, prelude::*};

use clap::{App, Arg};
use std::{
    io,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Duration,
};

const TPS: u64 = 30;
const RUST_LOG_ENV: &str = "RUST_LOG";

lazy_static! {
    static ref LOG: TuiLog<'static> = TuiLog::default();
}

fn main() -> io::Result<()> {
    let matches = App::new("Veloren server cli")
        .version(common::util::DISPLAY_VERSION_LONG.as_str())
        .author("The veloren devs <https://gitlab.com/veloren/veloren>")
        .about("The veloren server cli provides an easy to use interface to start a veloren server")
        .arg(
            Arg::with_name("basic")
                .short("b")
                .long("basic")
                .help("Disables the tui")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("interactive")
                .short("i")
                .long("interactive")
                .help("Enables command input for basic mode")
                .takes_value(false),
        )
        .get_matches();

    let basic = matches.is_present("basic");
    let interactive = matches.is_present("interactive");

    let sigusr1_signal = Arc::new(AtomicBool::new(false));

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let _ = signal_hook::flag::register(SIGUSR1, Arc::clone(&sigusr1_signal));

    // Init logging
    let base_exceptions = |env: EnvFilter| {
        env.add_directive("veloren_world::sim=info".parse().unwrap())
            .add_directive("veloren_world::civ=info".parse().unwrap())
            .add_directive("uvth=warn".parse().unwrap())
            .add_directive("tiny_http=warn".parse().unwrap())
            .add_directive("mio::sys::windows=debug".parse().unwrap())
            .add_directive(LevelFilter::INFO.into())
    };

    #[cfg(not(feature = "tracy"))]
    let filter = match std::env::var_os(RUST_LOG_ENV).map(|s| s.into_string()) {
        Some(Ok(env)) => {
            let mut filter = base_exceptions(EnvFilter::new(""));
            for s in env.split(',').into_iter() {
                match s.parse() {
                    Ok(d) => filter = filter.add_directive(d),
                    Err(err) => println!("WARN ignoring log directive: `{}`: {}", s, err),
                };
            }
            filter
        },
        _ => base_exceptions(EnvFilter::from_env(RUST_LOG_ENV)),
    };

    #[cfg(feature = "tracy")]
    tracing_subscriber::registry()
        .with(tracing_tracy::TracyLayer::new().with_stackdepth(0))
        .init();

    #[cfg(not(feature = "tracy"))]
    // TODO: when tracing gets per Layer filters re-enable this when the tracy feature is being
    // used (and do the same in voxygen)
    {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::ERROR)
            .with_env_filter(filter);

        if basic {
            subscriber.init();
        } else {
            subscriber.with_writer(|| LOG.clone()).init();
        }
    }

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

    // Load settings
    let settings = ServerSettings::load();
    let server_port = &settings.gameserver_address.port();
    let metrics_port = &settings.metrics_address.port();
    // Create server
    let mut server = Server::new(settings).expect("Failed to create server instance!");

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
