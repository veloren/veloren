extern crate server;

use server::Server;

fn main() {

    println!("Server starting...");

    let server = Server::new();
    server.start();
}