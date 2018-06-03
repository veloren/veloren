#![feature(nll)]

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate get_if_addrs;
#[macro_use]
extern crate enum_map;
#[macro_use]
extern crate nalgebra;

extern crate client;
extern crate region;

mod game;
mod window;
mod renderer;
mod mesh;
mod vertex_buffer;
mod pipeline;
mod camera;

use std::io;
use std::net::SocketAddr;

use client::ClientMode;
use game::Game;

fn main() {
    println!("Starting Voxygen...");

    // TODO: Seriously? This needs to go. Make it auto-detect this stuff
    // <rubbish>
    let ip = get_if_addrs::get_if_addrs().unwrap()[0].ip();

    let mut port = String::new();
    println!("Local port [59001]:");
    io::stdin().read_line(&mut port).unwrap();
    let port = u16::from_str_radix(&port.trim(), 10).unwrap();

    println!("Binding to {}:{}...", ip.to_string(), port);

    let mut remote_addr = String::new();
    println!("Remote server address:");
    io::stdin().read_line(&mut remote_addr).unwrap();
    // </rubbish>

    let game = Game::new(
        ClientMode::Player,
        &"voxygen-test",
        SocketAddr::new(ip, port),
        remote_addr.trim()
    ).run();
}
