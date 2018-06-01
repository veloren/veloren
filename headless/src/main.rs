extern crate client;
extern crate local_ip;
extern crate syrup;

use std::io;
use std::sync::mpsc;
use std::net::SocketAddr;

use syrup::Window;

use client::{ClientHandle, ClientMode};

const PORT: u16 = 59002;

fn main() {
    println!("Starting headless client...");

    let ip = local_ip::get().unwrap();

    let mut port = String::new();
    println!("Port:");
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

    let mut client = match ClientHandle::new(ClientMode::Headless, &alias.trim(), SocketAddr::new(ip, port), &remote_addr.trim()) {
        Ok(c) => c,
        Err(e) => panic!("An error occured when attempting to initiate the client: {:?}", e),
    };

    let (tx, rx) = mpsc::channel();
    client.set_chat_callback(move |alias, msg| {
        tx.send(format!("{}: {}", alias, msg)).unwrap();
    });

    client.run();

    loop {
        if let Ok(msg) = rx.try_recv() {
            win.writeln(format!("{}", msg));
        }

        if let Some(msg) = win.get() {
            client.send_chat_msg(&msg);
        }
    }
}
