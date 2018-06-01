#[macro_use]
extern crate gfx;
extern crate gfx_window_glutin;
extern crate glutin;
extern crate local_ip;

extern crate client;

mod game;
mod window;

// Reexports
use game::GameHandle as GameHandle;
use window::RenderWindow as RenderWindow;

fn main() {
    println!("Starting Voxygen...");

    let game = GameHandle::new(&"test-player");
    while game.next_frame() {}
}
