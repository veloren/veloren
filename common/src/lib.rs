#![feature(nll)]

extern crate bincode;
extern crate get_if_addrs;
#[macro_use]
extern crate log;
extern crate nalgebra;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate time;
extern crate byteorder;
extern crate rand;
#[macro_use]
extern crate coord;
#[macro_use]
extern crate lazy_static;

// Reexports
pub use clock::Clock;

pub mod clock;
pub mod names;
//pub mod network;
pub mod net;

pub type Uid = u64;

const CARGO_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

pub fn get_version() -> String {
    CARGO_VERSION.unwrap_or("UNKNOWN VERSION").to_string()
}
