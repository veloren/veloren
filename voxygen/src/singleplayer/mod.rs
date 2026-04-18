use common::{clock::Clock, match_some};
use common_base::local_lan_ip;
use crossbeam_channel::{Receiver, Sender, TryRecvError, bounded, unbounded};
use i18n::LocalizationHandle;
use rand::seq::IteratorRandom;
use server::{
    Error as ServerError, Event, Input, Server, ServerInitStage,
    persistence::{DatabaseSettings, SqlLogMode},
    settings::server_description::ServerDescription,
};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::runtime::Runtime;
use tracing::{error, info, trace, warn};

use crate::lan_discovery;

mod singleplayer_world;
pub use singleplayer_world::{SingleplayerWorld, SingleplayerWorlds};

const TPS: u64 = 30;

/// Used to start and stop the background thread running the server
/// when in singleplayer mode.
pub struct Singleplayer {
    _server_thread: JoinHandle<()>,
    stop_server_s: Sender<()>,
    pub receiver: Receiver<Result<(), ServerError>>,
    pub init_stage_receiver: Receiver<ServerInitStage>,
    // Wether the server is stopped or not
    paused: Arc<AtomicBool>,
    /// True when the server is listening on all interfaces (LAN co-op mode)
    /// rather than localhost only.
    pub is_lan: bool,
    /// Signals the LAN discovery broadcaster thread to stop (set on Drop).
    /// For singleplayer-only (non-LAN) servers this is a no-op `AtomicBool`
    /// that starts `false` and is never set, so the broadcaster is never
    /// started and no thread is created.
    stop_broadcast: Arc<AtomicBool>,
}

impl Singleplayer {
    /// Returns wether or not the server is paused
    pub fn is_paused(&self) -> bool { self.paused.load(Ordering::SeqCst) }

    /// Pauses if true is passed and unpauses if false (Does nothing if in that
    /// state already)
    pub fn pause(&self, state: bool) { self.paused.store(state, Ordering::SeqCst); }
}

impl Drop for Singleplayer {
    fn drop(&mut self) {
        // Stop the server tick loop.
        let _ = self.stop_server_s.send(());
        // Stop the LAN discovery broadcaster (if any).
        self.stop_broadcast.store(true, Ordering::Relaxed);
    }
}

#[derive(Default)]
pub enum SingleplayerState {
    #[default]
    None,
    Init(SingleplayerWorlds),
    Running(Singleplayer),
}

impl SingleplayerState {
    pub fn init() -> Self {
        let dir = common_base::userdata_dir();

        Self::Init(SingleplayerWorlds::load(&dir))
    }

