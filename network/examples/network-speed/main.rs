///run with
/// ```bash
/// (cd network/examples/network-speed && RUST_BACKTRACE=1 cargo run --profile=debuginfo -Z unstable-options -- --trace=error --protocol=tcp --mode=server)
/// (cd network/examples/network-speed && RUST_BACKTRACE=1 cargo run --profile=debuginfo -Z unstable-options -- --trace=error --protocol=tcp --mode=client)
/// ```
use clap::{Arg, Command};
use prometheus::Registry;
use prometheus_hyper::Server;
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;
use tracing::*;
use tracing_subscriber::EnvFilter;
use veloren_network::{ConnectAddr, ListenAddr, Message, Network, Pid, Promises};

#[derive(Serialize, Deserialize, Debug)]
enum Msg {
    Ping { id: u64, data: Vec<u8> },
    Pong { id: u64, data: Vec<u8> },
}

/// This utility tests the speed of veloren network by creating a client that
/// opens a stream and pipes as many messages through it as possible.
fn main() {
    let matches = Command::new("Veloren Speed Test Utility")
        .version("0.1.0")
        .author("Marcel Märtens <marcel.cochem@googlemail.com>")
        .about("Runs speedtests regarding different parameter to benchmark veloren-network")
        .arg(
            Arg::new("mode")
                .short('m')
                .long("mode")
                .takes_value(true)
                .possible_values(["server", "client", "both"])
                .default_value("both")
                .help(
                    "choose whether you want to start the server or client or both needed for \
                     this program",
                ),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .takes_value(true)
                .default_value("52000")
                .help("port to listen on"),
        )
        .arg(
            Arg::new("ip")
                .long("ip")
                .takes_value(true)
                .default_value("127.0.0.1")
                .help("ip to listen and connect to"),
        )
        .arg(
            Arg::new("protocol")
                .long("protocol")
                .takes_value(true)
                .default_value("tcp")
                .possible_values(["tcp", "udp", "mpsc"])
                .help(
                    "underlying protocol used for this test, mpsc can only combined with mode=both",
                ),
        )
        .arg(
            Arg::new("trace")
                .short('t')
                .long("trace")
                .takes_value(true)
                .default_value("warn")
                .possible_values(["trace", "debug", "info", "warn", "error"])
                .help("set trace level, not this has a performance impact!"),
        )
        .get_matches();

    let trace = matches.value_of("trace").unwrap();
    let filter = EnvFilter::from_default_env()
        .add_directive(trace.parse().unwrap())
        .add_directive("network_speed=debug".parse().unwrap())
        .add_directive("veloren_network::participant=trace".parse().unwrap())
        .add_directive("veloren_network::protocol=trace".parse().unwrap())
        .add_directive("veloren_network::scheduler=trace".parse().unwrap())
        .add_directive("veloren_network::api=trace".parse().unwrap())
        /*
    .add_directive("veloren_network::participant=debug".parse().unwrap()).add_directive("veloren_network::api=debug".parse().unwrap())*/;
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::ERROR)
        .with_env_filter(filter)
        .init();

    let port: u16 = matches.value_of("port").unwrap().parse().unwrap();
    let ip: &str = matches.value_of("ip").unwrap();
    let addresses = match matches.value_of("protocol") {
        Some("tcp") => (
            ListenAddr::Tcp(format!("{}:{}", ip, port).parse().unwrap()),
            ConnectAddr::Tcp(format!("{}:{}", ip, port).parse().unwrap()),
        ),
        Some("udp") => (
            ListenAddr::Udp(format!("{}:{}", ip, port).parse().unwrap()),
            ConnectAddr::Udp(format!("{}:{}", ip, port).parse().unwrap()),
        ),
        _ => panic!("invalid mode, run --help!"),
    };

    let mut background = None;
    let runtime = Arc::new(Runtime::new().unwrap());
    match matches.value_of("mode") {
        Some("server") => server(addresses.0, Arc::clone(&runtime)),
        Some("client") => client(addresses.1, Arc::clone(&runtime)),
        Some("both") => {
            let s = addresses.0;
            let runtime2 = Arc::clone(&runtime);
            background = Some(thread::spawn(|| server(s, runtime2)));
            thread::sleep(Duration::from_millis(200)); //start client after server
            client(addresses.1, Arc::clone(&runtime));
        },
        _ => panic!("Invalid mode, run --help!"),
    };
    if let Some(background) = background {
        background.join().unwrap();
    }
}

fn server(address: ListenAddr, runtime: Arc<Runtime>) {
    let registry = Arc::new(Registry::new());
    let mut server = Network::new_with_registry(Pid::new(), &runtime, &registry);
    runtime.spawn(Server::run(
        Arc::clone(&registry),
        SocketAddr::from(([0; 4], 59112)),
        futures_util::future::pending(),
    ));
    runtime.block_on(server.listen(address)).unwrap();

    loop {
        info!("----");
        info!("Waiting for participant to connect");
        let mut p1 = runtime.block_on(server.connected()).unwrap(); //remote representation of p1
        let mut s1 = runtime.block_on(p1.opened()).unwrap(); //remote representation of s1
        runtime.block_on(async {
            let mut last = Instant::now();
            let mut id = 0u64;
            while let Ok(_msg) = s1.recv_raw().await {
                id += 1;
                if id.rem_euclid(1000000) == 0 {
                    let new = Instant::now();
                    let diff = new.duration_since(last);
                    last = new;
                    println!("Recv 1.000.000 took {}", diff.as_millis());
                }
            }
            info!("Other stream was closed");
        });
    }
}

fn client(address: ConnectAddr, runtime: Arc<Runtime>) {
    let registry = Arc::new(Registry::new());
    let client = Network::new_with_registry(Pid::new(), &runtime, &registry);
    runtime.spawn(Server::run(
        Arc::clone(&registry),
        SocketAddr::from(([0; 4], 59111)),
        futures_util::future::pending(),
    ));

    let p1 = runtime.block_on(client.connect(address)).unwrap(); //remote representation of p1
    let s1 = runtime
        .block_on(p1.open(4, Promises::ORDERED | Promises::CONSISTENCY, 0))
        .unwrap(); //remote representation of s1
    let mut last = Instant::now();
    let mut id = 0u64;
    let raw_msg = Message::serialize(
        &Msg::Ping {
            id,
            data: vec![0; 1000],
        },
        s1.params(),
    );
    loop {
        s1.send_raw(&raw_msg).unwrap();
        id += 1;
        if id.rem_euclid(1000000) == 0 {
            let new = Instant::now();
            let diff = new.duration_since(last);
            last = new;
            println!("Send 1.000.000 took {}", diff.as_millis());
        }
        if id > 2000000 {
            println!("Stop");
            thread::sleep(Duration::from_millis(2000));
            break;
        }
    }
    drop(s1);
    thread::sleep(Duration::from_millis(2000));
    info!("Closing participant");
    runtime.block_on(p1.disconnect()).unwrap();
    thread::sleep(Duration::from_millis(2000));
    info!("DROPPING! client");
    drop(client);
    thread::sleep(Duration::from_millis(2000));
}
