use lazy_static::*;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tracing::*;
use tracing_subscriber::EnvFilter;
use uvth::ThreadPoolBuilder;
use veloren_network::{Address, Network, Participant, Pid, Stream, PROMISES_NONE};

pub fn setup(tracing: bool, mut sleep: u64) -> (u64, u64) {
    if tracing {
        sleep += 1000
    }
    if sleep > 0 {
        thread::sleep(Duration::from_millis(sleep));
    }

    let _subscriber = if tracing {
        let filter = EnvFilter::from_default_env()
            .add_directive("trace".parse().unwrap())
            .add_directive("async_std::task::block_on=warn".parse().unwrap())
            .add_directive("veloren_network::tests=trace".parse().unwrap())
            .add_directive("veloren_network::controller=trace".parse().unwrap())
            .add_directive("veloren_network::channel=trace".parse().unwrap())
            .add_directive("veloren_network::message=trace".parse().unwrap())
            .add_directive("veloren_network::metrics=trace".parse().unwrap())
            .add_directive("veloren_network::types=trace".parse().unwrap());

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

pub async fn network_participant_stream(
    addr: Address,
) -> (
    Network,
    Arc<Participant>,
    Stream,
    Network,
    Arc<Participant>,
    Stream,
) {
    let pool = ThreadPoolBuilder::new().num_threads(2).build();
    let n_a = Network::new(Pid::fake(1), &pool);
    let n_b = Network::new(Pid::fake(2), &pool);

    n_a.listen(addr.clone()).await.unwrap();
    let p1_b = n_b.connect(addr).await.unwrap();
    let p1_a = n_a.connected().await.unwrap();

    let s1_a = p1_a.open(10, PROMISES_NONE).await.unwrap();
    let s1_b = p1_b.opened().await.unwrap();

    (n_a, p1_a, s1_a, n_b, p1_b, s1_b)
}

pub fn tcp() -> veloren_network::Address {
    lazy_static! {
        static ref PORTS: AtomicU16 = AtomicU16::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    veloren_network::Address::Tcp(SocketAddr::from(([127, 0, 0, 1], port)))
}

pub fn udp() -> veloren_network::Address {
    lazy_static! {
        static ref PORTS: AtomicU16 = AtomicU16::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    veloren_network::Address::Udp(SocketAddr::from(([127, 0, 0, 1], port)))
}
