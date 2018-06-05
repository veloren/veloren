#![feature(nll)]

#[macro_use] extern crate log;
extern crate pretty_env_logger;
extern crate server;
extern crate get_if_addrs;

use std::io;
use std::net::SocketAddr;
use server::ServerHandle;

const PORT: u16 = 59003;

fn main() {
    pretty_env_logger::init();
    info!("Started server-cli...");

    let ifaces = get_if_addrs::get_if_addrs().unwrap();
    for (i, iface) in ifaces.iter().enumerate() {
        println!("[{}] {}", i, iface.ip().to_string());
    }
    let ip = loop {
        let mut ip_index = String::new();
        println!("Enter the number of the IP address you wish to choose.");
        io::stdin().read_line(&mut ip_index).unwrap();

        if let Ok(index) = ip_index.trim().parse::<usize>() {
            if let Some(iface) = ifaces.get(index) {
                break iface.ip();
            }
        }
        println!("Invalid number!");
    };

    println!("Hosting on {}:{}...", ip.to_string(), PORT);

    let mut server = ServerHandle::new(SocketAddr::new(ip, PORT), 1227, 1024)
        .expect("Could not create server");

    server.run();
}
