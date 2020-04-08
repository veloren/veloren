use chrono::prelude::*;
use clap::{App, Arg};
use futures::executor::block_on;
use network::{Address, Network, Pid, Stream, PROMISES_NONE};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    thread,
    time::{Duration, Instant},
};
use tracing::*;
use tracing_subscriber::EnvFilter;
use uvth::ThreadPoolBuilder;

#[derive(Serialize, Deserialize, Debug)]
enum Msg {
    Ping(u64),
    Pong(u64),
}

/// This utility checks if async functionatily of veloren-network works
/// correctly and outputs it at the end
fn main() {
    let matches = App::new("Veloren Async Prove Utility")
        .version("0.1.0")
        .author("Marcel Märtens <marcel.cochem@googlemail.com>")
        .about("proves that veloren-network runs async")
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
                .possible_values(&["tcp", "upd", "mpsc"])
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

    if let Some(trace) = matches.value_of("trace") {
        let filter = EnvFilter::from_default_env().add_directive(trace.parse().unwrap());
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(Level::TRACE)
            .with_env_filter(filter)
            .init();
    };
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
            client(address)
        },
        _ => panic!("invalid mode, run --help!"),
    };
    if let Some(background) = background {
        background.join().unwrap();
    }
}

fn server(address: Address) {
    let thread_pool = ThreadPoolBuilder::new().build();
    let server = Network::new(Pid::new(), &thread_pool);
    block_on(server.listen(address.clone())).unwrap(); //await
    println!("waiting for client");

    let p1 = block_on(server.connected()).unwrap(); //remote representation of p1
    let mut s1 = block_on(p1.opened()).unwrap(); //remote representation of s1
    let mut s2 = block_on(p1.opened()).unwrap(); //remote representation of s2
    let t1 = thread::spawn(move || {
        if let Ok(Msg::Ping(id)) = block_on(s1.recv()) {
            thread::sleep(Duration::from_millis(3000));
            s1.send(Msg::Pong(id)).unwrap();
            println!("[{}], send s1_1", Utc::now().time());
        }
        if let Ok(Msg::Ping(id)) = block_on(s1.recv()) {
            thread::sleep(Duration::from_millis(3000));
            s1.send(Msg::Pong(id)).unwrap();
            println!("[{}], send s1_2", Utc::now().time());
        }
        thread::sleep(Duration::from_millis(10000));
    });
    let t2 = thread::spawn(move || {
        if let Ok(Msg::Ping(id)) = block_on(s2.recv()) {
            thread::sleep(Duration::from_millis(1000));
            s2.send(Msg::Pong(id)).unwrap();
            println!("[{}], send s2_1", Utc::now().time());
        }
        if let Ok(Msg::Ping(id)) = block_on(s2.recv()) {
            thread::sleep(Duration::from_millis(1000));
            s2.send(Msg::Pong(id)).unwrap();
            println!("[{}], send s2_2", Utc::now().time());
        }
        thread::sleep(Duration::from_millis(10000));
    });
    t1.join().unwrap();
    t2.join().unwrap();
    thread::sleep(Duration::from_millis(50));
}

async fn async_task1(mut s: Stream) -> u64 {
    s.send(Msg::Ping(100)).unwrap();
    println!("[{}], s1_1...", Utc::now().time());
    let m1: Result<Msg, _> = s.recv().await;
    println!("[{}], s1_1: {:?}", Utc::now().time(), m1);
    thread::sleep(Duration::from_millis(1000));
    s.send(Msg::Ping(101)).unwrap();
    println!("[{}], s1_2...", Utc::now().time());
    let m2: Result<Msg, _> = s.recv().await;
    println!("[{}], s1_2: {:?}", Utc::now().time(), m2);
    match m2.unwrap() {
        Msg::Pong(id) => id,
        _ => panic!("wrong answer"),
    }
}

async fn async_task2(mut s: Stream) -> u64 {
    s.send(Msg::Ping(200)).unwrap();
    println!("[{}], s2_1...", Utc::now().time());
    let m1: Result<Msg, _> = s.recv().await;
    println!("[{}], s2_1: {:?}", Utc::now().time(), m1);
    thread::sleep(Duration::from_millis(5000));
    s.send(Msg::Ping(201)).unwrap();
    println!("[{}], s2_2...", Utc::now().time());
    let m2: Result<Msg, _> = s.recv().await;
    println!("[{}], s2_2: {:?}", Utc::now().time(), m2);
    match m2.unwrap() {
        Msg::Pong(id) => id,
        _ => panic!("wrong answer"),
    }
}

fn client(address: Address) {
    let thread_pool = ThreadPoolBuilder::new().build();
    let client = Network::new(Pid::new(), &thread_pool);

    let p1 = block_on(client.connect(address.clone())).unwrap(); //remote representation of p1
    let s1 = block_on(p1.open(16, PROMISES_NONE)).unwrap(); //remote representation of s1
    let s2 = block_on(p1.open(16, PROMISES_NONE)).unwrap(); //remote representation of s2
    let before = Instant::now();
    block_on(async {
        let f1 = async_task1(s1);
        let f2 = async_task2(s2);
        let _ = futures::join!(f1, f2);
    });
    if before.elapsed() < Duration::from_secs(13) {
        println!("IT WORKS!");
    } else {
        println!("doesn't seem to work :/")
    }
    thread::sleep(Duration::from_millis(50));
}
