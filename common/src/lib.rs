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
