extern crate worldgen;
extern crate worldsim;

use worldgen::MacroWorld;

fn server() {
    let mut mw = MacroWorld::new(1337, 1024);
    worldsim::simulate(&mut mw, 1);
}
