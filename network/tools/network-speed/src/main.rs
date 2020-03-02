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
    Ping { id: u64, data: Vec<u8> },
    Pong { id: u64, data: Vec<u8> },
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
            SubCommand::with_name("run")
                .arg(
                    Arg::with_name("port")
                        .short("p")
                        .long("port")
                        .takes_value(true)
                        .help("port to connect too"),
                )
                .arg(
                    Arg::with_name("participants")
                        .long("participants")
                        .takes_value(true)
                        .help("number of participants to open"),
                )
                .arg(
                    Arg::with_name("streams")
                        .long("streams")
                        .takes_value(true)
                        .help("number of streams to open per participant"),
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
        server();
    };
    if let Some(matches) = matches.subcommand_matches("run") {
        client();
    };
}

fn server() {
    let thread_pool = Arc::new(
        ThreadPoolBuilder::new()
            .name("veloren-network-server".into())
            .build(),
    );
    thread::sleep(Duration::from_millis(200));
    let server = Network::new(Uuid::new_v4(), thread_pool.clone());
    let address = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52000)));
    block_on(server.listen(&address)).unwrap(); //await
    thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

    loop {
        let p1 = block_on(server.connected()).unwrap(); //remote representation of p1
        let s1 = block_on(p1.opened()).unwrap(); //remote representation of s1
        loop {
            let m: Result<Option<Msg>, _> = s1.recv();
            match m {
                Ok(Some(Msg::Ping { id, data })) => {
                    //s1.send(Msg::Pong {id, data});
                },
                Err(e) => {},
                _ => {},
            }
        }
    }
}

fn client() {
    let thread_pool = Arc::new(
        ThreadPoolBuilder::new()
            .name("veloren-network-server".into())
            .build(),
    );
    thread::sleep(Duration::from_millis(200));
    let client = Network::new(Uuid::new_v4(), thread_pool.clone());
    let address = Address::Tcp(SocketAddr::from(([127, 0, 0, 1], 52000)));
    thread::sleep(Duration::from_millis(3)); //TODO: listeing still doesnt block correctly!

    loop {
        let p1 = block_on(client.connect(&address)).unwrap(); //remote representation of p1
        let s1 = block_on(p1.open(16, Promise::InOrder | Promise::NoCorrupt)).unwrap(); //remote representation of s1
        let mut last = Instant::now();
        let mut id = 0u64;
        loop {
            s1.send(Msg::Ping {
                id,
                data: vec![0; 1000],
            });
            id += 1;
            if id.rem_euclid(1000000) == 0 {
                let new = Instant::now();
                let diff = new.duration_since(last);
                last = new;
                println!("1.000.000 took {}", diff.as_millis());
            }
            let _: Result<Option<Msg>, _> = s1.recv();
        }
    }
}
