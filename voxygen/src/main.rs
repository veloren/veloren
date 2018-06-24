#![feature(nll)]

#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate get_if_addrs;
#[macro_use] extern crate enum_map;
extern crate nalgebra;
extern crate time;
#[macro_use] extern crate coord;
extern crate dot_vox;

extern crate client;
extern crate common;
extern crate region;

extern crate pretty_env_logger;
#[macro_use] extern crate log;

mod game;
mod window;
mod renderer;
mod mesh;
mod model_object;
mod pipeline;
mod camera;
mod render_volume;
mod key_state;
mod map;
mod vox;

use std::io;
use std::net::SocketAddr;

use client::ClientMode;
use game::Game;
use common::get_version;

fn main() {
    pretty_env_logger::init();

    info!("Starting Voxygen... Version: {}", get_version());

    let mut remote_addr = String::new();
    println!("Remote server address [127.0.0.1:59003] (use m for testserver):");
    io::stdin().read_line(&mut remote_addr).unwrap();
    let mut remote_addr = remote_addr.trim();
    if remote_addr.len() == 0 {
        remote_addr = "127.0.0.1:59003";
    } else if remote_addr == "m" {
        remote_addr = "91.67.21.222:38888";
    }

    let name_seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_micros();
    Game::new(
        ClientMode::Character,
        common::names::generate(),
        remote_addr
    ).run();
}
