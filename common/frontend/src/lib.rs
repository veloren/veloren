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
/// more complex tracing can be done by concatenating with a `,` as seperator:
///  - warn for `prometheus_hyper`, `dot_vox`, `gfx_device_gl::factory,
///    `gfx_device_gl::shade` trace for `veloren_voxygen`, info for everything
///    else
/// `RUST_LOG="prometheus_hyper=warn,dot_vox::parser=warn,gfx_device_gl::
/// factory=warn,gfx_device_gl::shade=warn,veloren_voxygen=trace,info"`
///
/// By default a few directives are set to `warn` by default, until explicitly
/// overwritten! e.g. `RUST_LOG="gfx_device_gl=debug"`
pub fn init<W2>(log_path_file: Option<(&Path, &str)>, terminal: W2) -> Vec<impl Drop>
where
    W2: MakeWriter + 'static,
    <W2 as MakeWriter>::Writer: Send + Sync,
{
    // To hold the guards that we create, they will cause the logs to be
    // flushed when they're dropped.
    let mut _guards: Vec<WorkerGuard> = vec![];

    // We will do lower logging than the default (INFO) by INCLUSION. This
    // means that if you need lower level logging for a specific module, then
    // put it in the environment in the correct format i.e. DEBUG logging for
    // this crate would be veloren_voxygen=debug.
    let base_exceptions = |env: EnvFilter| {
        env.add_directive("dot_vox::parser=warn".parse().unwrap())
            .add_directive("gfx_device_gl=warn".parse().unwrap())
            .add_directive("veloren_common::trade=info".parse().unwrap())
            .add_directive("veloren_world::sim=info".parse().unwrap())
            .add_directive("veloren_world::civ=info".parse().unwrap())
            .add_directive("hyper=info".parse().unwrap())
            .add_directive("prometheus_hyper=info".parse().unwrap())
            .add_directive("mio::pool=info".parse().unwrap())
            .add_directive("mio::sys::windows=info".parse().unwrap())
            .add_directive("h2=info".parse().unwrap())
            .add_directive("tokio_util=info".parse().unwrap())
            .add_directive("rustls=info".parse().unwrap())
            .add_directive("veloren_network_protocol=info".parse().unwrap())
            .add_directive("quinn_proto::connection=info".parse().unwrap())
            .add_directive(
                "veloren_server::persistence::character=info"
                    .parse()
                    .unwrap(),
            )
            .add_directive("veloren_server::settings=info".parse().unwrap())
            .add_directive(LevelFilter::INFO.into())
    };

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
        let (non_blocking, _stdio_guard) = tracing_appender::non_blocking(terminal.make_writer());
        _guards.push(_stdio_guard);
        registry.with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
    };

    // Try to create the log file's parent folders.
    #[cfg(not(feature = "tracy"))]
    if let Some((path, file)) = log_path_file {
        match fs::create_dir_all(path) {
            Ok(_) => {
                let file_appender = tracing_appender::rolling::daily(path, file);
                let (non_blocking_file, _file_guard) =
                    tracing_appender::non_blocking(file_appender);
                _guards.push(_file_guard);
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
    _guards
}

pub fn init_stdout(log_path_file: Option<(&Path, &str)>) -> Vec<impl Drop> {
    init(log_path_file, || StandardStream::stdout(ColorChoice::Auto))
}
