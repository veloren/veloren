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
extern crate euler;

extern crate client;
extern crate region;

mod game;
mod window;
mod renderer;
mod mesh;
mod vertex_buffer;
mod pipeline;
mod camera;

use game::GameHandle;

fn main() {
    println!("Starting Voxygen...");

    let game = GameHandle::new(&"test-player");
    while game.next_frame() {}
}
