mod metrics;

use clap::{App, Arg};
use futures::executor::block_on;
use network::{Address, Network, Pid, PROMISES_CONSISTENCY, PROMISES_ORDERED, MessageBuffer};
use serde::{Deserialize, Serialize};
use std::{
    thread,
    time::{Duration, Instant},
    sync::Arc,
};
use tracing::*;
use tracing_subscriber::EnvFilter;
use uvth::ThreadPoolBuilder;

#[derive(Serialize, Deserialize, Debug)]
enum Msg {
    Ping { id: u64, data: Vec<u8> },
    Pong { id: u64, data: Vec<u8> },
}

/// This utility tests the speed of veloren network by creating a client that
/// opens a stream and pipes as many messages through it as possible.
fn main() {
    let matches = App::new("Veloren Speed Test Utility")
        .version("0.1.0")
        .author("Marcel Märtens <marcel.cochem@googlemail.com>")
        .about("Runs speedtests regarding different parameter to benchmark veloren-network")
        .arg(
            Arg::with_name("mode")
                .short("m")
                .long("mode")
                .takes_value(true)
                .possible_values(&["server", "client", "both"])
                .default_value("both")
                .help(
                    "choose whether you want to start the server or client or both needed for \
                     this program",
                ),
        )
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .default_value("52000")
                .help("port to listen on"),
        )
        .arg(
            Arg::with_name("ip")
                .long("ip")
                .takes_value(true)
                .default_value("127.0.0.1")
                .help("ip to listen and connect to"),
        )
        .arg(
            Arg::with_name("protocol")
                .long("protocol")
                .takes_value(true)
                .default_value("tcp")
                .possible_values(&["tcp", "udp", "mpsc"])
                .help(
                    "underlying protocol used for this test, mpsc can only combined with mode=both",
                ),
        )
        .arg(
            Arg::with_name("trace")
                .short("t")
                .long("trace")
                .takes_value(true)
                .default_value("warn")
                .possible_values(&["trace", "debug", "info", "warn", "error"])
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
    let address = match matches.value_of("protocol") {
        Some("tcp") => Address::Tcp(format!("{}:{}", ip, port).parse().unwrap()),
        Some("udp") => Address::Udp(format!("{}:{}", ip, port).parse().unwrap()),
        _ => panic!("invalid mode, run --help!"),
    };

    let mut background = None;
    match matches.value_of("mode") {
        Some("server") => server(address),
        Some("client") => client(address),
        Some("both") => {
            let address1 = address.clone();
            background = Some(thread::spawn(|| server(address1)));
            thread::sleep(Duration::from_millis(200)); //start client after server
            client(address);
        },
        _ => panic!("invalid mode, run --help!"),
    };
    if let Some(background) = background {
        background.join().unwrap();
    }
}

fn server(address: Address) {
    let thread_pool = ThreadPoolBuilder::new().num_threads(1).build();
    let mut metrics = metrics::SimpleMetrics::new();
    let server = Network::new(Pid::new(), &thread_pool, Some(metrics.registry()));
    metrics.run("0.0.0.0:59112".parse().unwrap()).unwrap();
    block_on(server.listen(address)).unwrap();

    loop {
        info!("waiting for participant to connect");
        let p1 = block_on(server.connected()).unwrap(); //remote representation of p1
        let mut s1 = block_on(p1.opened()).unwrap(); //remote representation of s1
        block_on(async {
            let mut last = Instant::now();
            let mut id = 0u64;
            while let Ok(_msg) = s1.recv_raw().await {
                id += 1;
                if id.rem_euclid(1000000) == 0 {
                    let new = Instant::now();
                    let diff = new.duration_since(last);
                    last = new;
                    println!("recv 1.000.000 took {}", diff.as_millis());
                }
            }
            info!("other stream was closed");
        });
    }
}

fn client(address: Address) {
    let thread_pool = ThreadPoolBuilder::new().num_threads(1).build();
    let mut metrics = metrics::SimpleMetrics::new();
    let client = Network::new(Pid::new(), &thread_pool, Some(metrics.registry()));
    metrics.run("0.0.0.0:59111".parse().unwrap()).unwrap();

    let p1 = block_on(client.connect(address.clone())).unwrap(); //remote representation of p1
    let mut s1 = block_on(p1.open(16, PROMISES_ORDERED | PROMISES_CONSISTENCY)).unwrap(); //remote representation of s1
    let mut last = Instant::now();
    let mut id = 0u64;
    let raw_msg = Arc::new(MessageBuffer{
        data: bincode::serialize(&Msg::Ping {
            id,
            data: vec![0; 1000],
        }).unwrap(),
    });
    loop {
        s1.send_raw(raw_msg.clone()).unwrap();
        id += 1;
        if id.rem_euclid(1000000) == 0 {
            let new = Instant::now();
            let diff = new.duration_since(last);
            last = new;
            println!("send 1.000.000 took {}", diff.as_millis());
        }
        if id > 2000000 {
            println!("stop");
            std::thread::sleep(std::time::Duration::from_millis(5000));
            break;
        }
    };
    drop(s1);
    std::thread::sleep(std::time::Duration::from_millis(5000));
    info!("closing participant");
    block_on(client.disconnect(p1)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(75000));
    info!("DROPPING! client");
    drop(client);
    std::thread::sleep(std::time::Duration::from_millis(75000));
}
