#![feature(nll)]

#[macro_use]
extern crate log;
extern crate time;
extern crate noise;

mod clock;
mod random_names;

// Reexports
pub use clock::Clock;

pub use random_names::NameGenerator;

pub type Uid = u64;

const CARGO_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");

pub fn get_version() -> String {
    CARGO_VERSION.unwrap_or("UNKNOWN VERSION").to_string()
}
