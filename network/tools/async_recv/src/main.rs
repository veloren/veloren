use chrono::prelude::*;
use clap::{App, Arg, SubCommand};
use futures::executor::block_on;
use network::{Address, Network, Participant, Promise, Stream};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use tracing::*;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;
use uvth::ThreadPoolBuilder;

#[derive(Serialize, Deserialize, Debug)]
enum Msg {
    Ping(u64),
    Pong(u64),
}

fn main() {
    let matches = App::new("Veloren Speed Test Utility")
        .version("0.1.0")
        .author("Marcel Märtens <marcel.cochem@googlemail.com>")
        .about("Runs speedtests regarding different parameter to benchmark veloren-network")
        .subcommand(
            SubCommand::with_name("listen")
                .about("Runs the counter part that pongs all requests")
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .takes_value(true)
                        .help("port to listen on"),
                ),
        )
        .subcommand(
            SubCommand::with_name("run").arg(
                Arg::with_name("port")
                    .short("p")
                    .long("port")
                    .takes_value(true)
                    .help("port to connect too"),
            ),
        )
        .get_matches();

    let filter = EnvFilter::from_default_env().add_directive("error".parse().unwrap());
    //.add_directive("veloren_network::tests=trace".parse().unwrap());

    tracing_subscriber::FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, info, warn, etc.)
        // will be written to stdout.
        .with_max_level(Level::TRACE)
        .with_env_filter(filter)
        // sets this to be the default, global subscriber for this application.
        .init();

    if let Some(matches) = matches.subcommand_matches("listen") {
        let port = matches
            .value_of("port")
            .map_or(52000, |v| v.parse::<u16>().unwrap_or(52000));
        server(port);
    };
    if let Some(matches) = matches.subcommand_matches("run") {
        let port = matches
            .value_of("port")
            .map_or(52000, |v| v.parse::<u16>().unwrap_or(52000));
        client(port);
    };
}

fn server(port: u16) {
    let thread_pool = Arc::new(
        ThreadPoolBuilder::new()
            .name("veloren-network-server".into())
            .build(),
    );
    thread::sleep(Duration::from_millis(200));
    let server = Network::new(Uuid::new_v4(), thread_pool.clone());
    let address = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], port)));
    block_on(server.listen(&address)).unwrap(); //await
    thread::sleep(Duration::from_millis(10)); //TODO: listeing still doesnt block correctly!
    println!("waiting for client");

    let p1 = block_on(server.connected()).unwrap(); //remote representation of p1
    let s1 = block_on(p1.opened()).unwrap(); //remote representation of s1
    let s2 = block_on(p1.opened()).unwrap(); //remote representation of s2
    let t1 = thread::spawn(move || {
        if let Ok(Msg::Ping(id)) = block_on(s1.recv()) {
            thread::sleep(Duration::from_millis(3000));
            s1.send(Msg::Pong(id));
            println!("[{}], send s1_1", Utc::now().time());
        }
        if let Ok(Msg::Ping(id)) = block_on(s1.recv()) {
            thread::sleep(Duration::from_millis(3000));
            s1.send(Msg::Pong(id));
            println!("[{}], send s1_2", Utc::now().time());
        }
    });
    let t2 = thread::spawn(move || {
        if let Ok(Msg::Ping(id)) = block_on(s2.recv()) {
            thread::sleep(Duration::from_millis(1000));
            s2.send(Msg::Pong(id));
            println!("[{}], send s2_1", Utc::now().time());
        }
        if let Ok(Msg::Ping(id)) = block_on(s2.recv()) {
            thread::sleep(Duration::from_millis(1000));
            s2.send(Msg::Pong(id));
            println!("[{}], send s2_2", Utc::now().time());
        }
    });
    t1.join();
    t2.join();
    thread::sleep(Duration::from_millis(50));
}

async fn async_task1(s: Stream) -> u64 {
    s.send(Msg::Ping(100));
    println!("[{}], s1_1...", Utc::now().time());
    let m1: Result<Msg, _> = s.recv().await;
    println!("[{}], s1_1: {:?}", Utc::now().time(), m1);
    thread::sleep(Duration::from_millis(1000));
    s.send(Msg::Ping(101));
    println!("[{}], s1_2...", Utc::now().time());
    let m2: Result<Msg, _> = s.recv().await;
    println!("[{}], s1_2: {:?}", Utc::now().time(), m2);
    match m2.unwrap() {
        Msg::Pong(id) => id,
        _ => panic!("wrong answer"),
    }
}

async fn async_task2(s: Stream) -> u64 {
    s.send(Msg::Ping(200));
    println!("[{}], s2_1...", Utc::now().time());
    let m1: Result<Msg, _> = s.recv().await;
    println!("[{}], s2_1: {:?}", Utc::now().time(), m1);
    thread::sleep(Duration::from_millis(5000));
    s.send(Msg::Ping(201));
    println!("[{}], s2_2...", Utc::now().time());
    let m2: Result<Msg, _> = s.recv().await;
    println!("[{}], s2_2: {:?}", Utc::now().time(), m2);
    match m2.unwrap() {
        Msg::Pong(id) => id,
        _ => panic!("wrong answer"),
    }
}

fn client(port: u16) {
    let thread_pool = Arc::new(
        ThreadPoolBuilder::new()
            .name("veloren-network-server".into())
            .build(),
    );
    thread::sleep(Duration::from_millis(200));
    let client = Network::new(Uuid::new_v4(), thread_pool.clone());
    let address = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], port)));
    thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

    let p1 = block_on(client.connect(&address)).unwrap(); //remote representation of p1
    let s1 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap(); //remote representation of s1
    let s2 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap(); //remote representation of s2
    let before = Instant::now();
    block_on(async {
        let f1 = async_task1(s1);
        let f2 = async_task2(s2);
        let x = futures::join!(f1, f2);
    });
    if before.elapsed() < Duration::from_secs(13) {
        println!("IT WORKS!");
    } else {
        println!("doesn't seem to work :/")
    }
    thread::sleep(Duration::from_millis(50));
}
