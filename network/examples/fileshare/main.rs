#![feature(async_closure, exclusive_range_pattern)]
//!run with
//! (cd network/examples/fileshare && RUST_BACKTRACE=1 cargo run
//! --profile=release -Z unstable-options  -- --trace=info --port 15006)
//! (cd network/examples/fileshare && RUST_BACKTRACE=1 cargo run
//! --profile=release -Z unstable-options  -- --trace=info --port 15007) ```
use clap::{Arg, Command};
use std::{path::PathBuf, sync::Arc, thread, time::Duration};
use tokio::{io, io::AsyncBufReadExt, runtime::Runtime, sync::mpsc};
use tracing::*;
use tracing_subscriber::EnvFilter;
use veloren_network::{ConnectAddr, ListenAddr};
mod commands;
mod server;
use commands::{FileInfo, LocalCommand};
use server::Server;

fn main() {
    let matches = Command::new("File Server")
        .version("0.1.0")
        .author("Marcel Märtens <marcel.cochem@googlemail.com>")
        .about("example file server implemented with veloren-network")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .default_value("15006")
                .value_parser(clap::value_parser!(u16))
                .help("port to listen on"),
        )
        .arg(
            Arg::new("trace")
                .short('t')
                .long("trace")
                .default_value("warn")
                .value_parser(["trace", "debug", "info", "warn", "error"])
                .help("set trace level, not this has a performance impact!"),
        )
        .get_matches();

    let trace = matches.get_one::<String>("trace").unwrap();
    let filter = EnvFilter::from_default_env()
        .add_directive(trace.parse().unwrap())
        .add_directive("fileshare::server=trace".parse().unwrap())
        .add_directive("fileshare::commands=trace".parse().unwrap());
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_env_filter(filter)
        .init();

    let port = matches.get_one::<u32>("port").unwrap();
    let address = ListenAddr::Tcp(format!("{}:{}", "127.0.0.1", port).parse().unwrap());
    let runtime = Arc::new(Runtime::new().unwrap());

    let (server, cmd_sender) = Server::new(Arc::clone(&runtime));
    runtime.spawn(server.run(address));

    thread::sleep(Duration::from_millis(50)); //just for trace

    runtime.block_on(client(cmd_sender));
}

fn file_exists(file: &str) -> Result<(), String> {
    let file: PathBuf = shellexpand::tilde(file).parse().unwrap();
    if file.exists() {
        Ok(())
    } else {
        Err("File does not exist".to_string())
    }
}

fn get_options() -> Command {
    Command::new("")
        .no_binary_name(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .color(clap::ColorChoice::Auto)
        .subcommand(Command::new("quit").about("closes program"))
        .subcommand(Command::new("disconnect").about("stop connections to all servers"))
        .subcommand(Command::new("t").about("quick test by connecting to 127.0.0.1:1231"))
        .subcommand(
            Command::new("connect")
                .about("opens a connection to another instance of this fileserver network")
                .no_binary_name(true)
                .arg(
                    Arg::new("ip:port")
                        .help("ip and port to connect to, example '127.0.0.1:1231'")
                        .required(true)
                        .value_parser(clap::value_parser!(std::net::SocketAddr)),
                ),
        )
        .subcommand(Command::new("list").about("lists all available files on the network"))
        .subcommand(
            Command::new("serve")
                .about("make file available on the network")
                .arg(
                    Arg::new("file")
                        .help("file to serve")
                        .required(true)
                        .value_parser(file_exists),
                ),
        )
        .subcommand(
            Command::new("get")
                .about(
                    "downloads file with the id from the `list` command. Optionally provide a \
                     storage path, if none is provided it will be saved in the current directory \
                     with the remote filename",
                )
                .arg(
                    Arg::new("id")
                        .help("id to download. get the id from the `list` command")
                        .required(true)
                        .value_parser(clap::value_parser!(u32)),
                )
                .arg(Arg::new("file").help("local path to store the file to")),
        )
}

async fn client(cmd_sender: mpsc::UnboundedSender<LocalCommand>) {
    use std::io::Write;

    loop {
        let mut line = String::new();
        let mut input_lines = io::BufReader::new(io::stdin());
        print!("==> ");
        std::io::stdout().flush().unwrap();
        input_lines.read_line(&mut line).await.unwrap();
        let matches = match get_options().try_get_matches_from(line.split_whitespace()) {
            Err(e) => {
                println!("{}", e);
                continue;
            },
            Ok(matches) => matches,
        };

        match matches.subcommand() {
            None => {
                println!("unknown subcommand");
                break;
            },
            Some(("quit", _)) => {
                cmd_sender.send(LocalCommand::Shutdown).unwrap();
                println!("goodbye");
                break;
            },
            Some(("disconnect", _)) => {
                cmd_sender.send(LocalCommand::Disconnect).unwrap();
            },
            Some(("connect", connect_matches)) => {
                let socketaddr = connect_matches
                    .get_one::<String>("ip:port")
                    .unwrap()
                    .parse()
                    .unwrap();
                cmd_sender
                    .send(LocalCommand::Connect(ConnectAddr::Tcp(socketaddr)))
                    .unwrap();
            },
            Some(("t", _)) => {
                cmd_sender
                    .send(LocalCommand::Connect(ConnectAddr::Tcp(
                        "127.0.0.1:1231".parse().unwrap(),
                    )))
                    .unwrap();
            },
            Some(("serve", serve_matches)) => {
                let path = shellexpand::tilde(serve_matches.get_one::<String>("file").unwrap());
                let path: PathBuf = path.parse().unwrap();
                if let Some(fileinfo) = FileInfo::new(&path).await {
                    cmd_sender.send(LocalCommand::Serve(fileinfo)).unwrap();
                }
            },
            Some(("list", _)) => {
                cmd_sender.send(LocalCommand::List).unwrap();
            },
            Some(("get", get_matches)) => {
                let id = *get_matches.get_one::<u32>("id").unwrap();
                let file = get_matches.get_one::<String>("file");
                cmd_sender
                    .send(LocalCommand::Get(id, file.map(|s| s.to_string())))
                    .unwrap();
            },

            Some((_, _)) => {
                unreachable!("this subcommand isn't yet handled");
            },
        }
        // this 100 ms is because i am super lazy, and i want to appear the logs before
        // the next '==>' appears...
        thread::sleep(Duration::from_millis(100));
        println!();
    }
    thread::sleep(Duration::from_millis(30)); // TODO: still needed for correct shutdown
}
