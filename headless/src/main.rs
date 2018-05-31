extern crate client;
extern crate local_ip;

use std::io;
use std::net::SocketAddr;
use client::{Client, ClientMode};

const PORT: u16 = 59002;

fn main() {
    println!("Starting headless client...");


    let ip = local_ip::get().unwrap();

    let mut remote_addr = String::new();
    println!("Remote server address:");
    io::stdin().read_line(&mut remote_addr).unwrap();

    let mut client = match Client::new(ClientMode::Headless, SocketAddr::new(ip, PORT), &remote_addr.trim()) {
        Ok(c) => c,
        Err(e) => panic!("An error occured when attempting to initiate the client: {:?}", e),
    };

    client.connect();
}
