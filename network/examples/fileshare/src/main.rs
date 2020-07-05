#![feature(async_closure, exclusive_range_pattern)]
//!run with
//! (cd network/examples/fileshare && RUST_BACKTRACE=1 cargo run --profile=release -Z unstable-options  -- --trace=info --port 15006)
//! (cd network/examples/fileshare && RUST_BACKTRACE=1 cargo run --profile=release -Z unstable-options  -- --trace=info --port 15007)
//! ```
use async_std::{io, path::PathBuf};
use clap::{App, Arg, SubCommand};
use futures::{
    channel::mpsc,
    executor::{block_on, ThreadPool},
    sink::SinkExt,
};
use network::Address;
use std::{thread, time::Duration};
use tracing::*;
use tracing_subscriber::EnvFilter;
mod commands;
mod server;
use commands::{FileInfo, LocalCommand};
use server::Server;

fn main() {
    let matches = App::new("File Server")
        .version("0.1.0")
        .author("Marcel Märtens <marcel.cochem@googlemail.com>")
        .about("example file server implemented with veloren-network")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .default_value("15006")
                .help("port to listen on"),
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
        .add_directive("fileshare::server=trace".parse().unwrap())
        .add_directive("fileshare::commands=trace".parse().unwrap());
    tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_env_filter(filter)
        .init();

    let port: u16 = matches.value_of("port").unwrap().parse().unwrap();
    let address = Address::Tcp(format!("{}:{}", "127.0.0.1", port).parse().unwrap());

    let (server, cmd_sender) = Server::new();
    let pool = ThreadPool::new().unwrap();
    pool.spawn_ok(server.run(address));

    thread::sleep(Duration::from_millis(50)); //just for trace

    block_on(client(cmd_sender));
}

fn file_exists(file: String) -> Result<(), String> {
    let file: std::path::PathBuf = shellexpand::tilde(&file).parse().unwrap();
    if file.exists() {
        Ok(())
    } else {
        Err(format!("File does not exist"))
    }
}

fn get_options<'a, 'b>() -> App<'a, 'b> {
    App::new("")
        .setting(clap::AppSettings::NoBinaryName)
        .setting(clap::AppSettings::SubcommandRequired)
        .setting(clap::AppSettings::VersionlessSubcommands)
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .setting(clap::AppSettings::ColorAuto)
        .subcommand(SubCommand::with_name("quit").about("closes program"))
        .subcommand(SubCommand::with_name("disconnect").about("stop connections to all servers"))
        .subcommand(SubCommand::with_name("t").about("quick test by connectiong to 127.0.0.1:1231"))
        .subcommand(
            SubCommand::with_name("connect")
                .about("opens a connection to another instance of this fileserver network")
                .setting(clap::AppSettings::NoBinaryName)
                .arg(
                    Arg::with_name("ip:port")
                        .help("ip and port to connect to, example '127.0.0.1:1231'")
                        .required(true)
                        .validator(|ipport| match ipport.parse::<std::net::SocketAddr>() {
                            Ok(_) => Ok(()),
                            Err(e) => Err(format!("must be valid Ip:Port combination {:?}", e)),
                        }),
                ),
        )
        .subcommand(SubCommand::with_name("list").about("lists all available files on the network"))
        .subcommand(
            SubCommand::with_name("serve")
                .about("make file available on the network")
                .arg(
                    Arg::with_name("file")
                        .help("file to serve")
                        .required(true)
                        .validator(file_exists),
                ),
        )
        .subcommand(
            SubCommand::with_name("get")
                .about(
                    "downloads file with the id from the `list` command. Optionally provide a \
                     storage path, if none is provided it will be saved in the current directory \
                     with the remote filename",
                )
                .arg(
                    Arg::with_name("id")
                        .help("id to download. get the id from the `list` command")
                        .required(true)
                        .validator(|id| match id.parse::<u32>() {
                            Ok(_) => Ok(()),
                            Err(e) => Err(format!("must be a number {:?}", e)),
                        }),
                )
                .arg(Arg::with_name("file").help("local path to store the file to")),
        )
}

async fn client(mut cmd_sender: mpsc::UnboundedSender<LocalCommand>) {
    use std::io::Write;

    loop {
        let mut line = String::new();
        print!("==> ");
        std::io::stdout().flush().unwrap();
        io::stdin().read_line(&mut line).await.unwrap();
        let matches = match get_options().get_matches_from_safe(line.split_whitespace()) {
            Err(e) => {
                println!("{}", e.message);
                continue;
            },
            Ok(matches) => matches,
        };

        match matches.subcommand() {
            ("quit", _) => {
                cmd_sender.send(LocalCommand::Shutdown).await.unwrap();
                println!("goodbye");
                break;
            },
            ("disconnect", _) => {
                cmd_sender.send(LocalCommand::Disconnect).await.unwrap();
            },
            ("connect", Some(connect_matches)) => {
                let socketaddr = connect_matches.value_of("ip:port").unwrap().parse().unwrap();
                cmd_sender
                    .send(LocalCommand::Connect(Address::Tcp(socketaddr)))
                    .await
                    .unwrap();
            },
            ("t", _) => {
                cmd_sender
                    .send(LocalCommand::Connect(Address::Tcp(
                        "127.0.0.1:1231".parse().unwrap(),
                    )))
                    .await
                    .unwrap();
            },
            ("serve", Some(serve_matches)) => {
                let path = shellexpand::tilde(serve_matches.value_of("file").unwrap());
                let path: PathBuf = path.parse().unwrap();
                if let Some(fileinfo) = FileInfo::new(&path).await {
                    cmd_sender
                        .send(LocalCommand::Serve(fileinfo))
                        .await
                        .unwrap();
                }
            },
            ("list", _) => {
                cmd_sender.send(LocalCommand::List).await.unwrap();
            },
            ("get", Some(get_matches)) => {
                let id: u32 = get_matches.value_of("id").unwrap().parse().unwrap();
                let file = get_matches.value_of("file");
                cmd_sender
                    .send(LocalCommand::Get(id, file.map(|s| s.to_string())))
                    .await
                    .unwrap();
            },

            (_, _) => {
                unreachable!("this subcommand isn't yet handled");
            },
        }
        // this 100 ms is because i am super lazy, and i want to appear the logs before
        // the next '==>' appears...
        thread::sleep(Duration::from_millis(100));
        println!("");
    }
    thread::sleep(Duration::from_millis(30)); // TODO: still needed for correct shutdown
}
