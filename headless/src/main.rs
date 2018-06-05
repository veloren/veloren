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

    let ip = get_if_addrs::get_if_addrs().unwrap()[0].ip();

    let mut port = String::new();
    println!("Local port [59001]:");
    io::stdin().read_line(&mut port).unwrap();
    let port = u16::from_str_radix(&port.trim(), 10).unwrap();

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