    pub fn run(
        &mut self,
        runtime: &Arc<Runtime>,
        selected_language: &String,
        i18n: &LocalizationHandle,
    ) {
        if let Self::Init(worlds) = self {
            let Some(world) = worlds.current() else {
                error!("Failed to get the current world.");
                return;
            };
            let server_data_dir = world.path.clone();

            let mut settings = server::Settings::singleplayer(&server_data_dir);
            let mut editable_settings = server::EditableSettings::singleplayer(&server_data_dir);

            let i18n = i18n.read();
            let motd = ["hud-chat-singleplayer-motd1", "hud-chat-singleplayer-motd2"]
                .iter()
                .choose(&mut rand::rng())
                .expect("Message of the day don't wanna play.");

            editable_settings.server_description.descriptions.insert(
                selected_language.to_string(),
                ServerDescription {
                    motd: i18n.get_msg(motd).to_string(),
                    rules: None,
                },
            );

            let file_opts = if let Some(gen_opts) = &world.gen_opts
                && !world.is_generated
            {
                server::FileOpts::Save(world.map_path.clone(), gen_opts.clone())
            } else {
                if !world.is_generated && world.gen_opts.is_none() {
                    world.copy_default_world();
                }
                server::FileOpts::Load(world.map_path.clone())
            };

            settings.map_file = Some(file_opts);
            settings.world_seed = world.seed;
            settings.day_length = world.day_length;

            let (stop_server_s, stop_server_r) = unbounded();

            let (server_stage_tx, server_stage_rx) = unbounded();

            // Create server

            // Relative to data_dir
            const PERSISTENCE_DB_DIR: &str = "saves";

            let database_settings = DatabaseSettings {
                db_dir: server_data_dir.join(PERSISTENCE_DB_DIR),
                sql_log_mode: SqlLogMode::Disabled, /* Voxygen doesn't take in command-line
                                                     * arguments
                                                     * so SQL logging can't be enabled for
                                                     * singleplayer without changing this line
                                                     * manually */
            };

            let paused = Arc::new(AtomicBool::new(false));
            let paused1 = Arc::clone(&paused);

            let (result_sender, result_receiver) = bounded(1);

            let builder = thread::Builder::new().name("singleplayer-server-thread".into());
            let runtime = Arc::clone(runtime);
            let thread = builder
                .spawn(move || {
                    trace!("starting singleplayer server thread");

                    let (server, init_result) = match Server::new(
                        settings,
                        editable_settings,
                        database_settings,
                        &server_data_dir,
                        &|init_stage| {
                            let _ = server_stage_tx.send(init_stage);
                        },
                        runtime,
                    ) {
                        Ok(server) => (Some(server), Ok(())),
                        Err(err) => (None, Err(err)),
                    };

                    match (result_sender.send(init_result), server) {
                        (Err(e), _) => warn!(
                            ?e,
                            "Failed to send singleplayer server initialization result. Most \
                             likely the channel was closed by cancelling server creation. \
                             Stopping Server"
                        ),
                        (Ok(()), None) => (),
                        (Ok(()), Some(server)) => run_server(
                            server,
                            stop_server_r,
                            paused1,
                            // Singleplayer doesn't broadcast; use a dummy counter.
                            Arc::new(AtomicU8::new(0)),
                        ),
                    }

                    trace!("ending singleplayer server thread");
                })
                .unwrap();

            *self = SingleplayerState::Running(Singleplayer {
                _server_thread: thread,
                stop_server_s,
                init_stage_receiver: server_stage_rx,
                receiver: result_receiver,
                paused,
                is_lan: false,
                stop_broadcast: Default::default(),
            });
        } else {
            error!("SingleplayerState::run was called, but singleplayer is already running!");
        }
    }

