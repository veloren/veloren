use crate::tuilog::TuiLog;
use termcolor::{ColorChoice, StandardStream};
use tracing::Level;
use tracing_subscriber::{filter::LevelFilter, EnvFilter, FmtSubscriber};
#[cfg(feature = "tracy")]
use tracing_subscriber::{layer::SubscriberExt, prelude::*};

const RUST_LOG_ENV: &str = "RUST_LOG";

lazy_static::lazy_static! {
    pub static ref LOG: TuiLog<'static> = TuiLog::default();
}

pub fn init(basic: bool) {
    // Init logging
    let base_exceptions = |env: EnvFilter| {
        env.add_directive("veloren_world::sim=info".parse().unwrap())
            .add_directive("veloren_world::civ=info".parse().unwrap())
            .add_directive("uvth=warn".parse().unwrap())
            .add_directive("hyper=info".parse().unwrap())
            .add_directive("prometheus_hyper=info".parse().unwrap())
            .add_directive("mio::pool=info".parse().unwrap())
            .add_directive("mio::sys::windows=debug".parse().unwrap())
            .add_directive("veloren_network_protocol=info".parse().unwrap())
            .add_directive(
                "veloren_server::persistence::character=info"
                    .parse()
                    .unwrap(),
            )
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
            subscriber
                .with_writer(|| StandardStream::stdout(ColorChoice::Auto))
                .init();
        } else {
            subscriber.with_writer(|| LOG.clone()).init();
        }
    }
}
