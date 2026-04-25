#![deny(unsafe_code)]
#![deny(clippy::clone_on_ref_ptr)]

#[cfg(all(
    target_os = "windows",
    not(feature = "hot-agent"),
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod app;
mod gui_log;
mod launch_config;
mod server_thread;

use crate::{
    app::ServerApp,
    gui_log::{GuiLog, SharedLog, new_shared_log},
    server_thread::build_runtime,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::AtomicBool,
    },
};
use tracing::info;

lazy_static::lazy_static! {
    /// Shared log buffer — accessible from both the GUI thread and the writer.
    static ref SHARED_LOG: SharedLog = new_shared_log();
    /// The tracing writer; cloned on every log call.
    static ref LOG_WRITER: GuiLog = GuiLog::new(Arc::clone(&SHARED_LOG));
}

fn main() -> eframe::Result<()> {
    // ── Init tracing first so all later messages are captured ────────────
    let _log_guards = common_frontend::init(None, &|| LOG_WRITER.clone());

    // ── Determine data directory ──────────────────────────────────────────
    let server_data_dir: PathBuf = {
        let mut p = common_base::userdata_dir();
        info!("Using userdata folder at {}", p.display());
        p.push(server::DEFAULT_DATA_DIR_NAME);
        p
    };

    // ── Tokio runtime ─────────────────────────────────────────────────────
    let runtime = build_runtime();

    // ── Stop flag (shared between GUI and server thread) ─────────────────
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Clone the shared log Arc so the GUI can read it.
    let shared_log = Arc::clone(&*SHARED_LOG);

    // ── Build the GUI app (server starts lazily on user action) ──────────
    let app = ServerApp::new(
        server_data_dir,
        Arc::clone(&runtime),
        Arc::clone(&stop_flag),
        shared_log,
    );

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Nova-Forge Server")
            .with_inner_size([1024.0, 700.0])
            .with_min_inner_size([640.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Nova-Forge Server",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(app))
        }),
    )
}
