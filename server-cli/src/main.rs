extern crate server;

use server::Server;

fn main() {
    println!("Started server-cli...");

    let mut server = Server::new(1227, 1024);
    while server.next_tick() {}
}
