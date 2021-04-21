//! This crate contains the [`State`] and shared between
//! server (`veloren-server`) and the client (`veloren-client`)

mod build_areas;
#[cfg(feature = "plugins")] pub mod plugin;
mod state;
// TODO: breakup state module and remove glob
pub use build_areas::{BuildAreaError, BuildAreas};
pub use state::{BlockChange, State, TerrainChanges};
