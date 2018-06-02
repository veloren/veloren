#[macro_use]
extern crate gfx;
#[macro_use]
extern crate gfx_macros;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate local_ip;

extern crate client;
extern crate region;

mod game;
mod window;
mod renderer;
mod mesh;
mod vertex_buffer;
mod pipeline;

// Reexports
use game::GameHandle;
use window::RenderWindow;

fn main() {
    println!("Starting Voxygen...");

    let game = GameHandle::new(&"test-player");
    while game.next_frame() {}
}
