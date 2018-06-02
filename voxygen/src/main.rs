#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate gfx_device_gl;
extern crate glutin;
extern crate local_ip;

extern crate client;

mod game;
mod window;
mod renderer;
mod mesh;

// Reexports
use game::GameHandle;
use renderer::Renderer;

fn main() {
    println!("Starting Voxygen...");

    let game = GameHandle::new(&"test-player");
    while game.next_frame() {}
}
