extern crate server;

use std::net::ToSocketAddrs;
use server::Server;

fn main() {
    println!("Started server-cli...");

    let mut server = Server::new("127.0.0.1:1337", 1227, 1024).expect("Could not create server");
    while server.next_tick() {}
}
