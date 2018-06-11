#![feature(nll)]

#[macro_use] extern crate log;
extern crate pretty_env_logger;
extern crate server;
extern crate get_if_addrs;
extern crate common;

use std::io;
use std::net::SocketAddr;
use common::get_version;
use server::Server;

const PORT: u16 = 59003;

fn main() {
    pretty_env_logger::init();
    info!("Started server-cli... Version: {}", get_version());

    let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(0,0,0,0));
    info!("Hosting on {}:{}...", ip.to_string(), PORT);

    let mut server = ServerHandle::new(SocketAddr::new(ip, PORT), 1227, 1024)
        .expect("Could not create server");

    server.run();
}
