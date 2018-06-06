#![feature(nll)]

#[macro_use]
extern crate log;
extern crate time;

mod clock;

// Reexports
pub use clock::Clock;

pub type Uid = u64;

const CARGO_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

pub fn get_version() -> String {
    CARGO_VERSION.unwrap_or("UNKNOWN VERSION").to_string()
}