    /// Start a LAN co-op server that listens on all network interfaces so
    /// other players on the local network can join.
    pub fn run_lan_coop(
        &mut self,
        runtime: &Arc<Runtime>,
        selected_language: &String,
        i18n: &LocalizationHandle,
    ) {
        if let Self::Init(worlds) = self {
            let Some(world) = worlds.current() else {
                error!("Failed to get the current world.");
                return;
            };
            let server_data_dir = world.path.clone();
            let world_name = world.name.clone();
            let world_max_players = world.max_players;

            let mut settings = server::Settings::lan_coop(&server_data_dir);
            let mut editable_settings = server::EditableSettings::lan_coop(&server_data_dir);

            let i18n = i18n.read();
            let motd = ["hud-chat-singleplayer-motd1", "hud-chat-singleplayer-motd2"]
                .iter()
                .choose(&mut rand::rng())
                .expect("Message of the day don't wanna play.");

            editable_settings.server_description.descriptions.insert(
                selected_language.to_string(),
                ServerDescription {
                    motd: i18n.get_msg(motd).to_string(),
                    rules: None,
                },
            );

            let file_opts = if let Some(gen_opts) = &world.gen_opts
                && !world.is_generated
            {
                server::FileOpts::Save(world.map_path.clone(), gen_opts.clone())
            } else {
                if !world.is_generated && world.gen_opts.is_none() {
                    world.copy_default_world();
                }
                server::FileOpts::Load(world.map_path.clone())
            };

            settings.map_file = Some(file_opts);
            settings.world_seed = world.seed;
            settings.day_length = world.day_length;
            settings.server_name = world_name.clone();
            settings.max_players = world_max_players;

            let (stop_server_s, stop_server_r) = unbounded();
            let (server_stage_tx, server_stage_rx) = unbounded();

            const PERSISTENCE_DB_DIR: &str = "saves";
            let database_settings = DatabaseSettings {
                db_dir: server_data_dir.join(PERSISTENCE_DB_DIR),
                sql_log_mode: SqlLogMode::Disabled,
            };

            let paused = Arc::new(AtomicBool::new(false));
            let paused1 = Arc::clone(&paused);

            // Shared player count — updated by run_server each tick, read by the
            // LAN broadcaster so that the discovery packet stays live.
            let broadcast_player_count = Arc::new(AtomicU8::new(0));
            let broadcast_player_count1 = Arc::clone(&broadcast_player_count);

            let (result_sender, result_receiver) = bounded(1);

            let builder = thread::Builder::new().name("lan-coop-server-thread".into());
            let runtime = Arc::clone(runtime);

            // Log LAN address so the host can share it with other players.
            match local_lan_ip() {
                Some(ip) => info!(
                    "LAN co-op server starting. Guests can connect to {}:{} (no account required)",
                    ip,
                    server::settings::LAN_COOP_PORT
                ),
                None => info!(
                    "LAN co-op server starting on port {} (could not detect LAN IP automatically)",
                    server::settings::LAN_COOP_PORT
                ),
            }

            let thread = builder
                .spawn(move || {
                    trace!("starting LAN co-op server thread");

                    let (server, init_result) = match Server::new(
                        settings,
                        editable_settings,
                        database_settings,
                        &server_data_dir,
                        &|init_stage| {
                            let _ = server_stage_tx.send(init_stage);
                        },
                        runtime,
                    ) {
                        Ok(server) => (Some(server), Ok(())),
                        Err(err) => (None, Err(err)),
                    };

                    match (result_sender.send(init_result), server) {
                        (Err(e), _) => warn!(
                            ?e,
                            "Failed to send LAN co-op server initialization result."
                        ),
                        (Ok(()), None) => (),
                        (Ok(()), Some(server)) => {
                            run_server(server, stop_server_r, paused1, broadcast_player_count1)
                        },
                    }

                    trace!("ending LAN co-op server thread");
                })
                .unwrap();

            *self = SingleplayerState::Running(Singleplayer {
                _server_thread: thread,
                stop_server_s,
                init_stage_receiver: server_stage_rx,
                receiver: result_receiver,
                paused,
                is_lan: true,
                stop_broadcast: {
                    let stop = Arc::new(AtomicBool::new(false));
                    let player_cap = world_max_players.min(u8::MAX as u16) as u8;
                    lan_discovery::start_broadcaster(
                        server::settings::LAN_COOP_PORT,
                        world_name,
                        player_cap,
                        broadcast_player_count,
                        Arc::clone(&stop),
                    );
                    stop
                },
            });
        } else {
            error!("SingleplayerState::run_lan_coop called, but server is already running!");
        }
    }

    pub fn as_running(&self) -> Option<&Singleplayer> {
        match_some!(self, SingleplayerState::Running(s) => s)
    }

    pub fn as_init(&self) -> Option<&SingleplayerWorlds> {
        match_some!(self, SingleplayerState::Init(s) => s)
    }

    pub fn is_running(&self) -> bool { matches!(self, SingleplayerState::Running(_)) }
}

fn run_server(
    mut server: Server,
    stop_server_r: Receiver<()>,
    paused: Arc<AtomicBool>,
    broadcast_player_count: Arc<AtomicU8>,
) {
    info!("Starting server-cli...");

    // Set up an fps clock
    let mut clock = Clock::new(Duration::from_secs_f64(1.0 / TPS as f64));

    loop {
        // Check any event such as stopping and pausing
        match stop_server_r.try_recv() {
            Ok(()) => break,
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => (),
        }

        // Wait for the next tick.
        clock.tick();

        // Skip updating the server if it's paused
        if paused.load(Ordering::SeqCst) && server.number_of_players() < 2 {
            continue;
        } else if server.number_of_players() > 1 {
            paused.store(false, Ordering::SeqCst);
        }

        let events = server
            .tick(Input::default(), clock.dt())
            .expect("Failed to tick server!");

        // Keep the LAN discovery broadcast up-to-date with the live player count.
        // number_of_players() is always ≥ 0; clamp to u8::MAX for the wire format.
        let count = server.number_of_players().min(u8::MAX as i64) as u8;
        broadcast_player_count.store(count, Ordering::Relaxed);

        for event in events {
            match event {
                Event::ClientConnected { .. } => info!("Client connected!"),
                Event::ClientDisconnected { .. } => info!("Client disconnected!"),
                Event::Chat { entity: _, msg } => info!("[Client] {}", msg),
            }
        }

        // Clean up the server after a tick.
        server.cleanup();
    }
}
