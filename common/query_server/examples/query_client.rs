use std::time::Duration;

use clap::Parser;
use tracing::{error, info};
use veloren_query_server::client::QueryClient;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    host: String,

    #[arg(short, long, default_value_t = 14006)]
    port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let mut ip = match tokio::net::lookup_host(format!("{}:{}", args.host, args.port)).await {
        Ok(ip) => ip,
        Err(e) => {
            error!(?e, "Couldn't look up hostname: {}", &args.host);
            return;
        },
    };
    let addr = match ip.next() {
        Some(ip) => ip,
        None => {
            error!("No IP-Addr found for: {}", &args.host);
            return;
        },
    };
    info!("Connecting to server at: {addr}");

    let mut client = QueryClient::new(addr);
    const REQUESTS: usize = 10;
    let mut infos = vec![];

    for _ in 0..REQUESTS {
        let info = client.server_info().await;
        match &info {
            Ok((_, ping)) => {
                info!("Ping: {}ms", ping.as_millis());
            },
            Err(e) => error!(?e, "Failed to fetch info from server"),
        }
        infos.push(info);
    }

    let successful = infos.iter().filter(|info| info.is_ok()).count();
    let errors = infos.iter().filter(|info| info.is_err()).count();
    let avg_ping_sum: Duration = infos
        .iter()
        .filter_map(|info| info.as_ref().ok().map(|info| info.1))
        .sum();

    println!("successful: {successful}");
    println!("errors: {errors}");
    if successful > 0 {
        println!("avg_ping: {:?}", avg_ping_sum / (successful as u32));
    }

    if let Some(last_info) = infos
        .iter()
        .rev()
        .filter_map(|info| info.as_ref().ok().map(|info| info.0))
        .next()
    {
        println!("{:?}", last_info);
    }
}
