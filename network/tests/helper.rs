use lazy_static::*;
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicU16, Ordering},
    thread,
    time::Duration,
};
use tracing::*;
use tracing_subscriber::EnvFilter;

pub fn setup(tracing: bool, mut sleep: u64) -> (u64, u64) {
    if tracing {
        sleep += 1000
    }
    if sleep > 0 {
        thread::sleep(Duration::from_millis(sleep));
    }

    let _subscriber = if tracing {
        let filter = EnvFilter::from_default_env()
            //.add_directive("[worker]=trace".parse().unwrap())
            .add_directive("trace".parse().unwrap())
            .add_directive("veloren_network::tests=trace".parse().unwrap())
            .add_directive("veloren_network::worker=debug".parse().unwrap())
            .add_directive("veloren_network::controller=trace".parse().unwrap())
            .add_directive("veloren_network::channel=trace".parse().unwrap())
            .add_directive("veloren_network::message=trace".parse().unwrap())
            .add_directive("veloren_network::metrics=trace".parse().unwrap())
            .add_directive("veloren_network::types=trace".parse().unwrap())
            .add_directive("veloren_network::mpsc=debug".parse().unwrap())
            .add_directive("veloren_network::udp=debug".parse().unwrap())
            .add_directive("veloren_network::tcp=debug".parse().unwrap());

        Some(
            tracing_subscriber::FmtSubscriber::builder()
            // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
            // will be written to stdout.
            .with_max_level(Level::TRACE)
            .with_env_filter(filter)
            // sets this to be the default, global subscriber for this application.
            .try_init(),
        )
    } else {
        None
    };

    (0, 0)
}

pub fn tcp() -> veloren_network::Address {
    lazy_static! {
        static ref PORTS: AtomicU16 = AtomicU16::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    veloren_network::Address::Tcp(SocketAddr::from(([127, 0, 0, 1], port)))
}
