//! This crate contains the [`State`] and shared between
//! server (`veloren-server`) and the client (`veloren-client`)

#![feature(maybe_uninit_uninit_array, maybe_uninit_array_assume_init)]

#[cfg(feature = "plugins")] pub mod plugin;
mod special_areas;
mod state;
// TODO: breakup state module and remove glob
pub use special_areas::*;
pub use state::{BlockChange, BlockDiff, ScheduledBlockChange, State, TerrainChanges};
