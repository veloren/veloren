//! This crate contains the [`State`] and shared between
//! server (`veloren-server`) and the client (`veloren-client`)

#[cfg(feature = "plugins")] pub mod plugin;
mod state;

// TODO: breakup state module and remove globs
pub use state::*;
