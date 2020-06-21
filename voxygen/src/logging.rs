use std::fs;

use crate::settings::Settings;

use tracing::{error, info, instrument};
use tracing_subscriber::{filter::LevelFilter, prelude::*, registry, EnvFilter};

const VOXYGEN_LOG_ENV: &str = "VOXYGEN_LOG";

/// Initialise tracing and logging fro the settings.
///
/// This function will attempt to set up both a file and a terminal logger,
/// falling back to just a terminal logger if the file is unable to be created.
///
/// The logging level is by deafult set to `INFO`, to change this for any
/// particular crate or module you must use the `VOXYGEN_LOG` environment
/// variable.
///
/// For example to set this crate's debug level to `TRACE` you would need the
/// following in your environment.
///
/// `VOXYGEN_LOG="veloren_voxygen=trace"`
#[instrument]
pub fn init(settings: &Settings) -> Vec<impl Drop> {
    // To hold the guards that we create, they will cause the logs to be
    // flushed when they're dropped.
    let mut _guards = vec![];

    // We will do lower logging than the default (INFO) by INCLUSION. This
    // means that if you need lower level logging for a specific module, then
    // put it in the environment in the correct format i.e. DEBUG logging for
    // this crate with would be veloren_voxygen=debug.
    let filter = EnvFilter::from_env(VOXYGEN_LOG_ENV)
        .add_directive("dot_vox::parser=warn".parse().unwrap())
        .add_directive("gfx_device_gl::factory=warn".parse().unwrap())
        .add_directive("gfx_device_gl::shade=warn".parse().unwrap())
        .add_directive("uvth=warn".parse().unwrap())
        .add_directive("tiny_http=warn".parse().unwrap())
        .add_directive(LevelFilter::INFO.into());

    // Create the terminal writer layer.
    let (non_blocking, _stdio_guard) = tracing_appender::non_blocking(std::io::stdout());
    _guards.push(_stdio_guard);

    // Try to create the log file's parent folders.
    let log_folders_created = fs::create_dir_all(&settings.log.logs_path);

    match log_folders_created {
        // If the parent folders were created then attach both a terminal and a
        // file writer to the registry and init it.
        Ok(_) => {
            let file_appender =
                tracing_appender::rolling::daily(&settings.log.logs_path, "voxygen.log");
            let (non_blocking_file, _file_guard) = tracing_appender::non_blocking(file_appender);
            _guards.push(_file_guard);
            registry()
                .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
                .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_file))
                .with(filter)
                .init();
            info!("Setup terminal and file logging.");
        },
        // Otherwise just add a terminal writer and init it.
        Err(e) => {
            error!(
                "Failed to create log file! {}. Falling back to terminal logging only.",
                e
            );
            registry()
                .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
                .with(filter)
                .init();
            info!("Setup terminal logging.");
        },
    };

    // Return the guards
    _guards
}
