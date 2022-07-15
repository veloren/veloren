#[cfg(not(feature = "tracy"))] use std::fs;
use std::path::Path;

use termcolor::{ColorChoice, StandardStream};
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    filter::LevelFilter, fmt::writer::MakeWriter, prelude::*, registry, EnvFilter,
};

const RUST_LOG_ENV: &str = "RUST_LOG";

/// Initialise tracing and logging for the logs_path.
///
/// This function will attempt to set up both a file and a terminal logger,
/// falling back to just a terminal logger if the file is unable to be created.
///
/// The logging level is by default set to `INFO`, to change this for any
/// particular crate or module you must use the `RUST_LOG` environment
/// variable.
///
/// For example to set this crate's debug level to `TRACE` you would need the
/// following in your environment.
/// `RUST_LOG="veloren_voxygen=trace"`
///
/// more complex tracing can be done by concatenating with a `,` as separator:
///  - warn for `prometheus_hyper`, `dot_vox`, `gfx_device_gl::factory,
///    `gfx_device_gl::shade` trace for `veloren_voxygen`, info for everything
///    else
/// `RUST_LOG="prometheus_hyper=warn,dot_vox::parser=warn,gfx_device_gl::
/// factory=warn,gfx_device_gl::shade=warn,veloren_voxygen=trace,info"`
///
/// By default a few directives are set to `warn` by default, until explicitly
/// overwritten! e.g. `RUST_LOG="gfx_device_gl=debug"`
pub fn init<W2>(log_path_file: Option<(&Path, &str)>, terminal: &'static W2) -> Vec<impl Drop>
where
    W2: MakeWriter<'static> + 'static,
    <W2 as MakeWriter<'static>>::Writer: 'static + Send + Sync,
{
    // To hold the guards that we create, they will cause the logs to be
    // flushed when they're dropped.
    #[cfg(not(feature = "tracy"))]
    let mut guards: Vec<WorkerGuard> = Vec::new();
    #[cfg(feature = "tracy")]
    let guards: Vec<WorkerGuard> = Vec::new();

    // We will do lower logging than the default (INFO) by INCLUSION. This
    // means that if you need lower level logging for a specific module, then
    // put it in the environment in the correct format i.e. DEBUG logging for
    // this crate would be veloren_voxygen=debug.

    let mut filter = EnvFilter::default().add_directive(LevelFilter::INFO.into());

    let default_directives = [
        "dot_vox::parser=warn",
        "veloren_common::trade=info",
        "veloren_world::sim=info",
        "veloren_world::civ=info",
        "veloren_world::site::economy=info",
        "veloren_server::events::entity_manipulation=info",
        "hyper=info",
        "prometheus_hyper=info",
        "mio::pool=info",
        "mio::sys::windows=info",
        "h2=info",
        "tokio_util=info",
        "rustls=info",
        "naga=info",
        "gfx_backend_vulkan=info",
        "wgpu_core=info",
        "wgpu_core::device=warn",
        "wgpu_core::swap_chain=info",
        "veloren_network_protocol=info",
        "quinn_proto::connection=info",
        "veloren_server::persistence::character=info",
        "veloren_server::settings=info",
    ];

    for s in default_directives {
        filter = filter.add_directive(s.parse().unwrap());
    }

    match std::env::var(RUST_LOG_ENV) {
        Ok(env) => {
            for s in env.split(',') {
                match s.parse() {
                    Ok(d) => filter = filter.add_directive(d),
                    Err(err) => eprintln!("WARN ignoring log directive: `{s}`: {err}"),
                }
            }
        },
        Err(std::env::VarError::NotUnicode(os_string)) => {
            eprintln!("WARN ignoring log directives due to non-unicode data: {os_string:?}");
        },
        Err(std::env::VarError::NotPresent) => {},
    };

    let filter = filter; // mutation is done

    let registry = registry();
    #[cfg(not(feature = "tracy"))]
    let mut file_setup = false;
    #[cfg(feature = "tracy")]
    let file_setup = false;
    #[cfg(feature = "tracy")]
    let _terminal = terminal;

    // Create the terminal writer layer.
    #[cfg(feature = "tracy")]
    let registry = registry.with(tracing_tracy::TracyLayer::new().with_stackdepth(0));
    #[cfg(not(feature = "tracy"))]
    let registry = {
        let (non_blocking, stdio_guard) = tracing_appender::non_blocking(terminal.make_writer());
        guards.push(stdio_guard);
        registry.with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
    };

    // Try to create the log file's parent folders.
    #[cfg(not(feature = "tracy"))]
    if let Some((path, file)) = log_path_file {
        match fs::create_dir_all(path) {
            Ok(_) => {
                let file_appender = tracing_appender::rolling::never(path, file); // It is actually rolling daily since the log name is changing daily
                let (non_blocking_file, file_guard) = tracing_appender::non_blocking(file_appender);
                guards.push(file_guard);
                file_setup = true;
                registry
                    .with(tracing_subscriber::fmt::layer().with_writer(non_blocking_file))
                    .with(filter)
                    .init();
            },
            Err(e) => {
                tracing::error!(
                    ?e,
                    "Failed to create log file!. Falling back to terminal logging only.",
                );
                registry.with(filter).init();
            },
        }
    } else {
        registry.with(filter).init();
    }
    #[cfg(feature = "tracy")]
    registry.with(filter).init();

    if file_setup {
        let (path, file) = log_path_file.unwrap();
        info!(?path, ?file, "Setup terminal and file logging.");
    }

    if tracing::level_enabled!(tracing::Level::TRACE) {
        info!("Tracing Level: TRACE");
    } else if tracing::level_enabled!(tracing::Level::DEBUG) {
        info!("Tracing Level: DEBUG");
    };

    // Return the guards
    guards
}

pub fn init_stdout(log_path_file: Option<(&Path, &str)>) -> Vec<impl Drop> {
    init(log_path_file, &|| StandardStream::stdout(ColorChoice::Auto))
}
