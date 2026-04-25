use common::{
    clock::Clock,
    comp::{ChatType, Player},
    consts::MIN_RECOMMENDED_TOKIO_THREADS,
};
use server::{Event, Input, Server, persistence::DatabaseSettings};
use specs::{Join, WorldExt};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tracing::info;

const TPS: u64 = 30;

/// Commands sent from the GUI to the server thread.
#[derive(Debug)]
pub enum ServerCmd {
    /// Initiate a graceful shutdown after `seconds` with a broadcast message.
    ShutdownGraceful { seconds: u64, reason: String },
    /// Shut down immediately.
    ShutdownImmediate,
    /// Send a message to all connected players.
    BroadcastMessage { msg: String },
    /// Disconnect every connected client.
    DisconnectAll,
    /// Add an admin role.
    AdminAdd { username: String, role: common::comp::AdminRole },
    /// Remove an admin role.
    AdminRemove { username: String },
}

/// Responses sent back to the GUI from the server thread.
#[derive(Debug)]
pub enum ServerEvent {
    /// Updated player list (sent periodically).
    Players(Vec<String>),
    /// Server has fully shut down.
    Stopped,
}

/// Run the server tick loop in a background thread.
///
/// Returns a pair of channels:
///   - `cmd_tx`: GUI → server  (send `ServerCmd`)
///   - `event_rx`: server → GUI (receive `ServerEvent`)
///
/// The thread exits when `stop_flag` is set or a `ShutdownImmediate` command
/// is received.
pub fn run_server_thread(
    server_data_dir: std::path::PathBuf,
    server_settings: server::Settings,
    editable_settings: server::EditableSettings,
    database_settings: DatabaseSettings,
    runtime: Arc<tokio::runtime::Runtime>,
    stop_flag: Arc<AtomicBool>,
) -> (
    mpsc::Sender<ServerCmd>,
    mpsc::Receiver<ServerEvent>,
) {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<ServerCmd>(64);
    let (event_tx, event_rx) = mpsc::channel::<ServerEvent>(64);

    std::thread::Builder::new()
        .name("server-thread".into())
        .spawn(move || {
            let mut server = match Server::new(
                server_settings,
                editable_settings,
                database_settings,
                &server_data_dir,
                &|_| {},
                Arc::clone(&runtime),
            ) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(?e, "Failed to create server instance");
                    let _ = event_tx.blocking_send(ServerEvent::Stopped);
                    return;
                },
            };

            info!("Server started successfully.");

            let mut clock = Clock::new(Duration::from_secs_f64(1.0 / TPS as f64));
            let mut shutdown_at: Option<Instant> = None;
            let mut player_list_ticker = 0u32;

            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    break;
                }

                // Check graceful shutdown timer.
                if let Some(t) = shutdown_at {
                    if Instant::now() >= t {
                        break;
                    }
                }

                // Tick the server.
                let events = match server.tick(Input::default(), clock.dt()) {
                    Ok(e) => e,
                    Err(e) => {
                        tracing::error!(?e, "Server tick failed");
                        break;
                    },
                };

                for event in events {
                    match event {
                        Event::ClientConnected { .. } => info!("Client connected!"),
                        Event::ClientDisconnected { .. } => info!("Client disconnected!"),
                        Event::Chat { entity: _, msg } => info!("[Chat] {}", msg),
                    }
                }

                server.cleanup();

                // Drain commands from the GUI.
                while let Ok(cmd) = cmd_rx.try_recv() {
                    match cmd {
                        ServerCmd::ShutdownImmediate => {
                            stop_flag.store(true, Ordering::Relaxed);
                        },
                        ServerCmd::ShutdownGraceful { seconds, reason } => {
                            let msg = ChatType::Meta
                                .into_plain_msg(format!("{reason} (in {seconds}s)"));
                            use server::state_ext::StateExt;
                            server.state().send_chat(msg, false);
                            shutdown_at =
                                Some(Instant::now() + Duration::from_secs(seconds));
                        },
                        ServerCmd::BroadcastMessage { msg } => {
                            use server::state_ext::StateExt;
                            let chat_msg = ChatType::Meta.into_plain_msg(msg);
                            server.state().send_chat(chat_msg, false);
                        },
                        ServerCmd::DisconnectAll => {
                            server.disconnect_all_clients();
                        },
                        ServerCmd::AdminAdd { username, role } => {
                            server.add_admin(&username, role);
                        },
                        ServerCmd::AdminRemove { username } => {
                            server.remove_admin(&username);
                        },
                    }
                }

                // Periodically emit the player list to the GUI.
                player_list_ticker = player_list_ticker.wrapping_add(1);
                if player_list_ticker % (TPS as u32) == 0 {
                    let players: Vec<String> = server
                        .state()
                        .ecs()
                        .read_storage::<Player>()
                        .join()
                        .map(|p| p.alias.clone())
                        .collect();
                    let _ = event_tx.try_send(ServerEvent::Players(players));
                }

                clock.tick();
            }

            info!("Server shutting down…");
            let _ = event_tx.blocking_send(ServerEvent::Stopped);
        })
        .expect("Failed to spawn server thread");

    (cmd_tx, event_rx)
}

/// Build a Tokio runtime suitable for the server.
pub fn build_runtime() -> Arc<tokio::runtime::Runtime> {
    Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads((num_cpus::get() / 4).max(MIN_RECOMMENDED_TOKIO_THREADS))
            .thread_name("tokio-server")
            .build()
            .expect("Failed to build Tokio runtime"),
    )
}
