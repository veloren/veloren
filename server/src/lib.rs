#![feature(nll, extern_prelude, box_syntax)]

extern crate time;
extern crate common;
extern crate world;
extern crate region;
extern crate nalgebra;
extern crate bifrost;
extern crate toml;
extern crate serde;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;
#[macro_use] extern crate pretty_env_logger;

extern crate byteorder;

pub mod server;
mod server_context;
mod init;
mod config;
mod player;
mod session;
mod network;

pub use server::*;
