#![feature(nll)]

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate get_if_addrs;
#[macro_use]
extern crate enum_map;
extern crate nalgebra;

extern crate client;
extern crate region;
extern crate common;

mod game;
mod window;
mod renderer;
mod mesh;
mod model_object;
mod pipeline;
mod camera;
mod render_volume;
mod key_state;

use std::io;
use std::net::SocketAddr;

use client::ClientMode;
use game::Game;
use common::get_version;

fn main() {
    println!("Starting Voxygen... Version: {}", get_version());

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

    println!("Binding local port to {}:{}...", ip.to_string(), port);

    let mut remote_addr = String::new();
    println!("Remote server address [127.0.0.1:59003]:");
    io::stdin().read_line(&mut remote_addr).unwrap();
    remote_addr = remote_addr.trim().to_string();
    if remote_addr.len() == 0 {
        remote_addr = "127.0.0.1:59003".to_string();
    }

    let game = Game::new(
        ClientMode::Character,
        &"voxygen-test",
        SocketAddr::new(ip, port),
        remote_addr
    ).run();
}
