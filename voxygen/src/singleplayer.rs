use common::clock::Clock;
use crossbeam::channel::{bounded, unbounded, Receiver, Sender, TryRecvError};
use server::{Error as ServerError, Event, Input, Server};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, trace, warn};

const TPS: u64 = 30;

/// Used to start and stop the background thread running the server
/// when in singleplayer mode.
pub struct Singleplayer {
    _server_thread: JoinHandle<()>,
    stop_server_s: Sender<()>,
    pub receiver: Receiver<Result<Arc<Runtime>, ServerError>>,
    // Wether the server is stopped or not
    paused: Arc<AtomicBool>,
    // Settings that the server was started with
    settings: server::Settings,
}

impl Singleplayer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (stop_server_s, stop_server_r) = unbounded();

        // Determine folder to save server data in
        let server_data_dir = {
            let mut path = common_base::userdata_dir_workspace!();
            path.push("singleplayer");
            path
        };

        // Copy saves from old folder if they don't exist in the new location
        (|| {
            let new_path = server_data_dir.join("saves");
            if new_path.exists() {
                return;
            }

            let working_dir = std::path::PathBuf::from("saves");
            let config_dir = directories_next::ProjectDirs::from("net", "veloren", "voxygen")
                .expect("System's $HOME directory path not found!")
                .config_dir()
                .join("saves");
            let old_path = if working_dir.exists() {
                working_dir
            } else if config_dir.exists() {
                config_dir
            } else {
                return;
            };

            info!(
                "Saves folder doesn't exist, but there is one in the old saves location, copying \
                 it to the new location"
            );
            if let Some(parent) = new_path.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    error!(?e, "Could not create folder to hold saves folder.");
                    return;
                }
            }

            if let Err(e) = copy_dir::copy_dir(old_path, new_path) {
                error!(?e, "Failed to copy saves from the old location");
            }
        })();

        // Create server
        let settings = server::Settings::singleplayer(&server_data_dir);
        let editable_settings = server::EditableSettings::singleplayer(&server_data_dir);

        let cores = num_cpus::get();
        debug!("Creating a new runtime for server");
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .worker_threads(if cores > 4 { cores - 1 } else { cores })
                .thread_name_fn(|| {
                    static ATOMIC_ID: AtomicUsize = AtomicUsize::new(0);
                    let id = ATOMIC_ID.fetch_add(1, Ordering::SeqCst);
                    format!("tokio-sp-{}", id)
                })
                .build()
                .unwrap(),
        );

        let settings2 = settings.clone();

        let paused = Arc::new(AtomicBool::new(false));
        let paused1 = Arc::clone(&paused);

        let (result_sender, result_receiver) = bounded(1);

        let builder = thread::Builder::new().name("singleplayer-server-thread".into());
        let thread = builder
            .spawn(move || {
                trace!("starting singleplayer server thread");
                let mut server = None;
                if let Err(e) = result_sender.send(
                    match Server::new(
                        settings2,
                        editable_settings,
                        &server_data_dir,
                        Arc::clone(&runtime),
                    ) {
                        Ok(s) => {
                            server = Some(s);
                            Ok(runtime)
                        },
                        Err(e) => Err(e),
                    },
                ) {
                    warn!(
                        ?e,
                        "Failed to send singleplayer server initialization result. Most likely \
                         the channel was closed by cancelling server creation. Stopping Server"
                    );
                    return;
                };

                let server = match server {
                    Some(s) => s,
                    None => return,
                };

                run_server(server, stop_server_r, paused1);
                trace!("ending singleplayer server thread");
            })
            .unwrap();

        Singleplayer {
            _server_thread: thread,
            stop_server_s,
            receiver: result_receiver,
            paused,
            settings,
        }
    }

    /// Returns reference to the settings the server was started with
    pub fn settings(&self) -> &server::Settings { &self.settings }

    /// Returns wether or not the server is paused
    pub fn is_paused(&self) -> bool { self.paused.load(Ordering::SeqCst) }

    /// Pauses if true is passed and unpauses if false (Does nothing if in that
    /// state already)
    pub fn pause(&self, state: bool) { self.paused.store(state, Ordering::SeqCst); }
}

impl Drop for Singleplayer {
    fn drop(&mut self) {
        // Ignore the result
        let _ = self.stop_server_s.send(());
    }
}

fn run_server(mut server: Server, stop_server_r: Receiver<()>, paused: Arc<AtomicBool>) {
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
