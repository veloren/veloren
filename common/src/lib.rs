#![feature(nll)]

#[macro_use]
extern crate log;
extern crate time;

mod clock;

// Reexports
pub use clock::Clock;
