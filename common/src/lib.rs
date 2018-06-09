#![feature(nll)]

extern crate bincode;
extern crate get_if_addrs;
#[macro_use]
extern crate log;
extern crate nalgebra;
extern crate noise;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate time;

// Reexports
pub use clock::Clock;
pub use random_names::NameGenerator;

mod clock;
mod random_names;
pub mod network;

pub type Uid = u64;

const CARGO_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

pub fn get_version() -> String {
    CARGO_VERSION.unwrap_or("UNKNOWN VERSION").to_string()
}
