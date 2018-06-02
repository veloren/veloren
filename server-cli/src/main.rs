extern crate server;
extern crate get_if_addrs;

use std::net::SocketAddr;
use server::ServerHandle;

const PORT: u16 = 59003;

fn main() {
    println!("Started server-cli...");

    let ip = get_if_addrs::get_if_addrs().unwrap()[0].ip();

    println!("Hosting on {}:{}...", ip.to_string(), PORT);

    let mut server = ServerHandle::new(SocketAddr::new(ip, PORT), 1227, 1024)
        .expect("Could not create server");

    server.run();
}
