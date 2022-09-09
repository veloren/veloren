use lazy_static::*;
use std::{
    net::{Ipv4Addr, SocketAddr},
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
use veloren_network::{ConnectAddr, ListenAddr, Network, Participant, Pid, Promises, Stream};

// sleep time when only internal rust calculations are done
#[allow(dead_code)]
pub const SLEEP_INTERNAL: Duration = Duration::from_millis(3000);
// sleep time when we interact with the system, e.g. actually send TCP/UDP
// package
#[allow(dead_code)]
pub const SLEEP_EXTERNAL: Duration = Duration::from_millis(5000);

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
    addr: (ListenAddr, ConnectAddr),
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
        let mut n_a = Network::new(Pid::fake(0), &runtime);
        let n_b = Network::new(Pid::fake(1), &runtime);

        n_a.listen(addr.0).await.unwrap();
        let mut p1_b = n_b.connect(addr.1).await.unwrap();
        let p1_a = n_a.connected().await.unwrap();

        let s1_a = p1_a.open(4, Promises::ORDERED, 0).await.unwrap();
        let s1_b = p1_b.opened().await.unwrap();

        (n_a, p1_a, s1_a, n_b, p1_b, s1_b)
    });
    (runtime, n_a, p1_a, s1_a, n_b, p1_b, s1_b)
}

#[allow(dead_code)]
pub fn tcp() -> (ListenAddr, ConnectAddr) {
    lazy_static! {
        static ref PORTS: AtomicU16 = AtomicU16::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    (
        ListenAddr::Tcp(SocketAddr::from((Ipv4Addr::LOCALHOST, port))),
        ConnectAddr::Tcp(SocketAddr::from((Ipv4Addr::LOCALHOST, port))),
    )
}

lazy_static! {
    static ref UDP_PORTS: AtomicU16 = AtomicU16::new(5000);
}

#[allow(dead_code)]
pub fn quic() -> (ListenAddr, ConnectAddr) {
    const LOCALHOST: &str = "localhost";
    let port = UDP_PORTS.fetch_add(1, Ordering::Relaxed);

    trace!("generating self-signed certificate");
    let cert = rcgen::generate_simple_self_signed(vec![LOCALHOST.into()]).unwrap();
    let key = cert.serialize_private_key_der();
    let cert = cert.serialize_der().unwrap();

    let key = rustls::PrivateKey(key);
    let cert = rustls::Certificate(cert);

    let mut root_store = rustls::RootCertStore::empty();
    root_store.add(&cert).expect("cannot add cert to rootstore");

    let server_config = quinn::ServerConfig::with_single_cert(vec![cert], key)
        .expect("Server Config Cert/Key failed");
    let client_config = quinn::ClientConfig::with_root_certificates(root_store);
    use std::net::IpAddr;
    (
        ListenAddr::Quic(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            server_config,
        ),
        ConnectAddr::Quic(
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
            client_config,
            LOCALHOST.to_owned(),
        ),
    )
}

#[allow(dead_code)]
pub fn udp() -> (ListenAddr, ConnectAddr) {
    let port = UDP_PORTS.fetch_add(1, Ordering::Relaxed);
    (
        ListenAddr::Udp(SocketAddr::from(([127, 0, 0, 1], port))),
        ConnectAddr::Udp(SocketAddr::from(([127, 0, 0, 1], port))),
    )
}

#[allow(dead_code)]
pub fn mpsc() -> (ListenAddr, ConnectAddr) {
    lazy_static! {
        static ref PORTS: AtomicU64 = AtomicU64::new(5000);
    }
    let port = PORTS.fetch_add(1, Ordering::Relaxed);
    (ListenAddr::Mpsc(port), ConnectAddr::Mpsc(port))
}
