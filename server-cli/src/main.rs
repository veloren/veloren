#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]

#[cfg(all(
    target_os = "windows",
    not(feature = "hot-agent"),
    not(feature = "hot-site"),
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// `server-cli` interface commands not to be confused with the commands sent
/// from the client to the server
mod cli;
mod settings;
mod shutdown_coordinator;
mod tui_runner;
mod tuilog;
use crate::{
    cli::{Admin, ArgvApp, ArgvCommand, Message, SharedCommand, Shutdown},
    shutdown_coordinator::ShutdownCoordinator,
    tui_runner::Tui,
    tuilog::TuiLog,
};
use common::{clock::Clock, consts::MIN_RECOMMENDED_TOKIO_THREADS};
use common_base::span;
use core::sync::atomic::{AtomicUsize, Ordering};
use server::{persistence::DatabaseSettings, settings::Protocol, Event, Input, Server};
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

fn main() -> io::Result<()> {
    #[cfg(feature = "tracy")]
    common_base::tracy_client::Client::start();

    use clap::Parser;
    let app = ArgvApp::parse();

    let basic = !app.tui || app.command.is_some();
    let noninteractive = app.non_interactive;
    let no_auth = app.no_auth;
    let sql_log_mode = app.sql_log_mode;

    // noninteractive implies basic
    let basic = basic || noninteractive;

    let sigusr1_signal = Arc::new(AtomicBool::new(false));

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let _ = signal_hook::flag::register(signal_hook::consts::SIGUSR1, Arc::clone(&sigusr1_signal));

    let (_guards, _guards2) = if basic {
        (Vec::new(), common_frontend::init_stdout(None))
    } else {
        (common_frontend::init(None, &|| LOG.clone()), Vec::new())
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
    // TODO: evaluate std::thread::available_concurrency as a num_cpus replacement
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads((num_cpus::get() / 4).max(MIN_RECOMMENDED_TOKIO_THREADS))
            .thread_name_fn(|| {
                static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                format!("tokio-server-{}", id)
            })
            .build()
            .unwrap(),
    );

    #[cfg(feature = "hot-agent")]
    {
        agent::init();
    }
    #[cfg(feature = "hot-site")]
    {
        world::init();
    }

    // Load server settings
    let mut server_settings = server::Settings::load(&server_data_dir);
    let mut editable_settings = server::EditableSettings::load(&server_data_dir);

    // Apply no_auth modifier to the settings
    if no_auth {
        server_settings.auth_server_address = None;
    }

    // Relative to data_dir
    const PERSISTENCE_DB_DIR: &str = "saves";

    let database_settings = DatabaseSettings {
        db_dir: server_data_dir.join(PERSISTENCE_DB_DIR),
        sql_log_mode,
    };

    if let Some(command) = app.command {
        return match command {
            ArgvCommand::Shared(SharedCommand::Admin { command }) => {
                let login_provider = server::login_provider::LoginProvider::new(
                    server_settings.auth_server_address,
                    runtime,
                );

                match command {
                    Admin::Add { username, role } => {
                        // FIXME: Currently the UUID can get returned even if the file didn't
                        // change, so this can't be relied on as an error
                        // code; moreover, we do nothing with the UUID
                        // returned in the success case.  Fix the underlying function to return
                        // enough information that we can reliably return an error code.
                        let _ = server::add_admin(
                            &username,
                            role,
                            &login_provider,
                            &mut editable_settings,
                            &server_data_dir,
                        );
                    },
                    Admin::Remove { username } => {
                        // FIXME: Currently the UUID can get returned even if the file didn't
                        // change, so this can't be relied on as an error
                        // code; moreover, we do nothing with the UUID
                        // returned in the success case.  Fix the underlying function to return
                        // enough information that we can reliably return an error code.
                        let _ = server::remove_admin(
                            &username,
                            &login_provider,
                            &mut editable_settings,
                            &server_data_dir,
                        );
                    },
                }
                Ok(())
            },
        };
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

    let tui = (!noninteractive).then(|| Tui::run(basic));

    info!("Starting server...");

    let protocols_and_addresses = server_settings.gameserver_protocols.clone();
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

    // Collect addresses that the server is listening to log.
    let gameserver_addresses = protocols_and_addresses
        .into_iter()
        .map(|protocol| match protocol {
            Protocol::Tcp { address } => ("TCP", address),
            Protocol::Quic {
                address,
                cert_file_path: _,
                key_file_path: _,
            } => ("QUIC", address),
        });

    info!(
        ?metrics_port,
        ?gameserver_addresses,
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
                    Message::Shutdown {
                        command: Shutdown::Cancel,
                    } => shutdown_coordinator.abort_shutdown(&mut server),
                    Message::Shutdown {
                        command: Shutdown::Graceful { seconds, reason },
                    } => {
                        shutdown_coordinator.initiate_shutdown(
                            &mut server,
                            Duration::from_secs(seconds),
                            reason,
                        );
                    },
                    Message::Shutdown {
                        command: Shutdown::Immediate,
                    } => {
                        info!("Closing the server");
                        break;
                    },
                    Message::Shared(SharedCommand::Admin {
                        command: Admin::Add { username, role },
                    }) => {
                        server.add_admin(&username, role);
                    },
                    Message::Shared(SharedCommand::Admin {
                        command: Admin::Remove { username },
                    }) => {
                        server.remove_admin(&username);
                    },
                    Message::LoadArea { view_distance } => {
                        #[cfg(feature = "worldgen")]
                        server.create_centered_persister(view_distance);
                    },
                    Message::SqlLogMode { mode } => {
                        server.set_sql_log_mode(mode);
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
        common_base::tracy_client::frame_mark();
    }

    Ok(())
}
