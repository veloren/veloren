#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]
#![feature(bool_to_option)]

mod admin;
/// `server-cli` interface commands not to be confused with the commands sent
/// from the client to the server
mod cmd;
mod settings;
mod shutdown_coordinator;
mod tui_runner;
mod tuilog;
use crate::{
    cmd::Message, shutdown_coordinator::ShutdownCoordinator, tui_runner::Tui, tuilog::TuiLog,
};
use clap::{App, Arg, SubCommand};
use common::clock::Clock;
use common_base::span;
use core::sync::atomic::{AtomicUsize, Ordering};
use server::{
    persistence::{DatabaseSettings, SqlLogMode},
    Event, Input, Server,
};
use std::{
    io,
    sync::{atomic::AtomicBool, mpsc, Arc},
    time::Duration,
};
use tracing::{info, trace};

lazy_static::lazy_static! {
    pub static ref LOG: TuiLog<'static> = TuiLog::default();
}
const TPS: u64 = 30;

#[allow(clippy::unnecessary_wraps)]
fn main() -> io::Result<()> {
    let matches = App::new("Veloren server cli")
        .version(common::util::DISPLAY_VERSION_LONG.as_str())
        .author("The veloren devs <https://gitlab.com/veloren/veloren>")
        .about("The veloren server cli provides an easy to use interface to start a veloren server")
        .args(&[
            Arg::with_name("basic")
                .short("b")
                .long("basic")
                .help("Disables the tui"),
            Arg::with_name("non-interactive")
                .short("n")
                .long("non-interactive")
                .help("doesn't listen on STDIN. Useful if you want to send the server in background, and your kernels terminal driver will send SIGTTIN to it otherwise. (https://www.gnu.org/savannah-checkouts/gnu/bash/manual/bash.html#Redirections). and you dont want to use `stty -tostop` or `nohub` or `tmux` or `screen` or `<<< \"\\004\"` to the programm. This implies `-b`"),
            Arg::with_name("no-auth")
                .long("no-auth")
                .help("Runs without auth enabled"),
            Arg::with_name("sql-log-mode")
                .long("sql-log-mode")
                .help("Enables SQL logging, valid values are \"trace\" and \"profile\"")
                .possible_values(&["trace", "profile"])
                .takes_value(true)
        ])
        .subcommand(
            SubCommand::with_name("admin")
                .about("Add or remove admins")
                .subcommands(vec![
                    SubCommand::with_name("add").about("Adds an admin").arg(
                        Arg::with_name("username")
                            .help("Name of the admin to add")
                            .required(true),
                    ),
                    SubCommand::with_name("remove")
                        .about("Removes an admin")
                        .arg(
                            Arg::with_name("username")
                                .help("Name of the admin to remove")
                                .required(true),
                        ),
                ]),
        )
        .get_matches();

    let basic = matches.is_present("basic")
        // Default to basic with these subcommands
        || matches
            .subcommand_name()
            .filter(|name| ["admin"].contains(name))
            .is_some();
    let noninteractive = matches.is_present("non-interactive");
    let no_auth = matches.is_present("no-auth");

    let sql_log_mode = match matches.value_of("sql-log-mode") {
        Some("trace") => SqlLogMode::Trace,
        Some("profile") => SqlLogMode::Profile,
        _ => SqlLogMode::Disabled,
    };

    // noninteractive implies basic
    let basic = basic || noninteractive;

    let sigusr1_signal = Arc::new(AtomicBool::new(false));

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let _ = signal_hook::flag::register(signal_hook::consts::SIGUSR1, Arc::clone(&sigusr1_signal));

    let (_guards, _guards2) = if basic {
        (vec![], common_frontend::init_stdout(None))
    } else {
        (common_frontend::init(None, || LOG.clone()), vec![])
    };

    // Load settings
    let settings = settings::Settings::load();

    // Determine folder to save server data in
    let server_data_dir = {
        let mut path = common_base::userdata_dir_workspace!();
        info!("Using userdata folder at {}", path.display());
        path.push(server::DEFAULT_DATA_DIR_NAME);
        path
    };

    // We don't need that many threads in the async pool, at least 2 but generally
    // 25% of all available will do
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads((num_cpus::get() / 4).max(2))
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                format!("tokio-server-{}", id)
            })
            .build()
            .unwrap(),
    );

    // Load server settings
    let mut server_settings = server::Settings::load(&server_data_dir);
    let mut editable_settings = server::EditableSettings::load(&server_data_dir);

    // Relative to data_dir
    const PERSISTENCE_DB_DIR: &str = "saves";

    let database_settings = DatabaseSettings {
        db_dir: server_data_dir.join(PERSISTENCE_DB_DIR),
        sql_log_mode,
    };

    #[allow(clippy::single_match)] // Note: remove this when there are more subcommands
    match matches.subcommand() {
        ("admin", Some(sub_m)) => {
            admin::admin_subcommand(
                runtime,
                sub_m,
                &server_settings,
                &mut editable_settings,
                &server_data_dir,
            );
            return Ok(());
        },
        _ => {},
    }

    // Panic hook to ensure that console mode is set back correctly if in non-basic
    // mode
    if !basic {
        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            Tui::shutdown(basic);
            hook(info);
        }));
    }

    let tui = (!basic || !noninteractive).then(|| Tui::run(basic));

    info!("Starting server...");

    if no_auth {
        server_settings.auth_server_address = None;
    }

    let server_port = &server_settings.gameserver_address.port();
    let metrics_port = &server_settings.metrics_address.port();
    // Create server
    let mut server = Server::new(
        server_settings,
        editable_settings,
        database_settings,
        &server_data_dir,
        runtime,
    )
    .expect("Failed to create server instance!");

    info!(
        ?server_port,
        ?metrics_port,
        "Server is ready to accept connections."
    );

    let mut shutdown_coordinator = ShutdownCoordinator::new(Arc::clone(&sigusr1_signal));

    // Set up an fps clock
    let mut clock = Clock::new(Duration::from_secs_f64(1.0 / TPS as f64));
    // Wait for a tick so we don't start with a zero dt

    let mut tick_no = 0u64;
    loop {
        tick_no += 1;
        span!(guard, "work");
        // Terminate the server if instructed to do so by the shutdown coordinator
        if shutdown_coordinator.check(&mut server, &settings) {
            break;
        }

        let events = server
            .tick(Input::default(), clock.dt())
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

        if tick_no.rem_euclid(1000) == 0 {
            trace!(?tick_no, "keepalive")
        }

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
                    Message::AddAdmin(username) => {
                        server.add_admin(&username);
                    },
                    Message::RemoveAdmin(username) => {
                        server.remove_admin(&username);
                    },
                    Message::LoadArea(view_distance) => {
                        #[cfg(feature = "worldgen")]
                        server.create_centered_persister(view_distance);
                    },
                    Message::SetSqlLogMode(sql_log_mode) => {
                        server.set_sql_log_mode(sql_log_mode);
                    },
                    Message::DisconnectAllClients => {
                        server.disconnect_all_clients();
                    },
                },
                Err(mpsc::TryRecvError::Empty) | Err(mpsc::TryRecvError::Disconnected) => {},
            }
        }

        drop(guard);
        // Wait for the next tick.
        clock.tick();
        #[cfg(feature = "tracy")]
        common_base::tracy_client::finish_continuous_frame!();
    }

    Ok(())
}
