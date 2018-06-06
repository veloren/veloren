#![feature(nll)]

extern crate client;
extern crate get_if_addrs;
extern crate syrup;

use std::io;
use std::sync::mpsc;
use std::net::SocketAddr;

use syrup::Window;

use client::{Client, ClientMode};

fn main() {
    println!("Starting headless client...");

    let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(0,0,0,0));
    // let ifaces = get_if_addrs::get_if_addrs().unwrap();
    // for (i, iface) in ifaces.iter().enumerate() {
    //     println!("[{}] {}", i, iface.ip().to_string());
    // }
    // let ip = loop {
    //     let mut ip_index = String::new();
    //     println!("Enter the number of the IP address you wish to choose.");
    //     io::stdin().read_line(&mut ip_index).unwrap();

    //     if let Ok(index) = ip_index.trim().parse::<usize>() {
    //         if let Some(iface) = ifaces.get(index) {
    //             break iface.ip();
    //         }
    //     }
    //     println!("Invalid number!");
    // };
    let port: u16 = 59001;

    let mut remote_addr = String::new();
    println!("Remote server address:");
    io::stdin().read_line(&mut remote_addr).unwrap();

    let mut alias = String::new();
    println!("Alias:");
    io::stdin().read_line(&mut alias).unwrap();

    let mut win = Window::initscr();
    win.writeln("Welcome to the Verloren headless client.");

    let client = match Client::new(ClientMode::Headless, alias.trim().to_string(), SocketAddr::new(ip, port), &remote_addr.trim()) {
        Ok(c) => c,
        Err(e) => panic!("An error occured when attempting to initiate the client: {:?}", e),
    };

    let (tx, rx) = mpsc::channel();
    client.set_chat_callback(move |alias, msg| {
        tx.send(format!("{}: {}", alias, msg)).unwrap();
    });

    Client::start(client.clone());

    loop {
        if let Ok(msg) = rx.try_recv() {
            win.writeln(format!("{}", msg));
        }

        if let Some(msg) = win.get() {
            if msg.starts_with("!") {
                client.send_command(&msg[1..]);
            }
            else {
                client.send_chat_msg(&msg);
            }
        }
    }
}
