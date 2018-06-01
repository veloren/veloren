extern crate client;
extern crate local_ip;

mod game;

use game::GameHandle;

fn main() {
    println!("Starting Voxygen...");

    let game = GameHandle::new(&"test-player");
    while game.next_frame() {}
}
