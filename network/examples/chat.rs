//!run with
//! ```bash
//! RUST_BACKTRACE=1 cargo run --example chat -- --trace=info --port 15006
//! RUST_BACKTRACE=1 cargo run --example chat -- --trace=info --port 15006 --mode=client
//! ```
use clap::{Arg, Command};
use std::{sync::Arc, thread, time::Duration};
use tokio::{io, io::AsyncBufReadExt, runtime::Runtime, sync::RwLock};
use tracing::*;
use tracing_subscriber::EnvFilter;
use veloren_network::{ConnectAddr, ListenAddr, Network, Participant, Pid, Promises, Stream};

///This example contains a simple chatserver, that allows to send messages
/// between participants, it's neither pretty nor perfect, but it should show
/// how to integrate network
fn main() {
    let matches = Command::new("Chat example")
        .version("0.1.0")
        .author("Marcel Märtens <marcel.cochem@googlemail.com>")
        .about("example chat implemented with veloren-network")
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
                .possible_values(["tcp", "upd", "mpsc"])
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
    let filter = EnvFilter::from_default_env().add_directive(trace.parse().unwrap());
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
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
    match matches.value_of("mode") {
        Some("server") => server(addresses.0),
        Some("client") => client(addresses.1),
        Some("both") => {
            let s = addresses.0;
            background = Some(thread::spawn(|| server(s)));
            thread::sleep(Duration::from_millis(200)); //start client after server
            client(addresses.1)
        },
        _ => panic!("invalid mode, run --help!"),
    };
    if let Some(background) = background {
        background.join().unwrap();
    }
}

fn server(address: ListenAddr) {
    let r = Arc::new(Runtime::new().unwrap());
    let mut server = Network::new(Pid::new(), &r);
    let participants = Arc::new(RwLock::new(Vec::new()));
    r.block_on(async {
        server.listen(address).await.unwrap();
        loop {
            let mut p1 = server.connected().await.unwrap();
            let s1 = p1.opened().await.unwrap();
            participants.write().await.push(p1);
            tokio::spawn(client_connection(s1, participants.clone()));
        }
    });
}

async fn client_connection(mut s1: Stream, participants: Arc<RwLock<Vec<Participant>>>) {
    let username = s1.recv::<String>().await.unwrap();
    println!("[{}] connected", username);
    loop {
        match s1.recv::<String>().await {
            Err(_) => {
                break;
            },
            Ok(msg) => {
                println!("[{}]: {}", username, msg);
                for p in participants.read().await.iter() {
                    match p
                        .open(4, Promises::ORDERED | Promises::CONSISTENCY, 0)
                        .await
                    {
                        Err(_) => info!("error talking to client, //TODO drop it"),
                        Ok(s) => s.send((username.clone(), msg.clone())).unwrap(),
                    };
                }
            },
        }
    }
    println!("[{}] disconnected", username);
}

fn client(address: ConnectAddr) {
    let r = Arc::new(Runtime::new().unwrap());
    let client = Network::new(Pid::new(), &r);

    r.block_on(async {
        let p1 = client.connect(address.clone()).await.unwrap(); //remote representation of p1
        let s1 = p1
            .open(4, Promises::ORDERED | Promises::CONSISTENCY, 0)
            .await
            .unwrap(); //remote representation of s1
        let mut input_lines = io::BufReader::new(io::stdin());
        println!("Enter your username:");
        let mut username = String::new();
        input_lines.read_line(&mut username).await.unwrap();
        username = username.split_whitespace().collect();
        println!("Your username is: {}", username);
        println!("write /quit to close");
        tokio::spawn(read_messages(p1));
        s1.send(username).unwrap();
        loop {
            let mut line = String::new();
            input_lines.read_line(&mut line).await.unwrap();
            line = line.split_whitespace().collect();
            if line.as_str() == "/quit" {
                println!("goodbye");
                break;
            } else {
                s1.send(line).unwrap();
            }
        }
    });
    thread::sleep(Duration::from_millis(30)); // TODO: still needed for correct shutdown
}

// I am quite lazy, the sending is done in a single stream above, but for
// receiving i open and close a stream per message. this can be done easier but
// this allows me to be quite lazy on the server side and just get a list of
// all participants and send to them...
async fn read_messages(mut participant: Participant) {
    while let Ok(mut s) = participant.opened().await {
        let (username, message) = s.recv::<(String, String)>().await.unwrap();
        println!("[{}]: {}", username, message);
    }
    println!("gracefully shut down");
}
