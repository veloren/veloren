//! This crate contains the [`State`] and shared between
//! server (`nova-forge-server`) and the client (`nova-forge-client`)

#[cfg(feature = "plugins")] pub mod plugin;
mod special_areas;
mod state;
// TODO: breakup state module and remove glob
pub use special_areas::*;
pub use state::{BlockChange, BlockDiff, ScheduledBlockChange, State, TerrainChanges};
