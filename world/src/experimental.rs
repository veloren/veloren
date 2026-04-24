//! Nova-Forge Track B (Experimental) world generation parameters.
//!
//! When `WorldOpts::experimental` is `Some(...)`, the world sim pipeline uses
//! these values in place of the hardcoded upstream defaults.

use serde::{Deserialize, Serialize};

/// Parameters that modify the Track B generation pipeline.
///
/// All fields are multipliers or offsets relative to the upstream baseline
/// so that `ExperimentalParams::default()` produces identical output to Track A.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalParams {
    /// Multiplier applied to the mountain uplift heightmap (default 1.0 = no change).
    /// Values > 1.0 produce taller, more dramatic mountain ranges.
    pub mountain_scale: f32,

    /// Sea-level fraction offset in the range [-0.2, 0.2] (default 0.0).
    /// Positive values raise sea level (more ocean), negative values lower it.
    pub sea_level_offset: f32,

    /// Number of extra erosion passes on top of the baseline (default 0).
    /// Each extra pass increases river definition and valley depth.
    pub extra_erosion_passes: u32,

    /// Temperature bias added to every chunk's raw temperature sample (default 0.0).
    /// Range [-1.0, 1.0]. Positive = warmer world (more desert/tropical), negative = colder.
    pub temperature_bias: f32,

    /// Humidity bias added to every chunk's raw humidity sample (default 0.0).
    /// Range [-1.0, 1.0]. Positive = wetter world (more jungle/swamp), negative = drier.
    pub humidity_bias: f32,

    /// Whether Nova-Forge-specific biome placement rules are active (Track B biome logic).
    /// When false, falls back to upstream biome assignment even if experimental is enabled.
    pub nova_biome_rules: bool,
}

impl Default for ExperimentalParams {
    fn default() -> Self {
        Self {
            mountain_scale: 1.0,
            sea_level_offset: 0.0,
            extra_erosion_passes: 0,
            temperature_bias: 0.0,
            humidity_bias: 0.0,
            nova_biome_rules: false,
        }
    }
}

impl ExperimentalParams {
    /// Returns a preset tuned for the first Nova-Forge Track B world feel:
    /// - 20 % taller mountains
    /// - 2 extra erosion passes (more defined rivers)
    /// - slightly warmer & wetter
    /// - Nova biome rules enabled
    pub fn nova_forge_v1() -> Self {
        Self {
            mountain_scale: 1.2,
            sea_level_offset: 0.02,
            extra_erosion_passes: 2,
            temperature_bias: 0.05,
            humidity_bias: 0.05,
            nova_biome_rules: true,
        }
    }
}
