use lazy_static::*;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU16, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio::runtime::Runtime;
use tracing::*;
use tracing_subscriber::EnvFilter;
use veloren_network::{Network, Participant, Pid, Promises, ProtocolAddr, Stream};

#[allow(dead_code)]
pub fn setup(tracing: bool, sleep: u64) -> (u64, u64) {
    if sleep > 0 {
        thread::sleep(Duration::from_millis(sleep));
    }

    let _subscriber = if tracing {
        let filter = EnvFilter::from_default_env()
            .add_directive("trace".parse().unwrap())
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

#[allow(dead_code)]
pub fn network_participant_stream(
    addr: ProtocolAddr,
) -> (
    Arc<Runtime>,
    Network,
    Participant,
    Stream,
    Network,
    Participant,
    Stream,
) {
    let runtime = Arc::new(Runtime::new().unwrap());
    let (n_a, p1_a, s1_a, n_b, p1_b, s1_b) = runtime.block_on(async {
        let n_a = Network::new(Pid::fake(0), &runtime);
        let n_b = Network::new(Pid::fake(1), &runtime);

        n_a.listen(addr.clone()).await.unwrap();
        let p1_b = n_b.connect(addr).await.unwrap();
        let p1_a = n_a.connected().await.unwrap();

        let s1_a = p1_a.open(4, Promises::empty(), 0).await.unwrap();
        let s1_b = p1_b.opened().await.unwrap();

        (n_a, p1_a, s1_a, n_b, p1_b, s1_b)
    });
    (runtime, n_a, p1_a, s1_a, n_b, p1_b, s1_b)
}

#[allow(dead_code)]
pub fn tcp() -> ProtocolAddr {
    lazy_static! {
        static ref PORTS: AtomicU16 = AtomicU16::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    ProtocolAddr::Tcp(SocketAddr::from(([127, 0, 0, 1], port)))
}

#[allow(dead_code)]
pub fn udp() -> ProtocolAddr {
    lazy_static! {
        static ref PORTS: AtomicU16 = AtomicU16::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    ProtocolAddr::Udp(SocketAddr::from(([127, 0, 0, 1], port)))
}

#[allow(dead_code)]
pub fn mpsc() -> ProtocolAddr {
    lazy_static! {
        static ref PORTS: AtomicU64 = AtomicU64::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    ProtocolAddr::Mpsc(port)
}
