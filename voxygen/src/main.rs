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
extern crate time;

extern crate client;
extern crate common;
extern crate region;

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

fn main() {
    println!("Starting Voxygen...");

    let ip = std::net::IpAddr::V4(std::net::Ipv4Addr::new(0,0,0,0));
    let mut port = String::new();
    println!("Local port [autodetect-59001]:");
    io::stdin().read_line(&mut port).unwrap();
    let mut port = port.trim();
    if port.len() == 0 {
        port = "59001";
    }
    let port = u16::from_str_radix(&port.trim(), 10).unwrap();

    println!("Binding local port to {}:{}...", ip.to_string(), port);

    let mut remote_addr = String::new();
    println!("Remote server address [127.0.0.1:59003]:");
    io::stdin().read_line(&mut remote_addr).unwrap();
    let mut remote_addr = remote_addr.trim();
    if remote_addr.len() == 0 {
        remote_addr = "127.0.0.1:59003";
    }

    let name_seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_micros();
    let game = Game::new(
        ClientMode::Character,
        &common::NameGenerator::new(name_seed).get(),
        SocketAddr::new(ip, port),
        remote_addr
    ).run();
}
