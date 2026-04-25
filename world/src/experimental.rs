//! Nova-Forge Track B (Experimental) world generation parameters.
//!
//! When `WorldOpts::experimental` is `Some(...)`, the world sim pipeline uses
//! these values in place of the hardcoded upstream defaults.
//!
//! # Starter-planet design goal
//!
//! The experimental preset (`nova_forge_v1`) generates a world 4× the area of
//! the standard Track A world (2× per dimension: `x_lg = y_lg = 11` vs. the
//! default 10).  `continent_scale` (controlled by `gen_opts.scale`) is doubled
//! at the same time so that terrain features remain visually identical to the
//! current landscape — mountains look like mountains, river valleys look like
//! river valleys — just spread across the larger canvas.  The result is a world
//! large enough that the curvature of the planet is imperceptible during normal
//! on-foot or mounted travel.

use crate::sim::GenOpts;
use common::resources::MapKind;
use serde::{Deserialize, Serialize};

/// The locked world seed for the Nova-Forge Starter Planet (Track B).
///
/// Once a suitable seed is chosen (by running generation and evaluating the
/// resulting terrain), this constant must be updated and committed.  From that
/// point on every fresh experimental world will be generated with exactly this
/// seed, guaranteeing that all players share an identical planet.
///
/// **Placeholder:** currently `0`.  Replace with the chosen seed before launch.
///
/// # How it is applied
/// * When the player switches a world to "Experimental (Starter Planet)" track,
///   `world.seed` is automatically overwritten with this value.
/// * The seed field in the world-creation UI is hidden for experimental worlds;
///   it is not user-configurable.
/// * `singleplayer::SingleplayerWorlds::new_world()` keeps `DEFAULT_WORLD_SEED`
///   as the default since new worlds default to Track A (stable).  The seed is
///   overwritten to `STARTER_PLANET_SEED` only when the experimental toggle is
///   engaged via `WorldChange::Experimental(true)`.
// TODO: replace 0 with the chosen starter-planet seed once identified.
pub const STARTER_PLANET_SEED: u32 = 0;

/// Parameters that modify the Track B generation pipeline.
///
/// All fields are multipliers or offsets relative to the upstream baseline
/// so that `ExperimentalParams::default()` produces identical output to Track A.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentalParams {
    /// Multiplier applied to the mountain uplift heightmap (default 1.0 = no change).
    /// Values > 1.0 produce taller, more dramatic mountain ranges.
    pub mountain_scale: f32,

    /// Sea-level fraction offset in the range [-0.5, 0.5] (default 0.0).
    ///
    /// Shifts the ocean/land altitude boundary.  Positive values raise effective
    /// sea level (more ocean coverage), negative values expose more land.
    /// This is applied directly to the uniform-noise altitude values before the
    /// ocean mask is computed, so it changes coastlines without disturbing the
    /// erosion or mountain-scale calculations.
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

    /// Optional world generation option overrides for the experimental pipeline.
    ///
    /// When `Some`, these values replace the `GenOpts` derived from the
    /// `FileOpts` when the world has not yet been generated (i.e. is being
    /// created fresh).  This is the primary mechanism by which the starter-
    /// planet preset specifies a larger-than-default world and its matching
    /// terrain scale.
    ///
    /// Fields that govern the *size* and *terrain density*:
    /// * `x_lg` / `y_lg` — world size in chunks as base-2 logarithm.
    ///   Default world: 10/10 (1024×1024 chunks ≈ 33 km²).
    ///   Starter planet:  11/11 (2048×2048 chunks ≈ 66 km², 4× area).
    /// * `scale` — continent scale multiplier fed into noise frequencies.
    ///   Doubling `scale` alongside the world size keeps terrain feature density
    ///   visually identical to Track A (mountains still look like mountains).
    /// * `erosion_quality` — fraction of the default erosion step count.
    ///   Reduced for the larger world to keep generation time manageable.
    /// * `map_kind` — map shape (`Circle` produces an island/planet continent).
    #[serde(default)]
    pub gen_opts_override: Option<GenOpts>,
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
            gen_opts_override: None,
        }
    }
}

impl ExperimentalParams {
    /// Returns the starter-planet preset for Nova-Forge Track B:
    ///
    /// * **4× world area** — `x_lg = y_lg = 11` (2048×2048 chunks, ≈ 66 km per side).
    ///   At normal running speed it takes hours of real-time to cross the world,
    ///   making planetary curvature completely imperceptible in everyday play.
    /// * **Doubled continent scale** — `scale = 4.0` (up from the Track A default of
    ///   2.0).  Because noise frequencies are divided by `continent_scale`, this
    ///   keeps every terrain feature — mountain ranges, river valleys, plains —
    ///   visually identical to the existing landscape, just spread over the
    ///   larger canvas.
    /// * **Circle map kind** — produces an island/continent shape surrounded by
    ///   deep ocean, reinforcing the "planet" aesthetic.
    /// * **Reduced erosion quality (0.75)** — 75 erosion steps instead of 100,
    ///   keeping generation time roughly proportional to the standard world
    ///   despite the 4× tile count increase.  River definition is still
    ///   excellent at this quality level.
    /// * **Slightly raised sea level (+0.03)** — a modest amount of extra ocean
    ///   gives the planet a more Earth-like land/water ratio.
    /// * **Warmer & wetter (+0.05 each)** — biases toward lush, habitable
    ///   climates befitting the "starter planet" theme.
    /// * Nova biome rules enabled (no-op until the biome system is extended,
    ///   but flags the world for future Track B biome overrides).
    pub fn nova_forge_v1() -> Self {
        Self {
            mountain_scale: 1.0,
            sea_level_offset: 0.03,
            extra_erosion_passes: 0,
            temperature_bias: 0.05,
            humidity_bias: 0.05,
            nova_biome_rules: true,
            gen_opts_override: Some(GenOpts {
                x_lg: 11,
                y_lg: 11,
                scale: 4.0,
                map_kind: MapKind::Circle,
                erosion_quality: 0.75,
            }),
        }
    }
}
