extern crate client;

use std::io;
use client::{Client, ClientMode};

fn main() {
    println!("Starting headless client...");

    let mut bind_addr = String::new();
    let mut remote_addr = String::new();

    println!("Local bind address:");
    io::stdin().read_line(&mut bind_addr).unwrap();

    println!("Remote server address:");
    io::stdin().read_line(&mut remote_addr).unwrap();

    let mut client = match Client::new(ClientMode::Headless, &bind_addr, &remote_addr) {
        Ok(c) => c,
        Err(_) => panic!("An error occured when attempting to initiate the client"),
    };
}
