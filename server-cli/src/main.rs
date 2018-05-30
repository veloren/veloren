extern crate local_ip;
extern crate server;

use std::net::SocketAddr;
use server::Server;

const PORT: u16 = 59003;

fn main() {
    println!("Started server-cli...");

    let ip = local_ip::get().unwrap();

    println!("Hosting on {}:{}...", ip.to_string(), PORT);

    let mut server = Server::new(SocketAddr::new(ip, PORT), 1227, 1024).expect("Could not create server");
    while server.next_tick() {}
}
