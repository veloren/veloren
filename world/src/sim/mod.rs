mod diffusion;
mod erosion;
mod location;
mod map;
mod settlement;
mod util;

// Reexports
pub use self::diffusion::diffusion;
use self::erosion::Compute;
pub use self::erosion::{
    do_erosion, fill_sinks, get_drainage, get_lakes, get_multi_drainage, get_multi_rec, get_rivers,
    mrec_downhill, Alt, RiverData, RiverKind,
};
pub use self::location::Location;
pub use self::map::{MapConfig, MapDebug};
pub use self::settlement::Settlement;
pub use self::util::{
    cdf_irwin_hall, downhill, get_oceans, local_cells, map_edge_factor, neighbors,
    uniform_idx_as_vec2, uniform_noise, uphill, vec2_as_uniform_idx, HybridMulti as HybridMulti_,
    InverseCdf, ScaleBias, NEIGHBOR_DELTA,
};

use crate::{
    all::ForestKind,
    block::BlockGen,
    column::ColumnGen,
    generator::TownState,
    util::{seed_expan, FastNoise, RandomField, Sampler, StructureGen2d},
    CONFIG,
};
use common::{
    assets,
    terrain::{BiomeKind, TerrainChunkSize},
    vol::RectVolSize,
};
use hashbrown::HashMap;
use noise::{
    BasicMulti, Billow, Fbm, HybridMulti, MultiFractal, NoiseFn, RangeFunction, RidgedMulti,
    Seedable, SuperSimplex, Worley,
};
use num::{Float, Signed};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use rayon::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::{
    f32, f64,
    fs::File,
    io::{BufReader, BufWriter},
    ops::{Add, Div, Mul, Neg, Sub},
    path::PathBuf,
    sync::Arc,
};
use vek::*;

// NOTE: I suspect this is too small (1024 * 16 * 1024 * 16 * 8 doesn't fit in an i32), but we'll see
// what happens, I guess!  We could always store sizes >> 3.  I think 32 or 64 is the absolute
// limit though, and would require substantial changes.  Also, 1024 * 16 * 1024 * 16 is no longer
// cleanly representable in f32 (that stops around 1024 * 4 * 1024 * 4, for signed floats anyway)
// but I think that is probably less important since I don't think we actually cast a chunk id to
// float, just coordinates... could be wrong though!
pub const WORLD_SIZE: Vec2<usize> = Vec2 {
    x: 1024 * 1,
    y: 1024 * 1,
};

/// A structure that holds cached noise values and cumulative distribution functions for the input
/// that led to those values.  See the definition of InverseCdf for a description of how to
/// interpret the types of its fields.
struct GenCdf {
    humid_base: InverseCdf,
    temp_base: InverseCdf,
    chaos: InverseCdf,
    alt: Box<[Alt]>,
    basement: Box<[Alt]>,
    water_alt: Box<[f32]>,
    dh: Box<[isize]>,
    /// NOTE: Until we hit 4096 × 4096, this should suffice since integers with an absolute value
    /// under 2^24 can be exactly represented in an f32.
    flux: Box<[Compute]>,
    pure_flux: InverseCdf<Compute>,
    alt_no_water: InverseCdf,
    rivers: Box<[RiverData]>,
}

pub(crate) struct GenCtx {
    pub turb_x_nz: SuperSimplex,
    pub turb_y_nz: SuperSimplex,
    pub chaos_nz: RidgedMulti,
    pub alt_nz: HybridMulti_,
    pub hill_nz: SuperSimplex,
    pub temp_nz: Fbm,
    // Humidity noise
    pub humid_nz: Billow,
    // Small amounts of noise for simulating rough terrain.
    pub small_nz: BasicMulti,
    pub rock_nz: HybridMulti,
    pub cliff_nz: HybridMulti,
    pub warp_nz: FastNoise,
    pub tree_nz: BasicMulti,

    pub cave_0_nz: SuperSimplex,
    pub cave_1_nz: SuperSimplex,

    pub structure_gen: StructureGen2d,
    pub region_gen: StructureGen2d,
    pub cliff_gen: StructureGen2d,

    pub fast_turb_x_nz: FastNoise,
    pub fast_turb_y_nz: FastNoise,

    pub town_gen: StructureGen2d,
    // pub loc_gen: StructureGen2d,
    pub river_seed: RandomField,
    pub rock_strength_nz: Fbm,
    pub uplift_nz: Worley,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum FileOpts {
    /// If set, generate the world map and do not try to save to or load from file
    /// (default).
    Generate,
    /// If set, generate the world map and save the world file (path is created
    /// the same way screenshot paths are).
    Save,
    /// If set, load the world file from this path in legacy format (errors if
    /// path not found).  This option may be removed at some point, since it only applies to maps
    /// generated before map saving was merged into master.
    LoadLegacy(PathBuf),
    /// If set, load the world file from this path (errors if path not found).
    Load(PathBuf),
    /// If set, look for  the world file at this asset specifier (errors if asset is not found).
    ///
    /// NOTE: Could stand to merge this with `Load` and construct an enum that can handle either a
    /// PathBuf or an asset specifier, at some point.
    LoadAsset(String),
}

impl Default for FileOpts {
    fn default() -> Self {
        Self::Generate
    }
}

pub struct WorldOpts {
    /// Set to false to disable seeding elements during worldgen.
    pub seed_elements: bool,
    pub world_file: FileOpts,
}

impl Default for WorldOpts {
    fn default() -> Self {
        Self {
            seed_elements: true,
            world_file: Default::default(),
        }
    }
}

/// LEGACY: Remove when people stop caring.
#[derive(Serialize, Deserialize)]
#[repr(C)]
pub struct WorldFileLegacy {
    /// Saved altitude height map.
    pub alt: Box<[Alt]>,
    /// Saved basement height map.
    pub basement: Box<[Alt]>,
}

/// Version of the world map intended for use in Veloren 0.5.0.
#[derive(Serialize, Deserialize)]
#[repr(C)]
pub struct WorldMap_0_5_0 {
    /// Saved altitude height map.
    pub alt: Box<[Alt]>,
    /// Saved basement height map.
    pub basement: Box<[Alt]>,
}

/// Errors when converting a map to the most recent type (currently,
/// shared by the various map types, but at some point we might switch to
/// version-specific errors if it feels worthwhile).
#[derive(Debug)]
pub enum WorldFileError {
    /// Map size was invalid, and it can't be converted to a valid one.
    WorldSizeInvalid,
}

/// WORLD MAP.
///
/// A way to store certain components between runs of map generation.  Only intended for
/// development purposes--no attempt is made to detect map invalidation or make sure that the map
/// is synchronized with updates to noise-rs, changes to other parameters, etc.
///
/// The map is verisoned to enable format detection between versions of Veloren, so that when we
/// update the map format we don't break existing maps (or at least, we will try hard not to break
/// maps between versions; if we can't avoid it, we can at least give a reasonable error message).
///
/// NOTE: We rely somemwhat heavily on the implementation specifics of bincode to make sure this is
/// backwards compatible.  When adding new variants here, Be very careful to make sure tha the old
/// variants are preserved in the correct order and with the correct names and indices, and make
/// sure to keep the #[repr(u32)]!
///
/// All non-legacy versions of world files should (ideally) fit in this format.  Since the format
/// contains a version and is designed to be extensible backwards-compatibly, the only
/// reason not to use this forever would be if we decided to move away from BinCode, or
/// store data across multiple files (or something else weird I guess).
///
/// Update this when you add a new map version.
#[derive(Serialize, Deserialize)]
#[repr(u32)]
pub enum WorldFile {
    Veloren0_5_0(WorldMap_0_5_0) = 0,
}

/// Data for the most recent map type.  Update this when you add a new map verson.
pub type ModernMap = WorldMap_0_5_0;

/// The default world map.
///
/// TODO: Consider using some naming convention to automatically change this
/// with changing versions, or at least keep it in a constant somewhere that's
/// easy to change.
pub const DEFAULT_WORLD_MAP: &'static str = "world.map.veloren_0_5_0_0";

impl WorldFileLegacy {
    #[inline]
    /// Idea: each map type except the latest knows how to transform
    /// into the the subsequent map version, and each map type including the
    /// latest exposes an "into_modern()" method that converts this map type
    /// to the modern map type.  Thus, to migrate a map from an old format to a new
    /// format, we just need to transform the old format to the subsequent map
    /// version, and then call .into_modern() on that--this should construct a call chain that
    /// ultimately ends up with a modern version.
    pub fn into_modern(self) -> Result<ModernMap, WorldFileError> {
        if self.alt.len() != self.basement.len()
            || self.alt.len() != WORLD_SIZE.x as usize * WORLD_SIZE.y as usize
        {
            return Err(WorldFileError::WorldSizeInvalid);
        }

        /* let f = |h| h;// / 4.0;
        let mut map = map;
        map.alt.par_iter_mut()
            .zip(map.basement.par_iter_mut())
            .for_each(|(mut h, mut b)| {
                *h = f(*h);
                *b = f(*b);
            }); */

        let map = WorldMap_0_5_0 {
            alt: self.alt,
            basement: self.basement,
        };

        map.into_modern()
    }
}

impl WorldMap_0_5_0 {
    #[inline]
    pub fn into_modern(self) -> Result<ModernMap, WorldFileError> {
        if self.alt.len() != self.basement.len()
            || self.alt.len() != WORLD_SIZE.x as usize * WORLD_SIZE.y as usize
        {
            return Err(WorldFileError::WorldSizeInvalid);
        }

        Ok(self)
    }
}

impl WorldFile {
    /// Turns map data from the latest version into a versioned WorldFile ready for serialization.
    /// Whenever a new map is updated, just change the variant we construct here to make sure we're
    /// using the latest map version.

    pub fn new(map: ModernMap) -> Self {
        WorldFile::Veloren0_5_0(map)
    }

    #[inline]
    /// Turns a WorldFile into the latest version.  Whenever a new map version is added, just add
    /// it to this match statement.
    pub fn into_modern(self) -> Result<ModernMap, WorldFileError> {
        match self {
            WorldFile::Veloren0_5_0(map) => map.into_modern(),
        }
    }
}

pub struct WorldSim {
    pub seed: u32,
    pub(crate) chunks: Vec<SimChunk>,
    pub(crate) locations: Vec<Location>,

    pub(crate) gen_ctx: GenCtx,
    pub rng: ChaChaRng,
}

impl WorldSim {
    pub fn generate(seed: u32, opts: WorldOpts) -> Self {
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
        let continent_scale = 1.0/*4.0*/
            * 5_000.0f64 /*32768.0*/
                .div(32.0)
                .mul(TerrainChunkSize::RECT_SIZE.x as f64);
        let rock_lacunarity = /*0.5*/2.0/*HybridMulti::DEFAULT_LACUNARITY*/;
        let uplift_scale = /*512.0*//*256.0*/128.0;
        let uplift_turb_scale = uplift_scale / 4.0/*32.0*//*64.0*/;

        // NOTE: Changing order will significantly change WorldGen, so try not to!
        let gen_ctx = GenCtx {
            turb_x_nz: SuperSimplex::new().set_seed(rng.gen()),
            turb_y_nz: SuperSimplex::new().set_seed(rng.gen()),
            chaos_nz: RidgedMulti::new()
                .set_octaves(/*7*//*3*/ /*7*//*3*/7)
                .set_frequency(
                    RidgedMulti::DEFAULT_FREQUENCY * (5_000.0 / continent_scale)
                    // /*RidgedMulti::DEFAULT_FREQUENCY **/ 3_000.0 * 8.0 / continent_scale,
                )
                // .set_persistence(RidgedMulti::DEFAULT_LACUNARITY.powf(-(1.0 - 0.5)))
                .set_seed(rng.gen()),
            hill_nz: SuperSimplex::new().set_seed(rng.gen()),
            alt_nz: HybridMulti_::new()
                .set_octaves(/*3*//*2*/ /*8*//*3*/8)
                // 1/2048*32*1024 = 16
                .set_frequency(
                    /*HybridMulti::DEFAULT_FREQUENCY*/
                    // (2^8*(10000/5000/10000))*32 = per-chunk
                    (10_000.0/* * 2.0*/ / continent_scale) as f64,
                )
                // .set_frequency(1.0 / ((1 << 0) as f64))
                // .set_lacunarity(1.0)
                // persistence = lacunarity^(-(1.0 - fractal increment))
                .set_lacunarity(HybridMulti_::DEFAULT_LACUNARITY)
                .set_persistence(HybridMulti_::DEFAULT_LACUNARITY.powf(-(1.0 - /*0.75*/0.0)))
                // .set_persistence(/*0.5*//*0.5*/0.5 + 1.0 / ((1 << 6) as f64))
                // .set_offset(/*0.7*//*0.5*//*0.75*/0.7)
                .set_offset(/*0.7*//*0.5*//*0.75*/0.0)
                .set_seed(rng.gen()),
            //temp_nz: SuperSimplex::new().set_seed(rng.gen()),
            temp_nz: Fbm::new()
                .set_octaves(6)
                .set_persistence(0.5)
                // 1/2^14*1024*32 = 2
                // 1/(2^14-2^12)*1024*32 = 8/3 ~= 3
                .set_frequency(
                    /*4.0 / /*(1024.0 * 4.0/* * 8.0*/)*//*32.0*/((1 << 6) * (WORLD_SIZE.x)) as f64*/
                    1.0 / (((1 << 6) * 64) as f64),
                )
                // .set_frequency(1.0 / 1024.0)
                // .set_frequency(1.0 / (1024.0 * 8.0))
                .set_lacunarity(2.0)
                .set_seed(rng.gen()),

            small_nz: BasicMulti::new().set_octaves(2).set_seed(rng.gen()),
            rock_nz: HybridMulti::new().set_persistence(0.3).set_seed(rng.gen()),
            cliff_nz: HybridMulti::new().set_persistence(0.3).set_seed(rng.gen()),
            warp_nz: FastNoise::new(rng.gen()), //BasicMulti::new().set_octaves(3).set_seed(gen_seed()),
            tree_nz: BasicMulti::new()
                .set_octaves(12)
                .set_persistence(0.75)
                .set_seed(rng.gen()),
            cave_0_nz: SuperSimplex::new().set_seed(rng.gen()),
            cave_1_nz: SuperSimplex::new().set_seed(rng.gen()),

            structure_gen: StructureGen2d::new(rng.gen(), 32, 16),
            region_gen: StructureGen2d::new(rng.gen(), 400, 96),
            cliff_gen: StructureGen2d::new(rng.gen(), 80, 56),
            humid_nz: Billow::new()
                .set_octaves(9)
                .set_persistence(0.4)
                .set_frequency(0.2)
                // .set_octaves(6)
                // .set_persistence(0.5)
                .set_seed(rng.gen()),

            fast_turb_x_nz: FastNoise::new(rng.gen()),
            fast_turb_y_nz: FastNoise::new(rng.gen()),

            town_gen: StructureGen2d::new(rng.gen(), 2048, 1024),
            river_seed: RandomField::new(rng.gen()),
            rock_strength_nz: Fbm/*HybridMulti_*//*BasicMulti*//*Fbm*/::new()
            .set_octaves(/*6*//*5*//*4*//*5*//*4*//*6*/10)
            .set_lacunarity(rock_lacunarity)
            // persistence = lacunarity^(-(1.0 - fractal increment))
            // NOTE: In paper, fractal increment is roughly 0.25.
            // .set_offset(0.0)
            // .set_offset(0.7)
            .set_persistence(/*0.9*/ /*2.0*//*1.5*//*HybridMulti::DEFAULT_LACUNARITY*/rock_lacunarity.powf(-(1.0 - 0.25/*0.75*//*0.9*/)))
            // 256*32/2^4
            // (0.5^(-(1.0-0.9)))^4/256/32*2^4*16*32
            // (0.5^(-(1.0-0.9)))^4/256/32*2^4*256*4
            // (0.5^(-(1.0-0.9)))^1/256/32*2^4*256*4
            // (2^(-(1.0-0.9)))^4
            // 16.0
            .set_frequency(/*0.9*/ /*Fbm*//*HybridMulti_::DEFAULT_FREQUENCY*/1.0 * (5_000.0 / continent_scale) / (2.0/*8.0*//*256.0*//*1.0*//*16.0*/ * TerrainChunkSize::RECT_SIZE.x as f64/*4.0*//* TerrainChunkSize::RECT_SIZE.x as f64 */ * 2.0.powi(10 - 1)))
            // .set_frequency(/*0.9*/ /*Fbm*//*HybridMulti_::DEFAULT_FREQUENCY*/1.0 / (8.0/*8.0*//*256.0*//*1.0*//*16.0*/ * 32.0/*4.0*//* TerrainChunkSize::RECT_SIZE.x as f64 */ * 2.0.powi(10 - 1)))
            // .set_persistence(/*0.9*/ /*2.0*/0.67)
            // .set_frequency(/*0.9*/ Fbm::DEFAULT_FREQUENCY / (2.0 * 32.0))
            // .set_lacunarity(0.5)
            .set_seed(rng.gen()),
            uplift_nz: Worley::new()
                .set_seed(rng.gen())
                .set_frequency(1.0 / (TerrainChunkSize::RECT_SIZE.x as f64 * uplift_scale))
                // .set_displacement(/*0.5*/0.0)
                .set_displacement(/*0.5*/1.0)
                .set_range_function(RangeFunction::Euclidean),
                // .enable_range(true),
            // g_nz: RidgedMulti::new()
            // loc_gen: StructureGen2d::new(rng.gen(), 2048, 1024),
        };

        let river_seed = &gen_ctx.river_seed;
        let rock_strength_nz = &gen_ctx.rock_strength_nz;
        // NOTE: octaves should definitely fit into i32, but we should check anyway to make
        // sure.
        /* assert!(rock_strength_nz.persistence > 0.0);
        let rock_strength_scale = (1..rock_strength_nz.octaves as i32)
            .map(|octave| rock_strength_nz.persistence.powi(octave + 1))
            .sum::<f64>()
            // For some reason, this is "scaled" by 3.0.
            .mul(3.0);
        let rock_strength_nz = ScaleBias::new(&rock_strength_nz)
            .set_scale(1.0 / rock_strength_scale); */

        // Suppose the old world has grid spacing Δx' = Δy', new Δx = Δy.
        // We define grid_scale such that Δx = height_scale * Δx' ⇒
        //  grid_scale = Δx / Δx'.
        let grid_scale = 1.0f64 / 4.0/*1.0*/;

        // Now, suppose we want to generate a world with "similar" topography, defined in this case
        // as having roughly equal slopes at steady state, with the simulation taking roughly as
        // many steps to get to the point the previous world was at when it finished being
        // simulated.
        //
        // Some computations with our coupled SPL/debris flow give us (for slope S constant) the following
        // suggested scaling parameters to make this work:
        //   k_fs_scale ≡ (K𝑓 / K𝑓') = grid_scale^(-2m) = grid_scale^(-2θn)
        let k_fs_scale = |theta, n| grid_scale.powf(-2.0 * (theta * n) as f64);

        //   k_da_scale ≡ (K_da / K_da') = grid_scale^(-2q)
        let k_da_scale = |q| grid_scale.powf(-2.0 * q);
        //
        // Some other estimated parameters are harder to come by and *much* more dubious, not being accurate
        // for the coupled equation. But for the SPL only one we roughly find, for h the height at steady
        // state and time τ = time to steady state, with Hack's Law estimated b = 2.0 and various other
        // simplifying assumptions, the estimate:
        //   height_scale ≡ (h / h') = grid_scale^(n)
        let height_scale = |n: f32| grid_scale.powf(n as f64) as Alt;
        //   time_scale ≡ (τ / τ') = grid_scale^(n)
        let time_scale = |n: f32| grid_scale.powf(n as f64);
        //
        // Based on this estimate, we have:
        //   delta_t_scale ≡ (Δt / Δt') = time_scale
        let delta_t_scale = |n: f32| time_scale(n);
        //   alpha_scale ≡ (α / α') = height_scale^(-1)
        let alpha_scale = |n: f32| height_scale(n).recip() as f32;
        //
        // Slightly more dubiously (need to work out the math better) we find:
        //   k_d_scale ≡ (K_d / K_d') = grid_scale^2 / (height_scale * time_scale)
        let k_d_scale = |n: f32| /*grid_scale.powi(2) / time_scale(n)*//*height_scale(n)*/grid_scale.powi(2) / (/*height_scale(n) * */time_scale(n))/* * (1.0 / 16.0)*/;
        //   epsilon_0_scale ≡ (ε₀ / ε₀') = height_scale(n) / time_scale(n)
        let epsilon_0_scale = |n| /*height_scale(n) as f32*//*1.0*/(height_scale(n) / time_scale(n)) as f32/* * 1.0 / 4.0*/;

        // Approximate n for purposes of computation of parameters above over the whole grid (when
        // a chunk isn't available).
        let n_approx = 1.0;
        let max_erosion_per_delta_t = /*8.0*//*32.0*//*1.0*//*32.0*//*32.0*//*16.0*//*64.0*//*32.0*/64.0/*128.0*//*1.0*//*0.2 * /*100.0*/250.0*//*128.0*//*16.0*//*128.0*//*32.0*/ * delta_t_scale(n_approx);
        /* let erosion_pow_low = /*0.25*//*1.5*//*2.0*//*0.5*//*4.0*//*0.25*//*1.0*//*2.0*//*1.5*//*1.5*//*0.35*//*0.43*//*0.5*//*0.45*//*0.37*/1.002;
        let erosion_pow_high = /*1.5*//*1.0*//*0.55*//*0.51*//*2.0*/1.002; */
        let erosion_center = /*0.45*//*0.75*//*0.75*//*0.5*//*0.75*/0.5;
        let n_steps = /*200*//*10_000*//*1000*//*50*//*100*/100; //100; // /*100*//*50*//*100*//*100*//*50*//*25*/25/*100*//*37*/;//150;//37/*100*/;//50;//50;//37;//50;//37; // /*37*//*29*//*40*//*150*/37; //150;//200;
        let n_small_steps = 0; //25;//8;//50;//50;//8;//8;//8;//8;//8; // 8
        let n_post_load_steps = 0; //25;//8

        // Logistic regression.  Make sure x ∈ (0, 1).
        let logit = |x: f64| x.ln() - (-x).ln_1p();
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
        let logistic_2_base = 3.0f64.sqrt() * f64::consts::FRAC_2_PI;
        // let logistic_base = /*3.0f64.sqrt() * f64::consts::FRAC_1_PI*/1.0f64;
        // Assumes μ = 0, σ = 1
        let logistic_cdf = |x: f64| (x / logistic_2_base).tanh() * 0.5 + 0.5;

        /* let exp_inverse_cdf = |x: f64/*, pow: f64*/| -(-x).ln_1p()/* / ln(pow)*/;
        // 2 / pi * ln(tan(pi/2 * p))
        let hypsec_inverse_cdf =
            |x: f64| f64::consts::FRAC_2_PI * ((x * f64::consts::FRAC_PI_2).tan().ln()); */

        let min_epsilon =
            1.0 / (WORLD_SIZE.x as f64 * WORLD_SIZE.y as f64).max(f64::EPSILON as f64 * 0.5);
        let max_epsilon = (1.0 - 1.0 / (WORLD_SIZE.x as f64 * WORLD_SIZE.y as f64))
            .min(1.0 - f64::EPSILON as f64 * 0.5);

        // fractal dimension should be between 0 and 0.9999...
        // (lacunarity^octaves)^(-H) = persistence^(octaves)
        // lacunarity^(octaves*-H) = persistence^(octaves)
        // e^(-octaves*H*ln(lacunarity)) = e^(octaves * ln(persistence))
        // -octaves * H * ln(lacunarity) = octaves * ln(persistence)
        // -H = ln(persistence) / ln(lacunarity)
        // H = -ln(persistence) / ln(lacunarity)
        // ln(persistence) = -H * ln(lacunarity)
        // persistence = lacunarity^(-H)
        //
        // -ln(2^(-0.25))/ln(2) = 0.25
        //
        // -ln(2^(-0.1))/ln(2)
        //
        // 0 = -ln(persistence) / ln(lacunarity)
        // 0 = ln(persistence) => persistence = e^0 = 1
        //
        // 1 = -ln(persistence) / ln(lacunarity)
        // -ln(lacunarity) = ln(persistence)
        // e^(-ln(lacunarity)) = e^(ln(persistence))
        // 1 / lacunarity = persistence
        //
        // Ergo, we should not set fractal dimension to anything  not between 1 / lacunarity and 1.
        //
        // dimension = -ln(0.25)/ln(2*pi/3) = 1.875
        //
        // (2*pi/3^1)^(-(-ln(0.25)/ln(2*pi/3))) = 0.25
        //
        // Default should be at most 1 / lacunarity.
        //
        // (2 * pi / 3)^(-ln(0.25)/ln(2*pi/3))
        //
        // -ln(0.25)/ln(2*pi/3) = 1.88
        //
        // (2 * pi / 3)^(-ln(0.25)/ln(2*pi/3))
        //
        // 2 * pi / 3
        //
        // 2.0^(2(-ln(1.5)/ln(2)))
        // (1 / 1.5)^(2)

        // No NaNs in these uniform vectors, since the original noise value always returns Some.
        let ((alt_base, _), (chaos, _)) = rayon::join(
            || {
                uniform_noise(|_, wposf| {
                    // "Base" of the chunk, to be multiplied by CONFIG.mountain_scale (multiplied value
                    // is from -0.35 * (CONFIG.mountain_scale * 1.05) to
                    // 0.35 * (CONFIG.mountain_scale * 0.95), but value here is from -0.3675 to 0.3325).
                    Some(
                        (gen_ctx
                            .alt_nz
                            .get((wposf.div(10_000.0)).into_array())
                            .min(1.0)
                            .max(-1.0)/* .mul(0.25)
                        .add(0.125) */)
                        // .add(0.5)
                        .sub(0.05)
                        // .add(0.05)
                        // .add(0.075)
                        .mul(0.35), /*-0.0175*/
                    )
                })
            },
            || {
                uniform_noise(|_, wposf| {
                    // From 0 to 1.6, but the distribution before the max is from -1 and 1.6, so there is
                    // a 50% chance that hill will end up at 0.3 or lower, and probably a very high
                    // change it will be exactly 0.
                    let hill = (0.0f64
                        //.add(0.0)
                        + gen_ctx
                            .hill_nz
                            .get((wposf.mul(32.0).div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64)).div(1_500.0)).into_array())
                            .min(1.0)
                            .max(-1.0)
                            .mul(1.0)
                        + gen_ctx
                            .hill_nz
                            .get((wposf.mul(32.0).div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64)).div(400.0)).into_array())
                            .min(1.0)
                            .max(-1.0)
                            .mul(0.3))
                    .add(0.3)
                    .max(0.0);

                    // chaos produces a value in [0.12, 1.32].  It is a meta-level factor intended to
                    // reflect how "chaotic" the region is--how much weird stuff is going on on this
                    // terrain.
                    Some(
                        ((gen_ctx
                            .chaos_nz
                            .get((wposf.div(3_000.0)).into_array())
                            .min(1.0)
                            .max(-1.0))
                        .add(1.0)
                        .mul(0.5)
                        // [0, 1] * [0.4, 1] = [0, 1] (but probably towards the lower end)
                        //.mul(1.0)
                        .mul(
                            (gen_ctx
                                .chaos_nz
                                .get((wposf.div(6_000.0)).into_array())
                                .min(1.0)
                                .max(-1.0))
                            .abs()
                            .max(0.4)
                            .min(1.0),
                        )
                        // Chaos is always increased by a little when we're on a hill (but remember
                        // that hill is 0.3 or less about 50% of the time).
                        // [0, 1] + 0.2 * [0, 1.6] = [0, 1.32]
                        .add(0.2 * hill)
                        // We can't have *no* chaos!
                        .max(0.12)) as f32,
                    )
                })
            },
        );

        // We ignore sea level because we actually want to be relative to sea level here and want
        // things in CONFIG.mountain_scale units, but otherwise this is a correct altitude
        // calculation.  Note that this is using the "unadjusted" temperature.
        //
        // No NaNs in these uniform vectors, since the original noise value always returns Some.
        let (alt_old, /*alt_old_inverse*/ _) = uniform_noise(|posi, wposf| {
            // This is the extension upwards from the base added to some extra noise from -1 to
            // 1.
            //
            // The extra noise is multiplied by alt_main (the mountain part of the extension)
            // powered to 0.8 and clamped to [0.15, 1], to get a value between [-1, 1] again.
            //
            // The sides then receive the sequence (y * 0.3 + 1.0) * 0.4, so we have
            // [-1*1*(1*0.3+1)*0.4, 1*(1*0.3+1)*0.4] = [-0.52, 0.52].
            //
            // Adding this to alt_main thus yields a value between -0.4 (if alt_main = 0 and
            // gen_ctx = -1, 0+-1*(0*.3+1)*0.4) and 1.52 (if alt_main = 1 and gen_ctx = 1).
            // Most of the points are above 0.
            //
            // Next, we add again by a sin of alt_main (between [-1, 1])^pow, getting
            // us (after adjusting for sign) another value between [-1, 1], and then this is
            // multiplied by 0.045 to get [-0.045, 0.045], which is added to [-0.4, 0.52] to get
            // [-0.445, 0.565].
            let alt_main = {
                // Extension upwards from the base.  A positive number from 0 to 1 curved to be
                // maximal at 0.  Also to be multiplied by CONFIG.mountain_scale.
                let alt_main = (gen_ctx
                    .alt_nz
                    .get((wposf.div(2_000.0)).into_array())
                    .min(1.0)
                    .max(-1.0))
                .abs()
                // 0.5
                .powf(1.35);

                fn spring(x: f64, pow: f64) -> f64 {
                    x.abs().powf(pow) * x.signum()
                }

                (0.0 + alt_main/*0.4*/
                    + (gen_ctx
                        .small_nz
                        .get((wposf.mul(32.0).div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64)).div(300.0)).into_array())
                        .min(1.0)
                        .max(-1.0))
                    .mul(alt_main.powf(0.8).max(/*0.25*/ 0.15))
                    .mul(0.3)
                    .add(1.0)
                    .mul(0.4)
                    // 0.52
                    + spring(alt_main.abs().powf(0.5).min(0.75).mul(60.0).sin(), 4.0).mul(0.045))
            };

            // Now we can compute the final altitude using chaos.
            // We multiply by chaos clamped to [0.1, 1.32] to get a value between [0.03, 2.232]
            // for alt_pre, then multiply by CONFIG.mountain_scale and add to the base and sea
            // level to get an adjusted value, then multiply the whole thing by map_edge_factor
            // (TODO: compute final bounds).
            //
            // [-.3675, .3325] + [-0.445, 0.565] * [0.12, 1.32]^1.2
            // ~ [-.3675, .3325] + [-0.445, 0.565] * [0.07, 1.40]
            // = [-.3675, .3325] + ([-0.5785, 0.7345])
            // = [-0.946, 1.067]
            Some(
                ((alt_base[posi].1
                    + alt_main /*1.0*/
                        .mul(
                            (chaos[posi].1 as f64) /*.mul(2.0).sub(1.0).max(0.0)*/
                                .powf(1.2), /*0.25)*//*0.285*/
                        )/*0.1425*/)
                .mul(map_edge_factor(posi) as f64)
                .add(
                    (CONFIG.sea_level as f64)
                        .div(CONFIG.mountain_scale as f64)
                        .mul(map_edge_factor(posi) as f64),
                )
                .sub((CONFIG.sea_level as f64).div(CONFIG.mountain_scale as f64)))
                    as f32,
            )
            /* Some(
                // FIXME: May fail on big-endian platforms.
                ((alt_base[posi].1 as f64 + 0.5 + (/*alt_main./*to_le_bytes()[7]*/to_bits() & 1) as f64 * ((1.0 / CONFIG.mountain_scale as f64).powf(1.0 / erosion_pow_low)) + */alt_main / CONFIG.mountain_scale as f64 * 128.0).mul(0.1).powf(1.2))
                    .mul(map_edge_factor(posi) as f64)
                    .add(
                        (CONFIG.sea_level as f64)
                            .div(CONFIG.mountain_scale as f64)
                            .mul(map_edge_factor(posi) as f64),
                    )
                    .sub((CONFIG.sea_level as f64).div(CONFIG.mountain_scale as f64)))
                    as f32,
            ) */
        });

        // Calculate oceans.
        let is_ocean = get_oceans(|posi: usize| alt_old[posi].1);
        let is_ocean_fn = |posi: usize| is_ocean[posi];
        /* let is_ocean = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|i| map_edge_factor(i) == 0.0)
        .collect::<Vec<_>>(); */

        let turb_wposf_div = 8.0/*64.0*/;
        let n_func = |posi| {
            if is_ocean_fn(posi) {
                return 1.0;
            }
            /* let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
            .map(|e| e as f64); */
            /* let turb_wposf = wposf
                .mul(5_000.0 / continent_scale)
                .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                .div(turb_wposf_div);
            let turb = Vec2::new(
                gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
            ) * uplift_turb_scale
                * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
            // let turb = Vec2::zero();
            let turb_wposf = wposf + turb; */
            /* let turb_wposi = turb_wposf
                .div(5_000.0 / continent_scale)
                .map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as f64)
                .map2(WORLD_SIZE, |e, f| (e as i32).max(f as i32 - 1).min(0));
            let turb_posi = vec2_as_uniform_idx(turb_wposi); */
            /* let uheight = gen_ctx
            .uplift_nz
            .get(turb_wposf.into_array())
            /* .min(0.5)
            .max(-0.5)*/
            .min(1.0)
            .max(-1.0)
            .mul(0.5)
            .add(0.5); */
            /* if uheight > 0.8 {
                1.5
            } else {
                1.0
            } */
            // ((1.5 - 0.6) * uheight + 0.6) as f32
            // ((1.5 - 1.0) * uheight + 1.0) as f32
            // ((3.5 - 1.5) * (1.0 - uheight) + 1.5) as f32
            1.0
        };
        let old_height = |posi: usize| {
            alt_old[posi].1 * CONFIG.mountain_scale * height_scale(n_func(posi)) as f32
        };

        // let uplift_nz_dist = gen_ctx.uplift_nz.clone().enable_range(true);
        // Recalculate altitudes without oceans.
        // NaNs in these uniform vectors wherever is_ocean_fn returns true.
        let (alt_old_no_ocean, alt_old_inverse) = uniform_noise(|posi, _| {
            if is_ocean_fn(posi) {
                None
            } else {
                Some(old_height(posi) /*.abs()*/)
            }
        });
        let (uplift_uniform, _) = uniform_noise(|posi, _wposf| {
            if is_ocean_fn(posi) {
                None
            } else {
                /* let turb_wposf = wposf
                    .mul(5_000.0 / continent_scale)
                    .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                    .div(turb_wposf_div);
                let turb = Vec2::new(
                    gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                    gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
                ) * uplift_turb_scale
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
                // let turb = Vec2::zero();
                let turb_wposf = wposf + turb; */
                /* let turb_wposi = turb_wposf
                    .div(5_000.0 / continent_scale)
                    .map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as f64)
                    .map2(WORLD_SIZE, |e, f| (e as i32).max(f as i32 - 1).min(0));
                let turb_posi = vec2_as_uniform_idx(turb_wposi); */
                /* let udist = uplift_nz_dist
                    .get(turb_wposf.into_array())
                    .min(1.0)
                    .max(-1.0)
                    .mul(0.5)
                    .add(0.5);
                let uheight = gen_ctx
                    .uplift_nz
                    .get(turb_wposf.into_array())
                    /* .min(0.5)
                    .max(-0.5)*/
                    .min(1.0)
                    .max(-1.0)
                    .mul(0.5)
                    .add(0.5); */
                /* let uchaos = /* gen_ctx.chaos_nz.get((wposf.div(3_000.0)).into_array())
                        .min(1.0)
                        .max(-1.0)
                        .mul(0.5)
                        .add(0.5); */
                        chaos[posi].1;

                let uchaos_1 = (uchaos as f64) / 1.32; */

                let oheight = /*alt_old*//*alt_base*/alt_old_no_ocean[/*(turb_posi / 64) * 64*/posi].0 as f64 - 0.5;
                /* assert!(udist >= 0.0);
                assert!(udist <= 1.0);
                let uheight_1 = uheight; //.powf(2.0);
                let udist_1 = (0.5 - udist).mul(2.0).max(0.0);
                let udist_2 = udist.mul(2.0).min(1.0);
                let udist_3 = (1.0 - udist).max(0.0);
                let udist_4 = udist.min(1.0);
                let variation = 1.0.min(
                    64.0 * 64.0
                        / (WORLD_SIZE.x as f64
                            * WORLD_SIZE.y as f64
                            * (TerrainChunkSize::RECT_SIZE.x as f64
                                * TerrainChunkSize::RECT_SIZE.y as f64
                                / 128.0
                                / 128.0)),
                );
                let variation_1 = (uheight * /*udist_2*/udist_4).min(variation); */
                let height = (oheight + 0.5).powf(2.0);
                // 1.0 - variation + variation * uchaos_1;
                // uheight * /*udist_2*/udist_4 - variation_1 + variation_1 * uchaos_1;
                // uheight * (0.5 + 0.5 * ((uchaos as f64) / 1.32)) - 0.125;
                // 0.2;
                // 1.0;
                // uheight_1;
                // uheight_1 * (0.8 + 0.2 * oheight.signum() * oheight.abs().powf(0.25));
                // uheight_1 * (/*udist_2*/udist.powf(2.0) * (f64::consts::PI * 2.0 * (1.0 / (1.0 - udist).max(f64::EPSILON)).min(2.5)/*udist * 5.0*/ * 2.0).cos().mul(0.5).add(0.5));
                // uheight * udist_ * (udist_ * 4.0 * 2 * f64::consts::PI).sin()
                // uheight;
                // (0.8 * uheight + oheight.powf(2.0) * 0.2).max(0.0).min(1.0);
                // ((0.8 - 0.2) * uheight + 0.2 + oheight.signum() * oheight.abs().powf(/*0.5*/2.0) * udist_2.powf(2.0)).max(0.0).min(1.0);
                // ((0.8 - 0.2) * uheight + 0.2 + oheight.signum() * oheight.abs().powf(/*0.5*/2.0) * 0.2).max(0.0).min(1.0);
                // (0.8 * uheight * udist_1 + 0.8 * udist_2 + oheight.powf(2.0) * 0.2).max(0.0).min(1.0);
                /* uheight * 0.8 * udist_1.powf(2.0) +
                /*exp_inverse_cdf*/(oheight/*.max(0.0).min(max_epsilon).abs()*/).powf(2.0) * 0.2 * udist_2.powf(2.0); */
                // (uheight + oheight.powf(2.0) * 0.05).max(0.0).min(1.0);
                // (uheight + oheight.powf(2.0) * 0.2).max(0.0).min(1.0);
                // * (1.0 - udist);// uheight * (1.0 - udist)/*oheight*//* * udist*/ + oheight * udist;/*uheight * (1.0 - udist);*/
                // let height = uheight * (0.5 - udist) * 0.8 + (oheight.signum() * oheight.max(0.0).abs().powf(2.0)) * 0.2;// * (1.0 - udist);// uheight * (1.0 - udist)/*oheight*//* * udist*/ + oheight * udist;/*uheight * (1.0 - udist);*/
                Some(height)
            }
        });

        // let old_height_uniform = |posi: usize| alt_old_no_ocean[posi].0;
        let alt_old_min_uniform = 0.0;
        let alt_old_max_uniform = 1.0;
        //  let alt_old_center_uniform = erosion_center;
        let (_alt_old_min_index, _alt_old_min) = alt_old_inverse.first().unwrap();
        let (_alt_old_max_index, _alt_old_max) = alt_old_inverse.last().unwrap();
        let (_alt_old_mid_index, _alt_old_mid) =
            alt_old_inverse[(alt_old_inverse.len() as f64 * erosion_center) as usize];
        /* let alt_old_center =
        ((alt_old_mid - alt_old_min) as f64 / (alt_old_max - alt_old_min) as f64); */

        /* // Find the minimum and maximum original altitudes.
        // NOTE: Will panic if there is no land, and will not work properly if the minimum and
        // maximum land altitude are identical (will most likely panic later).
        let old_height_uniform = |posi: usize| alt_old[posi].0;
        let (alt_old_min_index, _alt_old_min) = alt_old_inverse
            .iter()
            .copied()
            .find(|&(_, h)| h > 0.0)
            .unwrap();
        let &(alt_old_max_index, _alt_old_max) = alt_old_inverse.last().unwrap();
        let alt_old_min_uniform = alt_old[alt_old_min_index].0;
        let alt_old_max_uniform = alt_old[alt_old_max_index].0; */

        // Perform some erosion.

        // 2^((2^10-2)/256) = 15.91...
        // -ln(1-(1-(2^(-22)*0.5)))
        // -ln(1-(1-(2^(-53)*0.5)))
        // ((-ln(1-((1-2^(-53)*0.5))))/ln(e))/((-ln(1-((2^(-53)*0.5))))/ln(e))
        // ((-ln(1-((0.5))))/ln(2))/((-ln(1-((1 - 2^(-53)*0.5))))/ln(2))
        // ((-ln(1-((0.5))))/ln(e))/((-ln(1-((1 - 2^(-53)*0.5))))/ln(e))
        // ((-ln(1-((0.5))))/ln(e))/((-ln(1-((2^(-53)*0.5))))/ln(e))
        // ((-ln(1-((1-2^(-53)))))/ln(1.002))/((-ln(1-((1 - 2^(-53)*0.5))))/ln(1+2^(-10*2)*0.5))
        // ((-ln(1-((0.9999999999999999))))/ln(e))/((-ln(1-((1 - 2^(-53)*0.5))))/ln(1+2^(-53)*0.5))
        //
        // ((-ln(1-((1-2^(-10*2)))))/ln(1.002))/((-ln(1-((1 - 2^(-10*2)))))/ln(1+2^(-9)))
        // ((-ln(1-((2^(-10*2)))))/ln(1.002))/((-ln(1-((1 - 2^(-10*2)))))/ln(1+2^(-9)))
        // ((-ln(1-((1-2^(-10*2)))))/ln(1.002))/((-ln(1-((1 - 2^(-10*2)))))/ln(1.002))
        // ((ln(0.6)-ln(1-0.6)) - (ln(1/(2048*2048))-ln((1-1/(2048*2048)))))/((ln(1-1/(2048*2048))-ln(1-(1-1/(2048*2048)))) - (ln(1/(2048*2048))-ln((1-1/(2048*2048)))))
        let inv_func = |x: f64| x/*exp_inverse_cdf*//*logit*//*hypsec_inverse_cdf*/;
        let alt_exp_min_uniform = /*exp_inverse_cdf*//*logit*/inv_func(min_epsilon);
        let alt_exp_max_uniform = /*exp_inverse_cdf*//*logit*/inv_func(max_epsilon);

        // let erosion_pow = 2.0;
        // let n_steps = 100;//150;
        // let erosion_factor = |x: f64| logistic_cdf(erosion_pow * logit(x));
        /* let log_odds = |x: f64| {
            logit(x)
                - logit(
                    /*erosion_center*/ alt_old_center_uniform, /*alt_old_center*/
                )
        }; */
        /* let erosion_factor = |x: f64| logistic_cdf(logistic_base * if x <= /*erosion_center*/alt_old_center_uniform/*alt_old_center*/ { erosion_pow_low.ln() } else { erosion_pow_high.ln() } * log_odds(x))/*0.5 + (x - 0.5).signum() * ((x - 0.5).mul(2.0).abs(
        ).powf(erosion_pow).mul(0.5))*/; */
        let erosion_factor = |x: f64| (/*if x <= /*erosion_center*/alt_old_center_uniform/*alt_old_center*/ { erosion_pow_low.ln() } else { erosion_pow_high.ln() } * */(/*exp_inverse_cdf*//*logit*/inv_func(x) - alt_exp_min_uniform) / (alt_exp_max_uniform - alt_exp_min_uniform))/*0.5 + (x - 0.5).signum() * ((x - 0.5).mul(2.0).abs(
).powf(erosion_pow).mul(0.5))*//*.powf(0.5)*//*.powf(1.5)*//*.powf(2.0)*/;
        let rock_strength_div_factor = /*8.0*/(2.0 * TerrainChunkSize::RECT_SIZE.x as f64) / 8.0;
        // let time_scale = 1.0; //4.0/*4.0*/;
        let theta_func = |_posi| 0.4;
        let kf_func = {
            |posi| {
                let kf_scale_i = k_fs_scale(theta_func(posi), n_func(posi)) as f64;
                // let precip_mul = (0.25).powf(m);
                if is_ocean_fn(posi) {
                    // multiplied by height_scale^(2m) to account for change in area.
                    return 1.0e-4 * kf_scale_i/* / time_scale*/; // .powf(-(1.0 - 2.0 * m_i))/* * 4.0*/;
                                                                 // return 2.0e-5;
                                                                 // return 2.0e-6;
                                                                 // return 2.0e-10;
                                                                 // return 0.0;
                }
                /* let wposf = (uniform_idx_as_vec2(posi)
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                .map(|e| e as f64); */
                /* let turb_wposf = wposf
                    .mul(5_000.0 / continent_scale)
                    .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                    .div(turb_wposf_div);
                let turb = Vec2::new(
                    gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                    gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
                ) * uplift_turb_scale
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
                // let turb = Vec2::zero();
                let turb_wposf = wposf + turb; */
                /* let turb_wposi = turb_wposf
                    .div(5_000.0 / continent_scale)
                    .map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as f64)
                    .map2(WORLD_SIZE, |e, f| (e as i32).max(f as i32 - 1).min(0));
                let turb_posi = vec2_as_uniform_idx(turb_wposi); */
                /* let uheight = gen_ctx
                .uplift_nz
                .get(turb_wposf.into_array())
                /* .min(0.5)
                .max(-0.5)*/
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5); */

                /* let uchaos = /* gen_ctx.chaos_nz.get((wposf.div(3_000.0)).into_array())
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5); */
                chaos[posi].1; */

                /* let oheight = /*alt_old*//*alt_base*/alt_old_no_ocean[/*(turb_posi / 64) * 64*/posi].0 as f64;
                let oheight_2 = /*alt_old*//*alt_base*/(alt_old_no_ocean[/*(turb_posi / 64) * 64*/posi].1 as f64 / CONFIG.mountain_scale as f64); */

                let kf_i = // kf = 1.5e-4: high-high (plateau [fan sediment])
                // kf = 1e-4: high (plateau)
                // kf = 2e-5: normal (dike [unexposed])
                // kf = 1e-6: normal-low (dike [exposed])
                // kf = 2e-6: low (mountain)
                // --
                // kf = 2.5e-7 to 8e-7: very low (Cordonnier papers on plate tectonics)
                // ((1.0 - uheight) * (1.5e-4 - 2.0e-6) + 2.0e-6) as f32
                //
                // ACTUAL recorded values worldwide: much lower...
                //
                // Or maybe not?  Getting something like 2e-3...
                //
                // ...or 8.345e5.
                // ((1.0 - uheight) * (5e-5 - 9.88e-15) + 9.88e-15)
                // ((1.0 - uheight) * (1.5e-4 - 9.88e-15) + 9.88e-15)
                // ((1.0 - uheight) * (8.345e5 - 2.0e-6) + 2.0e-6) as f32
                // ((1.0 - uheight) * (1.5e-4 - 2.0e-6) + 2.0e-6)
                // ((1.0 - uheight) * (0.5 + 0.5 * ((1.32 - uchaos as f64) / 1.32)) * (1.5e-4 - 2.0e-6) + 2.0e-6)
                // ((1.0 - uheight) * (0.5 + 0.5 * /*((1.32 - uchaos as f64) / 1.32)*/oheight) * (1.5e-4 - 2.0e-6) + 2.0e-6)
                // ((1.0 - uheight) * (0.5 - 0.5 * /*((1.32 - uchaos as f64) / 1.32)*/oheight_2) * (1.5e-4 - 2.0e-6) + 2.0e-6)
                // ((1.0 - uheight) * (0.5 - 0.5 * /*((1.32 - uchaos as f64) / 1.32)*/oheight) * (1.5e-4 - 2.0e-6) + 2.0e-6)
                // 2e-5
                // multiplied by height_scale^(2m) to account for change in area.
                // 2.5e-6/* / time_scale*//* / 4.0 * 0.25 *//* * 4.0*/
                1.0e-6
                // 2.0e-5 // 1.8e-4 to 2.2e-6
                // 1.0e-5 // 9.0e-5 to 1.1e-6
                // 2.0e-6
                // 2.9e-10
                // ((1.0 - uheight) * (5e-5 - 2.9e-10) + 2.9e-10)
                // ((1.0 - uheight) * (5e-5 - 2.9e-14) + 2.9e-14)
                ;
                kf_i * kf_scale_i
            }
        };
        let kd_func = {
            |posi| {
                let n = n_func(posi);
                let kd_scale_i = k_d_scale(n);
                if is_ocean_fn(posi) {
                    let kd_i =
                        /*1.0e-2*/
                        1.0e-2 / 4.0
                    ;
                    return kd_i * kd_scale_i;
                }
                /* let wposf = (uniform_idx_as_vec2(posi)
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                .map(|e| e as f64); */
                /* let turb_wposf = wposf
                    .mul(5_000.0 / continent_scale)
                    .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                    .div(turb_wposf_div);
                let turb = Vec2::new(
                    gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                    gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
                ) * uplift_turb_scale
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
                // let turb = Vec2::zero();
                let turb_wposf = wposf + turb; */
                /* let turb_wposi = turb_wposf
                    .div(5_000.0 / continent_scale)
                    .map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as f64)
                    .map2(WORLD_SIZE, |e, f| (e as i32).max(f as i32 - 1).min(0));
                let turb_posi = vec2_as_uniform_idx(turb_wposi); */
                /* let uheight = gen_ctx
                .uplift_nz
                .get(turb_wposf.into_array())
                /* .min(0.5)
                .max(-0.5)*/
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5); */
                /* let uchaos = /* gen_ctx.chaos_nz.get((wposf.div(3_000.0)).into_array())
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5); */
                chaos[posi].1; */

                // kd = 1e-1: high (mountain, dike)
                // kd = 1.5e-2: normal-high (plateau [fan sediment])
                // kd = 1e-2: normal (plateau)
                // multiplied by height_scale² to account for change in area, then divided by
                // time_scale to account for lower dt.
                let kd_i = // 1.0e-2 * kd_scale_i;// m_old^2 / y * (1 m_new / 4 m_old)^2
                    1.0e-2  / 4.0
                    // (uheight * (1.0e-1 - 1.0e-2) + 1.0e-2)
                    // ((1.0 - uheight) * (0.5 + 0.5 * ((1.32 - uchaos as f64) / 1.32)) * (1.0e-2 - 1.0e-3) + 1.0e-3)
                    // (uheight * (1.0e-2 - 1.0e-3) + 1.0e-3) / 2.0
                ;
                kd_i * kd_scale_i
            }
        };
        let g_func = |posi| {
            if
            /*is_ocean_fn(posi)*/
            map_edge_factor(posi) == 0.0 {
                return 0.0;
                // return 5.0;
            }
            /* let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
            .map(|e| e as f64); */
            /* let turb_wposf = wposf
                .mul(5_000.0 / continent_scale)
                .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                .div(turb_wposf_div);
            let turb = Vec2::new(
                gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
            ) * uplift_turb_scale
                * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
            // let turb = Vec2::zero();
            let turb_wposf = wposf + turb; */
            /* let turb_wposi = turb_wposf
                .div(5_000.0 / continent_scale)
                .map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as f64)
                .map2(WORLD_SIZE, |e, f| (e as i32).max(f as i32 - 1).min(0));
            let turb_posi = vec2_as_uniform_idx(turb_wposi); */
            /* let uheight = gen_ctx
            .uplift_nz
            .get(turb_wposf.into_array())
            /* .min(0.5)
            .max(-0.5)*/
            .min(1.0)
            .max(-1.0)
            .mul(0.5)
            .add(0.5); */

            /* let uchaos = /* gen_ctx.chaos_nz.get((wposf.div(3_000.0)).into_array())
                    .min(1.0)
                    .max(-1.0)
                    .mul(0.5)
                    .add(0.5); */
                    chaos[posi].1;

            assert!(uchaos <= 1.32); */

            // G = d* v_s / p_0, where
            //  v_s is the settling velocity of sediment grains
            //  p_0 is the mean precipitation rate
            //  d* is the sediment concentration ratio (between concentration near riverbed
            //  interface, and average concentration over the water column).
            //  d* varies with Rouse number which defines relative contribution of bed, suspended,
            //  and washed loads.
            //
            // G is typically on the order of 1 or greater.  However, we are only guaranteed to
            // converge for G ≤ 1, so we keep it in the chaos range of [0.12, 1.32].
            // (((1.32 - uchaos) / 1.32).powf(0.75) * 1.32).min(/*1.1*/1.0)
            // ((1.32 - 0.12) * (1.0 - uheight) + 0.12) as f32
            // 1.1 * (1.0 - uheight) as f32
            // 1.0 * (1.0 - uheight) as f32
            // 1.0
            // 5.0
            // 10.0
            // 2.0
            // 0.0
            1.0
            // 4.0
            // 1.0
            // 1.5
        };
        let epsilon_0_func = |posi| {
            // epsilon_0_scale is roughly [using Hack's Law with b = 2 and SPL without debris flow or
            // hillslopes] equal to the ratio of the old to new area, to the power of -n_i.
            let epsilon_0_scale_i = epsilon_0_scale(n_func(posi));
            if is_ocean_fn(posi) {
                // marine: ε₀ = 2.078e-3
                // divide by height scale, multiplied by time_scale, cancels out to 1; idea is that
                // we are finishing in time τ = τ' * height_scale.  We have production
                // rate
                //
                // ∆P = ε₀ e^(-αH) Δt
                //    = ε₀ e^(-α' / height_scale * (H' * height_scale)) (Δt' * height_scale)
                //    = ε₀ e^(-α' H') (Δt' * height_scale)
                //
                // while the old production rate was
                //
                // ∆P' = ε₀' e^(-α'H') Δt'.
                //
                // BUT, we don't actually want the same production rate, but rather the same
                // *relative* production rate, which means we actually want to multiply by
                // height_scale again... this entails multiplying the right hand side by the
                // production rate, which gets us
                //
                // ΔP = ΔP' * height_scale
                // ΔP / height_scale = ΔP'
                //
                // so to equate them we need
                //
                //      ε₀ e^(-α' H') (Δt' * height_scale) / height_scale = ε₀' e^(-α' H') Δt'
                //      ε₀ = ε₀'
                let epsilon_0_i =
                    //2.078e-3
                    2.078e-3 /  4.0
                ;
                return epsilon_0_i * epsilon_0_scale_i/* * time_scale*/;
                // return 5.0;
            }
            let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                .map(|e| e as f64);
            let turb_wposf = wposf
                .mul(5_000.0 / continent_scale)
                .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                .div(turb_wposf_div);
            let turb = Vec2::new(
                gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
            ) * uplift_turb_scale
                * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
            // let turb = Vec2::zero();
            let turb_wposf = wposf + turb;
            /* let turb_wposi = turb_wposf
                .div(5_000.0 / continent_scale)
                .map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as f64)
                .map2(WORLD_SIZE, |e, f| (e as i32).max(f as i32 - 1).min(0));
            let turb_posi = vec2_as_uniform_idx(turb_wposi); */
            let uheight = gen_ctx
                .uplift_nz
                .get(turb_wposf.into_array())
                /* .min(0.5)
                .max(-0.5)*/
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5);
            /* let n_i = n_func(posi);
            let height_scale = height_scale(n_i);
            let uheight = uheight / height_scale; */
            let wposf3 = Vec3::new(
                wposf.x,
                wposf.y,
                uheight * CONFIG.mountain_scale as f64 * rock_strength_div_factor,
            );
            let rock_strength = gen_ctx
                .rock_strength_nz
                .get(wposf3.into_array())
                /* .min(0.5)
                .max(-0.5)*/
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5);
            let center = /*0.25*/0.4;
            let dmin = center - /*0.15;//0.05*/0.05;
            let dmax = center + /*0.05*//*0.10*/0.05; //0.05;
            let log_odds = |x: f64| logit(x) - logit(center);
            let ustrength = logistic_cdf(
                1.0 * logit(rock_strength.min(1.0f64 - 1e-7).max(1e-7))
                    + 1.0 * log_odds(uheight.min(dmax).max(dmin)),
            );
            // marine: ε₀ = 2.078e-3
            // San Gabriel Mountains: ε₀ = 3.18e-4
            // Oregon Coast Range: ε₀ = 2.68e-4
            // Frogs Hollow (peak production = 0.25): ε₀ = 1.41e-4
            // Point Reyes: ε₀ = 8.1e-5
            // Nunnock River (fractured granite, least weathered?): ε₀ = 5.3e-5
            // The stronger the rock, the lower the production rate of exposed bedrock.
            // divide by height scale, then multiplied by time_scale, cancels out.
            let epsilon_0_i =
                // ((1.0 - ustrength) * (/*3.18e-4*/2.078e-3 - 5.3e-5) + 5.3e-5) as f32
                ((1.0 - ustrength) * (/*3.18e-4*/2.078e-3 - 5.3e-5) + 5.3e-5) as f32 / 4.0
            /* * time_scale*/
            // 0.0
            ;
            epsilon_0_i * epsilon_0_scale_i
        };
        let alpha_func = |posi| {
            // height_scale is roughly [using Hack's Law with b = 2 and SPL without debris flow or
            // hillslopes] equal to the ratio of the old to new area, to the power of -n_i.
            // the old height * height scale, and we take the rate as ε₀ * e^(-αH), to keep
            // the rate of rate of change in soil production consistent we must divide H by
            // height_scale.
            //
            // αH = α(H' * height_scale) = α'H'
            // α = α' / height_scale
            let alpha_scale_i = alpha_scale(n_func(posi));
            if is_ocean_fn(posi) {
                // marine: α = 3.7e-2
                return 3.7e-2 * alpha_scale_i;
            }
            let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                .map(|e| e as f64);
            let turb_wposf = wposf
                .mul(5_000.0 / continent_scale)
                .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                .div(turb_wposf_div);
            let turb = Vec2::new(
                gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
            ) * uplift_turb_scale
                * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
            // let turb = Vec2::zero();
            let turb_wposf = wposf + turb;
            /* let turb_wposi = turb_wposf
                .div(5_000.0 / continent_scale)
                .map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as f64)
                .map2(WORLD_SIZE, |e, f| (e as i32).max(f as i32 - 1).min(0));
            let turb_posi = vec2_as_uniform_idx(turb_wposi); */
            let uheight = gen_ctx
                .uplift_nz
                .get(turb_wposf.into_array())
                /* .min(0.5)
                .max(-0.5)*/
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5);
            /* let n_i = n_func(posi);
            let height_scale = height_scale(n_i);
            let uheight = uheight / height_scale; */
            let wposf3 = Vec3::new(
                wposf.x,
                wposf.y,
                uheight * CONFIG.mountain_scale as f64 * rock_strength_div_factor,
            );
            let rock_strength = gen_ctx
                .rock_strength_nz
                .get(wposf3.into_array())
                /* .min(0.5)
                .max(-0.5)*/
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5);
            let center = /*0.25*/0.4;
            let dmin = center - /*0.15;//0.05*/0.05;
            let dmax = center + /*0.05*//*0.10*/0.05; //0.05;
            let log_odds = |x: f64| logit(x) - logit(center);
            let ustrength = logistic_cdf(
                1.0 * logit(rock_strength.min(1.0f64 - 1e-7).max(1e-7))
                    + 1.0 * log_odds(uheight.min(dmax).max(dmin)),
            );
            // Frog Hollow (peak production = 0.25): α = 4.2e-2
            // San Gabriel Mountains: α = 3.8e-2
            // marine: α = 3.7e-2
            // Oregon Coast Range: α = 3e-2
            // Nunnock river (fractured granite, least weathered?): α = 2e-3
            // Point Reyes: α = 1.6e-2
            // The stronger  the rock, the faster the decline in soil production.
            let alpha_i = (ustrength * (4.2e-2 - 1.6e-2) + 1.6e-2) as f32;
            alpha_i * alpha_scale_i
        };
        let uplift_fn = |posi| {
            if is_ocean_fn(posi) {
                /* return 1e-2
                .mul(max_erosion_per_delta_t) as f32; */
                return 0.0;
            }
            /* let wposf = (uniform_idx_as_vec2(posi) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                .map(|e| e as f64);
            let alt_main = {
                // Extension upwards from the base.  A positive number from 0 to 1 curved to be
                // maximal at 0.  Also to be multiplied by CONFIG.mountain_scale.
                let alt_main = (gen_ctx
                    .alt_nz
                    .get((wposf.div(2_000.0)).into_array())
                    .min(1.0)
                    .max(-1.0))
                .abs()
                .powf(1.35);

                fn spring(x: f64, pow: f64) -> f64 {
                    x.abs().powf(pow) * x.signum()
                }

                (0.0 + alt_main
                    + (gen_ctx
                        .small_nz
                        .get((wposf.div(300.0)).into_array())
                        .min(1.0)
                        .max(-1.0))
                    .mul(alt_main.powf(0.8).max(/*0.25*/ 0.15))
                    .mul(0.3)
                    .add(1.0)
                    .mul(0.4)/* + spring(alt_main.abs().powf(0.5).min(0.75).mul(60.0).sin(), 4.0)
                .mul(0.045)*/)
            }; */
            let height =
                    (/*old_height_uniform*/uplift_uniform[posi]./*0*/1 - alt_old_min_uniform) as f64
                    / (alt_old_max_uniform - alt_old_min_uniform) as f64
                    /*((old_height(posi) - alt_old_min) as f64
                    / (alt_old_max - alt_old_min) as f64)*/
                ;

            let height = height.mul(max_epsilon - min_epsilon).add(min_epsilon);
            /*.max(1e-7 / CONFIG.mountain_scale as f64)
            .min(1.0f64 - 1e-7);*/
            /* let alt_main = {
                // Extension upwards from the base.  A positive number from 0 to 1 curved to be
                // maximal at 0.  Also to be multiplied by CONFIG.mountain_scale.
                let alt_main = (gen_ctx
                    .alt_nz
                    .get((wposf.div(2_000.0)).into_array())
                    .min(1.0)
                    .max(-1.0))
                .abs()
                .powf(1.35);

                fn spring(x: f64, pow: f64) -> f64 {
                    x.abs().powf(pow) * x.signum()
                }

                (0.0 + alt_main
                    + (gen_ctx
                        .small_nz
                        .get((wposf.div(300.0)).into_array())
                        .min(1.0)
                        .max(-1.0))
                    .mul(alt_main.powf(0.8).max(/*0.25*/ 0.15))
                    .mul(0.3)
                    .add(1.0)
                    .mul(0.4)
                    + spring(alt_main.abs().powf(0.5).min(0.75).mul(60.0).sin(), 4.0).mul(0.045))
            }; */
            // let height = height + (alt_main./*to_le_bytes()[7]*/to_bits() & 1) as f64 * ((1.0 / CONFIG.mountain_scale as f64).powf(1.0 / erosion_pow_low));
            let height = erosion_factor(height);
            assert!(height >= 0.0);
            assert!(height <= 1.0);
            // assert!(alt_main >= 0.0);
            let (bump_factor, bump_max) = if
            /*height < f32::EPSILON as f64 * 0.5*//*false*/
            /*true*/
            false {
                (
                    /*(alt_main./*to_le_bytes()[7]*/to_bits() & 1) as f64*/
                    /* (alt_main / CONFIG.mountain_scale as f64 * 128.0).mul(0.1).powf(1.2) * /*(1.0 / CONFIG.mountain_scale as f64)*/(f32::EPSILON * 0.5) as f64, */
                    0.0,
                    (f32::EPSILON * 0.5) as f64,
                )
            } else {
                (0.0, 0.0)
            };
            // tan(6/360*2*pi)*32 ~ 3.4
            // 3.4/32*512 ~ 54
            // 18/32*512 ~ 288
            // tan(pi/6)*32 ~ 18
            // tan(54/360*2*pi)*32
            // let height = 1.0f64;
            /* let turb_wposf = wposf
                .mul(5_000.0 / continent_scale)
                .div(TerrainChunkSize::RECT_SIZE.map(|e| e as f64))
                .div(turb_wposf_div);
            let turb = Vec2::new(
                gen_ctx.turb_x_nz.get(turb_wposf.into_array()),
                gen_ctx.turb_y_nz.get(turb_wposf.into_array()),
            ) * uplift_turb_scale
                * TerrainChunkSize::RECT_SIZE.map(|e| e as f64);
            let turb_wposf = wposf + turb;
            let uheight = gen_ctx
                .uplift_nz
                .get(turb_wposf.into_array())
                /* .min(0.5)
                .max(-0.5)*/
                .min(1.0)
                .max(-1.0)
                .mul(0.5)
                .add(0.5); */
            // u = 1e-3: normal-high (dike, mountain)
            // u = 5e-4: normal (mid example in Yuan, average mountain uplift)
            // u = 2e-4: low (low example in Yuan; known that lagoons etc. may have u ~ 0.05).
            // u = 0: low (plateau [fan, altitude = 0.0])
            // let height = uheight;
            // let height = 1.0f64;

            // let height = 1.0 / 7.0f64;
            // let height = 0.0 / 31.0f64;
            let bfrac = /*erosion_factor(0.5);*/0.0;
            let height = (height - bfrac).abs().div(1.0 - bfrac);
            let height = height
                /* .mul(31.0 / 32.0)
                .add(1.0 / 32.0) */
                /* .mul(15.0 / 16.0)
                .add(1.0 / 16.0) */
                /* .mul(5.0 / 8.0)
                .add(3.0 / 8.0) */
                /* .mul(6.0 / 8.0)
                .add(2.0 / 8.0) */
                /* .mul(7.0 / 8.0)
                .add(1.0 / 8.0) */
                .mul(max_erosion_per_delta_t)
                .sub(/*1.0 / CONFIG.mountain_scale as f64*/ bump_max)
                .add(bump_factor);
            /* .sub(/*1.0 / CONFIG.mountain_scale as f64*/(f32::EPSILON * 0.5) as f64)
            .add(bump_factor); */
            height as f64
        };
        let alt_func = |posi| {
            if is_ocean_fn(posi) {
                // -max_erosion_per_delta_t as f32
                // -1.0 / CONFIG.mountain_scale
                // -0.75
                // -CONFIG.sea_level / CONFIG.mountain_scale
                // 0.0
                // 0.0
                old_height(posi) // 0.0
            } else {
                // uplift_fn(posi)
                /* let wposf = (uniform_idx_as_vec2(posi)
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                .map(|e| e as f64);
                let alt_main = {
                    // Extension upwards from the base.  A positive number from 0 to 1 curved to be
                    // maximal at 0.  Also to be multiplied by CONFIG.mountain_scale.
                    let alt_main = (gen_ctx
                        .alt_nz
                        .get((wposf.div(2_000.0)).into_array())
                        .min(1.0)
                        .max(-1.0))
                    .abs()
                    .powf(1.35);

                    fn spring(x: f64, pow: f64) -> f64 {
                        x.abs().powf(pow) * x.signum()
                    }

                    (0.0 + alt_main
                        + (gen_ctx
                            .small_nz
                            .get((wposf.div(300.0)).into_array())
                            .min(1.0)
                            .max(-1.0))
                        .mul(alt_main.powf(0.8).max(/*0.25*/ 0.15))
                        .mul(0.3)
                        .add(1.0)
                        .mul(0.4)/* + spring(alt_main.abs().powf(0.5).min(0.75).mul(60.0).sin(), 4.0)
                    .mul(0.045)*/)
                }; */

                // (kf_func(posi) / 1.5e-4 * CONFIG.mountain_scale as f64) as f32
                // (old_height_uniform(posi) as f64 * CONFIG.mountain_scale as f64) as f32
                // (old_height_uniform(posi) as f64 * CONFIG.mountain_scale as f64) as f32
                // (uplift_fn(posi) * CONFIG.mountain_scale as f64) as f32
                // (old_height_uniform(posi) - 0.5)/* * max_erosion_per_delta_t as f32*/
                (old_height(posi) as f64 / CONFIG.mountain_scale as f64) as f32 - 0.5
                // ((((old_height(posi) - alt_old_min) as f64 / (alt_old_max - alt_old_min) as f64) - 0.25) * (CONFIG.mountain_scale as f64)) as f32
                // old_height(posi)/* * max_erosion_per_delta_t as f32*/
                // uplift_fn(posi) * (CONFIG.mountain_scale / max_erosion_per_delta_t as f32)
                // 0.0
                // /*-CONFIG.mountain_scale * 0.5 + *//*-CONFIG.mountain_scale/* * 0.75*/ + */(old_height_uniform(posi)/*.powf(2.0)*/ - 0.5)/* * CONFIG.mountain_scale as f32*/
                // uplift_fn(posi) * (CONFIG.mountain_scale / max_erosion_per_delta_t as f32)
                // 0.0
                /* // 0.0
                // -/*CONFIG.sea_level / CONFIG.mountain_scale*//* 0.75 */1.0
                // ((old_height(posi) - alt_old_min) as f64 / (alt_old_max - alt_old_min) as f64) as f32
                // uplift_fn(posi) / max_erosion_per_delta_t as f32
                // old_height_uniform(posi) *
                (/*((old_height(posi) - alt_old_min) as f64 / (alt_old_max - alt_old_min) as f64) **/(((6.0 / 360.0 * 2.0 * f64::consts::PI).tan()
                    * TerrainChunkSize::RECT_SIZE.reduce_partial_min() as f64)
                    .floor()
                    * height_scale)) as f32
                // 5.0 / CONFIG.mountain_scale */
            }
        };

        /* // FIXME: Remove.
        let is_ocean = (0..WORLD_SIZE.x * WORLD_SIZE.y)
            .into_par_iter()
            .map(|i| map_edge_factor(i) == 0.0)
            .collect::<Vec<_>>();
        let is_ocean_fn = |posi: usize| is_ocean[posi]; */

        // Parse out the contents of various map formats into the values we need.
        let parsed_world_file = (|| {
            let map = match opts.world_file {
                FileOpts::LoadLegacy(ref path) => {
                    let file = match File::open(path) {
                        Ok(file) => file,
                        Err(err) => {
                            log::warn!("Couldn't read path for maps: {:?}", err);
                            return None;
                        }
                    };

                    let reader = BufReader::new(file);
                    let map: WorldFileLegacy = match bincode::deserialize_from(reader) {
                        Ok(map) => map,
                        Err(err) => {
                            log::warn!("Couldn't parse legacy map: {:?}).  Maybe you meant to try a regular load?", err);
                            return None;
                        }
                    };

                    map.into_modern()
                }
                FileOpts::Load(ref path) => {
                    let file = match File::open(path) {
                        Ok(file) => file,
                        Err(err) => {
                            log::warn!("Couldn't read path for maps: {:?}", err);
                            return None;
                        }
                    };

                    let reader = BufReader::new(file);
                    let map: WorldFile = match bincode::deserialize_from(reader) {
                        Ok(map) => map,
                        Err(err) => {
                            log::warn!("Couldn't parse modern map: {:?}).  Maybe you meant to try a legacy load?", err);
                            return None;
                        }
                    };

                    map.into_modern()
                }
                FileOpts::LoadAsset(ref specifier) => {
                    let reader = match assets::load_file(specifier, &["bin"]) {
                        Ok(reader) => reader,
                        Err(err) => {
                            log::warn!(
                                "Couldn't read asset specifier {:?} for maps: {:?}",
                                specifier,
                                err
                            );
                            return None;
                        }
                    };

                    let map: WorldFile = match bincode::deserialize_from(reader) {
                        Ok(map) => map,
                        Err(err) => {
                            log::warn!("Couldn't parse modern map: {:?}).  Maybe you meant to try a legacy load?", err);
                            return None;
                        }
                    };

                    map.into_modern()
                }
                FileOpts::Generate | FileOpts::Save => return None,
            };

            match map {
                Ok(map) => Some(map),
                Err(e) => {
                    match e {
                        WorldFileError::WorldSizeInvalid => {
                            log::warn!("World size of map is invalid.");
                        }
                    }
                    None
                }
            }
        })();

        let (alt, basement /*, alluvium*/) = if let Some(map) = parsed_world_file {
            // let map_len = map.alt.len();
            (
                map.alt,
                map.basement, /* vec![0.0; map_len].into_boxed_slice() */
            )
        } else {
            let (alt, basement /*, alluvium*/) = do_erosion(
                max_erosion_per_delta_t as f32,
                n_steps,
                &river_seed,
                // varying conditions
                &rock_strength_nz,
                // initial conditions
                |posi| alt_func(posi), // + if is_ocean_fn(posi) { 0.0 } else { 128.0 },
                |posi| {
                    alt_func(posi)
                        - if is_ocean_fn(posi) {
                            0.0
                        } else {
                            /*1400.0*//*CONFIG.mountain_scale * 0.75*/
                            0.0
                        }
                }, // if is_ocean_fn(posi) { old_height(posi) } else { 0.0 },
                // |posi| 0.0,
                is_ocean_fn,
                // empirical constants
                uplift_fn,
                |posi| n_func(posi),
                |posi| theta_func(posi),
                |posi| kf_func(posi),
                |posi| kd_func(posi),
                |posi| g_func(posi),
                |posi| epsilon_0_func(posi),
                |posi| alpha_func(posi),
                // scaling factors
                |n| height_scale(n),
                k_d_scale(n_approx),
                |q| k_da_scale(q),
            );

            // Quick "small scale" erosion cycle in order to lower extreme angles.
            do_erosion(
                (1.0/* * height_scale*/) as f32,
                n_small_steps,
                &river_seed,
                &rock_strength_nz,
                |posi| /* if is_ocean_fn(posi) { old_height(posi) } else { alt[posi] } *//*alt[posi] as f32*/(alt[posi]/* + alluvium[posi]*/) as f32,
                |posi| basement[posi] as f32,
                // |posi| /*alluvium[posi] as f32*/0.0f32,
                is_ocean_fn,
                |posi| uplift_fn(posi) * (1.0/* * height_scale*/ / max_erosion_per_delta_t),
                |posi| n_func(posi),
                |posi| theta_func(posi),
                |posi| kf_func(posi),
                |posi| kd_func(posi),
                |posi| g_func(posi),
                |posi| epsilon_0_func(posi),
                |posi| alpha_func(posi),
                |n| height_scale(n),
                k_d_scale(n_approx),
                |q| k_da_scale(q),
            )
        };

        // Save map, if necessary.
        // NOTE: We wll always save a map with latest version.
        let map = WorldFile::new(ModernMap { alt, basement });
        (|| {
            if let FileOpts::Save = opts.world_file {
                use std::time::SystemTime;
                // Check if folder exists and create it if it does not
                let mut path = PathBuf::from("./maps");
                if !path.exists() {
                    if let Err(err) = std::fs::create_dir(&path) {
                        log::warn!("Couldn't create folder for map: {:?}", err);
                        return;
                    }
                }
                path.push(format!(
                    // TODO: Work out a nice bincode file extension.
                    "map_{}.bin",
                    SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                ));
                let file = match File::create(path) {
                    Ok(file) => file,
                    Err(err) => {
                        log::warn!("Couldn't create file for maps: {:?}", err);
                        return;
                    }
                };

                let writer = BufWriter::new(file);
                if let Err(err) = bincode::serialize_into(writer, &map) {
                    log::warn!("Couldn't write map: {:?}", err);
                }
            }
        })();

        // Skip validation--we just performed a no-op conversion for this map, so it had better be
        // valid!
        let ModernMap { alt, basement } = map.into_modern().unwrap();

        // Additional small-scale eroson after map load, only used during testing.
        let (alt, basement /*, alluvium*/) = if n_post_load_steps == 0 {
            (alt, basement /*, alluvium*/)
        } else {
            do_erosion(
                (1.0/* * height_scale*/) as f32,
                n_post_load_steps,
                &river_seed,
                &rock_strength_nz,
                |posi| /* if is_ocean_fn(posi) { old_height(posi) } else { alt[posi] } */alt[posi] as f32,
                |posi| basement[posi] as f32,
                // |posi| alluvium[posi] as f32,
                is_ocean_fn,
                |posi| uplift_fn(posi) * (1.0/* * height_scale*/ / max_erosion_per_delta_t),
                |posi| n_func(posi),
                |posi| theta_func(posi),
                |posi| kf_func(posi),
                |posi| kd_func(posi),
                |posi| g_func(posi),
                |posi| epsilon_0_func(posi),
                |posi| alpha_func(posi),
                |n| height_scale(n),
                k_d_scale(n_approx),
                |q| k_da_scale(q),
            )
        };

        let is_ocean = get_oceans(|posi| alt[posi]);
        let is_ocean_fn = |posi: usize| is_ocean[posi];
        let mut dh = downhill(
            |posi| alt[posi], /*&alt*/
            /*old_height*/ is_ocean_fn,
        );
        let (boundary_len, indirection, water_alt_pos, maxh) =
            get_lakes(/*&/*water_alt*/alt*/ |posi| alt[posi], &mut dh);
        log::debug!("Max height: {:?}", maxh);
        let (mrec, mstack, mwrec) = {
            let mut wh = vec![0.0; WORLD_SIZE.x * WORLD_SIZE.y];
            get_multi_rec(
                |posi| alt[posi],
                &dh,
                &water_alt_pos,
                &mut wh,
                WORLD_SIZE.x,
                WORLD_SIZE.y,
                TerrainChunkSize::RECT_SIZE.x as Compute,
                TerrainChunkSize::RECT_SIZE.y as Compute,
                maxh,
            )
        };
        let flux_old = get_multi_drainage(&mstack, &mrec, &*mwrec, boundary_len);
        let flux_rivers = get_drainage(&water_alt_pos, &dh, boundary_len);
        // let flux_rivers = flux_old.clone();

        let water_height_initial = |chunk_idx| {
            let indirection_idx = indirection[chunk_idx];
            // Find the lake this point is flowing into.
            let lake_idx = if indirection_idx < 0 {
                chunk_idx
            } else {
                indirection_idx as usize
            };
            /* // Find the pass this lake is flowing into (i.e. water at the lake bottom gets
            // pushed towards the point identified by pass_idx).
            let neighbor_pass_idx = dh[lake_idx]; */
            let chunk_water_alt = if
            /*neighbor_pass_idx*/
            dh[lake_idx] < 0 {
                // This is either a boundary node (dh[chunk_idx] == -2, i.e. water is at sea level)
                // or part of a lake that flows directly into the ocean.  In the former case, water
                // is at sea level so we just return 0.0.  In the latter case, the lake bottom must
                // have been a boundary node in the first place--meaning this node flows directly
                // into the ocean.  In that case, its lake bottom is ocean, meaning its water is
                // also at sea level.  Thus, we return 0.0 in both cases.
                0.0
            } else {
                // This chunk is draining into a body of water that isn't the ocean (i.e., a lake).
                // Then we just need to find the pass height of the surrounding lake in order to
                // figure out the initial water height (which fill_sinks will then extend to make
                // sure it fills the entire basin).

                // Find the height of "our" side of the pass (the part of it that drains into this
                // chunk's lake).
                let pass_idx = -indirection[lake_idx] as usize;
                let pass_height_i = alt[pass_idx];
                // Find the pass this lake is flowing into (i.e. water at the lake bottom gets
                // pushed towards the point identified by pass_idx).
                let neighbor_pass_idx = dh[pass_idx/*lake_idx*/];
                // Find the height of the pass into which our lake is flowing.
                let pass_height_j = alt[neighbor_pass_idx as usize];
                // Find the maximum of these two heights.
                let pass_height = pass_height_i.max(pass_height_j);
                // Use the pass height as the initial water altitude.
                pass_height
            };
            // Use the maximum of the pass height and chunk height as the parameter to fill_sinks.
            let chunk_alt = alt[chunk_idx];
            chunk_alt.max(chunk_water_alt)
        };

        let water_alt = fill_sinks(water_height_initial, is_ocean_fn);
        /* let water_alt = (0..WORLD_SIZE.x * WORLD_SIZE.y)
        .into_par_iter()
        .map(|posi| water_height_initial(posi))
        .collect::<Vec<_>>(); */

        let rivers = get_rivers(&water_alt_pos, &water_alt, &dh, &indirection, &flux_rivers);

        let water_alt = indirection
            .par_iter()
            .enumerate()
            .map(|(chunk_idx, &indirection_idx)| {
                // Find the lake this point is flowing into.
                let lake_idx = if indirection_idx < 0 {
                    chunk_idx
                } else {
                    indirection_idx as usize
                };
                /* // Find the pass this lake is flowing into (i.e. water at the lake bottom gets
                // pushed towards the point identified by pass_idx).
                let neighbor_pass_idx = dh[lake_idx]; */
                if
                /*neighbor_pass_idx*/
                dh[lake_idx] < 0 {
                    // This is either a boundary node (dh[chunk_idx] == -2, i.e. water is at sea level)
                    // or part of a lake that flows directly into the ocean.  In the former case, water
                    // is at sea level so we just return 0.0.  In the latter case, the lake bottom must
                    // have been a boundary node in the first place--meaning this node flows directly
                    // into the ocean.  In that case, its lake bottom is ocean, meaning its water is
                    // also at sea level.  Thus, we return 0.0 in both cases.
                    0.0
                } else {
                    // This is not flowing into the ocean, so we can use the existing water_alt.
                    water_alt[chunk_idx] as f32
                }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let is_underwater = |chunk_idx: usize| match rivers[chunk_idx].river_kind {
            Some(RiverKind::Ocean) | Some(RiverKind::Lake { .. }) => true,
            Some(RiverKind::River { .. }) => false, // TODO: inspect width
            None => false,
        };

        // Check whether any tiles around this tile are not water (since Lerp will ensure that they
        // are included).
        let pure_water = |posi: usize| {
            /* let river_data = &rivers[posi];
            match river_data.river_kind {
                Some(RiverKind::Lake { .. }) => {
                    // Lakes are always completely submerged.
                    return true;
                },
                /* Some(RiverKind::River { cross_section }) if cross_section.x >= TerrainChunkSize::RECT_SIZE.x as f32 => {
                    // Rivers that are wide enough are considered completely submerged (not a
                    // completely fair approximation).
                    return true;
                }, */
                _ => {}
            } */
            let pos = uniform_idx_as_vec2(posi);
            for x in pos.x - 1..(pos.x + 1) + 1 {
                for y in pos.y - 1..(pos.y + 1) + 1 {
                    if x >= 0 && y >= 0 && x < WORLD_SIZE.x as i32 && y < WORLD_SIZE.y as i32 {
                        let posi = vec2_as_uniform_idx(Vec2::new(x, y));
                        if !is_underwater(posi) {
                            return false;
                        }
                    }
                }
            }
            true
        };

        // NaNs in these uniform vectors wherever pure_water() returns true.
        let (((alt_no_water, _), (pure_flux, _)), ((temp_base, _), (humid_base, _))) = rayon::join(
            || {
                rayon::join(
                    || {
                        uniform_noise(|posi, _| {
                            if pure_water(posi) {
                                None
                            } else {
                                // A version of alt that is uniform over *non-water* (or land-adjacent water)
                                // chunks.
                                Some(alt[posi] as f32)
                            }
                        })
                    },
                    || {
                        uniform_noise(|posi, _| {
                            if pure_water(posi) {
                                None
                            } else {
                                Some(flux_old[posi])
                            }
                        })
                    },
                )
            },
            || {
                rayon::join(
                    || {
                        uniform_noise(|posi, wposf| {
                            if pure_water(posi) {
                                None
                            } else {
                                // -1 to 1.
                                Some(gen_ctx.temp_nz.get((wposf/*.div(12000.0)*/).into_array())
                                    as f32)
                            }
                        })
                    },
                    || {
                        uniform_noise(|posi, wposf| {
                            // Check whether any tiles around this tile are water.
                            if pure_water(posi) {
                                None
                            } else {
                                // 0 to 1, hopefully.
                                Some(
                                    (gen_ctx.humid_nz.get(wposf.div(1024.0).into_array()) as f32)
                                        .add(1.0)
                                        .mul(0.5),
                                )
                            }
                        })
                    },
                )
            },
        );

        let gen_cdf = GenCdf {
            humid_base,
            temp_base,
            chaos,
            alt,
            basement,
            water_alt,
            dh,
            flux: flux_old,
            pure_flux,
            alt_no_water,
            rivers,
        };

        let chunks = (0..WORLD_SIZE.x * WORLD_SIZE.y)
            .into_par_iter()
            .map(|i| SimChunk::generate(i, &gen_ctx, &gen_cdf))
            .collect::<Vec<_>>();

        let mut this = Self {
            seed: seed,
            chunks,
            locations: Vec::new(),
            gen_ctx,
            rng,
        };

        if opts.seed_elements {
            this.seed_elements();
        }

        this
    }

    /// Draw a map of the world based on chunk information.  Returns a buffer of u32s.
    pub fn get_map(&self) -> Vec<u32> {
        let mut v = vec![0u32; WORLD_SIZE.x * WORLD_SIZE.y];
        // TODO: Parallelize again.
        MapConfig::default().generate(&self, |pos, (r, g, b, a)| {
            v[pos.y * WORLD_SIZE.x + pos.x] = u32::from_le_bytes([r, g, b, a]);
        });
        v
    }

    /// Prepare the world for simulation
    pub fn seed_elements(&mut self) {
        let mut rng = self.rng.clone();

        let cell_size = 16;
        let grid_size = WORLD_SIZE / cell_size;
        let loc_count = 100;

        let mut loc_grid = vec![None; grid_size.product()];
        let mut locations = Vec::new();

        // Seed the world with some locations
        (0..loc_count).for_each(|_| {
            let cell_pos = Vec2::new(
                self.rng.gen::<usize>() % grid_size.x,
                self.rng.gen::<usize>() % grid_size.y,
            );
            let wpos = (cell_pos * cell_size + cell_size / 2)
                .map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
                    e as i32 * sz as i32 + sz as i32 / 2
                });

            locations.push(Location::generate(wpos, &mut rng));

            loc_grid[cell_pos.y * grid_size.x + cell_pos.x] = Some(locations.len() - 1);
        });

        // Find neighbours
        let mut loc_clone = locations
            .iter()
            .map(|l| l.center)
            .enumerate()
            .collect::<Vec<_>>();
        (0..locations.len()).for_each(|i| {
            let pos = locations[i].center.map(|e| e as i64);

            loc_clone.sort_by_key(|(_, l)| l.map(|e| e as i64).distance_squared(pos));

            loc_clone.iter().skip(1).take(2).for_each(|(j, _)| {
                locations[i].neighbours.insert(*j);
                locations[*j].neighbours.insert(i);
            });
        });

        // Simulate invasion!
        let invasion_cycles = 25;
        (0..invasion_cycles).for_each(|_| {
            (0..grid_size.y).for_each(|j| {
                (0..grid_size.x).for_each(|i| {
                    if loc_grid[j * grid_size.x + i].is_none() {
                        const R_COORDS: [i32; 5] = [-1, 0, 1, 0, -1];
                        let idx = self.rng.gen::<usize>() % 4;
                        let new_i = i as i32 + R_COORDS[idx];
                        let new_j = j as i32 + R_COORDS[idx + 1];
                        if new_i >= 0 && new_j >= 0 {
                            let loc = Vec2::new(new_i as usize, new_j as usize);
                            loc_grid[j * grid_size.x + i] =
                                loc_grid.get(loc.y * grid_size.x + loc.x).cloned().flatten();
                        }
                    }
                });
            });
        });

        // Place the locations onto the world
        let gen = StructureGen2d::new(self.seed, cell_size as u32, cell_size as u32 / 2);

        self.chunks
            .par_iter_mut()
            .enumerate()
            .for_each(|(ij, chunk)| {
                let chunk_pos = uniform_idx_as_vec2(ij);
                let i = chunk_pos.x as usize;
                let j = chunk_pos.y as usize;
                let block_pos = Vec2::new(
                    chunk_pos.x * TerrainChunkSize::RECT_SIZE.x as i32,
                    chunk_pos.y * TerrainChunkSize::RECT_SIZE.y as i32,
                );
                let _cell_pos = Vec2::new(i / cell_size, j / cell_size);

                // Find the distance to each region
                let near = gen.get(chunk_pos);
                let mut near = near
                    .iter()
                    .map(|(pos, seed)| RegionInfo {
                        chunk_pos: *pos,
                        block_pos: pos
                            .map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e * sz as i32),
                        dist: (pos - chunk_pos).map(|e| e as f32).magnitude(),
                        seed: *seed,
                    })
                    .collect::<Vec<_>>();

                // Sort regions based on distance
                near.sort_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap());

                let nearest_cell_pos = near[0].chunk_pos;
                if nearest_cell_pos.x >= 0 && nearest_cell_pos.y >= 0 {
                    let nearest_cell_pos = nearest_cell_pos.map(|e| e as usize) / cell_size;
                    chunk.location = loc_grid
                        .get(nearest_cell_pos.y * grid_size.x + nearest_cell_pos.x)
                        .cloned()
                        .unwrap_or(None)
                        .map(|loc_idx| LocationInfo { loc_idx, near });

                    let town_size = 200;
                    let in_town = chunk
                        .location
                        .as_ref()
                        .map(|l| {
                            locations[l.loc_idx]
                                .center
                                .map(|e| e as i64)
                                .distance_squared(block_pos.map(|e| e as i64))
                                < town_size * town_size
                        })
                        .unwrap_or(false);

                    if in_town {
                        chunk.spawn_rate = 0.0;
                    }
                }
            });

        // Stage 2 - towns!
        let chunk_idx_center = |e: Vec2<i32>| {
            e.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
                e * sz as i32 + sz as i32 / 2
            })
        };
        let maybe_towns = self
            .gen_ctx
            .town_gen
            .par_iter(
                chunk_idx_center(Vec2::zero()),
                chunk_idx_center(WORLD_SIZE.map(|e| e as i32)),
            )
            .map_init(
                || Box::new(BlockGen::new(ColumnGen::new(self))),
                |mut block_gen, (pos, seed)| {
                    let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
                    // println!("Town: {:?}", town);
                    TownState::generate(pos, &mut block_gen, &mut rng).map(|t| (pos, Arc::new(t)))
                },
            )
            .filter_map(|x| x)
            .collect::<HashMap<_, _>>();

        let gen_ctx = &self.gen_ctx;
        self.chunks
            .par_iter_mut()
            .enumerate()
            .for_each(|(ij, chunk)| {
                let chunk_pos = uniform_idx_as_vec2(ij);
                let wpos = chunk_idx_center(chunk_pos);

                let near_towns = gen_ctx.town_gen.get(wpos);
                let town = near_towns
                    .iter()
                    .min_by_key(|(pos, _seed)| wpos.distance_squared(*pos));

                let maybe_town = town
                    .and_then(|(pos, _seed)| maybe_towns.get(pos))
                    // Only care if we're close to the town
                    .filter(|town| {
                        Vec2::from(town.center()).distance_squared(wpos)
                            < town.radius().add(64).pow(2)
                    })
                    .cloned();

                chunk.structures.town = maybe_town;
            });

        self.rng = rng;
        self.locations = locations;
    }

    pub fn get(&self, chunk_pos: Vec2<i32>) -> Option<&SimChunk> {
        if chunk_pos
            .map2(WORLD_SIZE, |e, sz| e >= 0 && e < sz as i32)
            .reduce_and()
        {
            Some(&self.chunks[vec2_as_uniform_idx(chunk_pos)])
        } else {
            None
        }
    }

    pub fn get_wpos(&self, wpos: Vec2<i32>) -> Option<&SimChunk> {
        self.get(
            wpos.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                e / sz as i32
            }),
        )
    }

    pub fn get_mut(&mut self, chunk_pos: Vec2<i32>) -> Option<&mut SimChunk> {
        if chunk_pos
            .map2(WORLD_SIZE, |e, sz| e >= 0 && e < sz as i32)
            .reduce_and()
        {
            Some(&mut self.chunks[vec2_as_uniform_idx(chunk_pos)])
        } else {
            None
        }
    }

    pub fn get_base_z(&self, chunk_pos: Vec2<i32>) -> Option<f32> {
        if !chunk_pos
            .map2(WORLD_SIZE, |e, sz| e > 0 && e < sz as i32 - 2)
            .reduce_and()
        {
            return None;
        }

        let chunk_idx = vec2_as_uniform_idx(chunk_pos);
        local_cells(chunk_idx)
            .flat_map(|neighbor_idx| {
                let neighbor_pos = uniform_idx_as_vec2(neighbor_idx);
                let neighbor_chunk = self.get(neighbor_pos);
                let river_kind = neighbor_chunk.and_then(|c| c.river.river_kind);
                let has_water = river_kind.is_some() && river_kind != Some(RiverKind::Ocean);
                if (neighbor_pos - chunk_pos).reduce_partial_max() <= 1 || has_water {
                    neighbor_chunk.map(|c| c.get_base_z())
                } else {
                    None
                }
            })
            .fold(None, |a: Option<f32>, x| a.map(|a| a.min(x)).or(Some(x)))
    }

    pub fn get_interpolated<T, F>(&self, pos: Vec2<i32>, mut f: F) -> Option<T>
    where
        T: Copy + Default + Add<Output = T> + Mul<f32, Output = T>,
        F: FnMut(&SimChunk) -> T,
    {
        let pos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
            e as f64 / sz as f64
        });

        let cubic = |a: T, b: T, c: T, d: T, x: f32| -> T {
            let x2 = x * x;

            // Catmull-Rom splines
            let co0 = a * -0.5 + b * 1.5 + c * -1.5 + d * 0.5;
            let co1 = a + b * -2.5 + c * 2.0 + d * -0.5;
            let co2 = a * -0.5 + c * 0.5;
            let co3 = b;

            co0 * x2 * x + co1 * x2 + co2 * x + co3
        };

        let mut x = [T::default(); 4];

        for (x_idx, j) in (-1..3).enumerate() {
            let y0 = f(self.get(pos.map2(Vec2::new(j, -1), |e, q| e.max(0.0) as i32 + q))?);
            let y1 = f(self.get(pos.map2(Vec2::new(j, 0), |e, q| e.max(0.0) as i32 + q))?);
            let y2 = f(self.get(pos.map2(Vec2::new(j, 1), |e, q| e.max(0.0) as i32 + q))?);
            let y3 = f(self.get(pos.map2(Vec2::new(j, 2), |e, q| e.max(0.0) as i32 + q))?);

            x[x_idx] = cubic(y0, y1, y2, y3, pos.y.fract() as f32);
        }

        Some(cubic(x[0], x[1], x[2], x[3], pos.x.fract() as f32))
    }

    /// M. Steffen splines.
    ///
    /// A more expensive cubic interpolation function that can preserve monotonicity between
    /// points.  This is useful if you rely on relative differences between endpoints being
    /// preserved at all interior points.  For example, we use this with riverbeds (and water
    /// height on along rivers) to maintain the invariant that the rivers always flow downhill at
    /// interior points (not just endpoints), without needing to flatten out the river.
    pub fn get_interpolated_monotone<T, F>(&self, pos: Vec2<i32>, mut f: F) -> Option<T>
    where
        T: Copy + Default + Signed + Float + Add<Output = T> + Mul<f32, Output = T>,
        F: FnMut(&SimChunk) -> T,
    {
        // See http://articles.adsabs.harvard.edu/cgi-bin/nph-iarticle_query?1990A%26A...239..443S&defaultprint=YES&page_ind=0&filetype=.pdf
        //
        // Note that these are only guaranteed monotone in one dimension; fortunately, that is
        // sufficient for our purposes.
        let pos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
            e as f64 / sz as f64
        });

        let secant = |b: T, c: T| c - b;

        let parabola = |a: T, c: T| -a * 0.5 + c * 0.5;

        let slope = |_a: T, _b: T, _c: T, s_a: T, s_b: T, p_b: T| {
            // ((b - a).signum() + (c - b).signum()) * s
            (s_a.signum() + s_b.signum()) * (s_a.abs().min(s_b.abs()).min(p_b.abs() * 0.5))
        };

        let cubic = |a: T, b: T, c: T, d: T, x: f32| -> T {
            // Compute secants.
            let s_a = secant(a, b);
            let s_b = secant(b, c);
            let s_c = secant(c, d);
            // Computing slopes from parabolas.
            let p_b = parabola(a, c);
            let p_c = parabola(b, d);
            // Get slopes (setting distance between neighbors to 1.0).
            let slope_b = slope(a, b, c, s_a, s_b, p_b);
            let slope_c = slope(b, c, d, s_b, s_c, p_c);
            let x2 = x * x;

            // Interpolating splines.
            let co0 = slope_b + slope_c - s_b * 2.0;
            // = a * -0.5 + c * 0.5 + b * -0.5 + d * 0.5 - 2 * (c - b)
            // = a * -0.5 + b * 1.5 - c * 1.5 + d * 0.5;
            let co1 = s_b * 3.0 - slope_b * 2.0 - slope_c;
            // = (3.0 * (c - b) - 2.0 * (a * -0.5 + c * 0.5) - (b * -0.5 + d * 0.5))
            // = a + b * -2.5 + c * 2.0 + d * -0.5;
            let co2 = slope_b;
            // = a * -0.5 + c * 0.5;
            let co3 = b;

            co0 * x2 * x + co1 * x2 + co2 * x + co3
        };

        let mut x = [T::default(); 4];

        for (x_idx, j) in (-1..3).enumerate() {
            let y0 = f(self.get(pos.map2(Vec2::new(j, -1), |e, q| e.max(0.0) as i32 + q))?);
            let y1 = f(self.get(pos.map2(Vec2::new(j, 0), |e, q| e.max(0.0) as i32 + q))?);
            let y2 = f(self.get(pos.map2(Vec2::new(j, 1), |e, q| e.max(0.0) as i32 + q))?);
            let y3 = f(self.get(pos.map2(Vec2::new(j, 2), |e, q| e.max(0.0) as i32 + q))?);

            x[x_idx] = cubic(y0, y1, y2, y3, pos.y.fract() as f32);
        }

        Some(cubic(x[0], x[1], x[2], x[3], pos.x.fract() as f32))
    }

    /// Bilinear interpolation.
    ///
    /// Linear interpolation in both directions (i.e. quadratic interpolation).
    pub fn get_interpolated_bilinear<T, F>(&self, pos: Vec2<i32>, mut f: F) -> Option<T>
    where
        T: Copy + Default + Signed + Float + Add<Output = T> + Mul<f32, Output = T>,
        F: FnMut(&SimChunk) -> T,
    {
        // (i) Find downhill for all four points.
        // (ii) Compute distance from each downhill point and do linear interpolation on their heights.
        // (iii) Compute distance between each neighboring point and do linear interpolation on
        //       their distance-interpolated heights.

        // See http://articles.adsabs.harvard.edu/cgi-bin/nph-iarticle_query?1990A%26A...239..443S&defaultprint=YES&page_ind=0&filetype=.pdf
        //
        // Note that these are only guaranteed monotone in one dimension; fortunately, that is
        // sufficient for our purposes.
        let pos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| {
            e as f64 / sz as f64
        });

        // Orient the chunk in the direction of the most downhill point of the four.  If there is
        // no "most downhill" point, then we don't care.
        let x0 = pos.map2(Vec2::new(0, 0), |e, q| e.max(0.0) as i32 + q);
        let p0 = self.get(x0)?;
        let y0 = f(p0);

        let x1 = pos.map2(Vec2::new(1, 0), |e, q| e.max(0.0) as i32 + q);
        let p1 = self.get(x1)?;
        let y1 = f(p1);

        let x2 = pos.map2(Vec2::new(0, 1), |e, q| e.max(0.0) as i32 + q);
        let p2 = self.get(x2)?;
        let y2 = f(p2);

        let x3 = pos.map2(Vec2::new(1, 1), |e, q| e.max(0.0) as i32 + q);
        let p3 = self.get(x3)?;
        let y3 = f(p3);

        let z0 = y0
            .mul(1.0 - pos.x.fract() as f32)
            .mul(1.0 - pos.y.fract() as f32);
        let z1 = y1.mul(pos.x.fract() as f32).mul(1.0 - pos.y.fract() as f32);
        let z2 = y2.mul(1.0 - pos.x.fract() as f32).mul(pos.y.fract() as f32);
        let z3 = y3.mul(pos.x.fract() as f32).mul(pos.y.fract() as f32);

        Some(z0 + z1 + z2 + z3)
    }
}

pub struct SimChunk {
    pub chaos: f32,
    pub alt: f32,
    pub basement: f32,
    pub water_alt: f32,
    pub downhill: Option<Vec2<i32>>,
    pub flux: f32,
    pub temp: f32,
    pub humidity: f32,
    pub rockiness: f32,
    pub is_cliffs: bool,
    pub near_cliffs: bool,
    pub tree_density: f32,
    pub forest_kind: ForestKind,
    pub spawn_rate: f32,
    pub location: Option<LocationInfo>,
    pub river: RiverData,
    pub is_underwater: bool,

    pub structures: Structures,
}

#[derive(Copy, Clone)]
pub struct RegionInfo {
    pub chunk_pos: Vec2<i32>,
    pub block_pos: Vec2<i32>,
    pub dist: f32,
    pub seed: u32,
}

#[derive(Clone)]
pub struct LocationInfo {
    pub loc_idx: usize,
    pub near: Vec<RegionInfo>,
}

#[derive(Clone)]
pub struct Structures {
    pub town: Option<Arc<TownState>>,
}

impl SimChunk {
    fn generate(posi: usize, gen_ctx: &GenCtx, gen_cdf: &GenCdf) -> Self {
        let pos = uniform_idx_as_vec2(posi);
        let wposf = (pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32)).map(|e| e as f64);

        let _map_edge_factor = map_edge_factor(posi);
        let (_, chaos) = gen_cdf.chaos[posi];
        let alt_pre = gen_cdf.alt[posi] as f32;
        let basement_pre = gen_cdf.basement[posi] as f32;
        let water_alt_pre = gen_cdf.water_alt[posi];
        let downhill_pre = gen_cdf.dh[posi];
        let flux = gen_cdf.flux[posi] as f32;
        let river = gen_cdf.rivers[posi].clone();

        // Can have NaNs in non-uniform part where pure_water returned true.  We just test one of
        // the four in order to find out whether this is the case.
        let (flux_uniform, /*flux_non_uniform*/ _) = gen_cdf.pure_flux[posi];
        let (alt_uniform, _) = gen_cdf.alt_no_water[posi];
        let (temp_uniform, _) = gen_cdf.temp_base[posi];
        let (humid_uniform, _) = gen_cdf.humid_base[posi];

        /* // Vertical difference from the equator (NOTE: "uniform" with much lower granularity than
        // other uniform quantities, but hopefully this doesn't matter *too* much--if it does, we
        // can always add a small x component).
        //
        // Not clear that we want this yet, let's see.
        let latitude_uniform = (pos.y as f32 / WORLD_SIZE.y as f32).sub(0.5).mul(2.0);

        // Even less granular--if this matters we can make the sign affect the quantiy slightly.
        let abs_lat_uniform = latitude_uniform.abs(); */

        // Take the weighted average of our randomly generated base humidity, the scaled
        // negative altitude, and the calculated water flux over this point in order to compute
        // humidity.
        const HUMID_WEIGHTS: [f32; /*3*/2] = [2.0, 1.0/*, 1.0*/];
        let humidity = /*if flux_non_uniform.is_nan() {
            0.0
        } else */{
            cdf_irwin_hall(
                &HUMID_WEIGHTS,
                [humid_uniform, flux_uniform/*, 1.0 - alt_uniform*/],
            )
        };

        // We also correlate temperature negatively with altitude and absolute latitude, using
        // different weighting than we use for humidity.
        const TEMP_WEIGHTS: [f32; 2] = [/*1.5, */ 1.0, 2.0];
        let temp = /*if flux_non_uniform.is_nan() {
            0.0
        } else */{
            cdf_irwin_hall(
                &TEMP_WEIGHTS,
                [
                    temp_uniform,
                    1.0 - alt_uniform, /* 1.0 - abs_lat_uniform*/
                ],
            )
        }
        // Convert to [-1, 1]
        .sub(0.5)
        .mul(2.0);
        /* if (temp - (1.0 - alt_uniform).sub(0.5).mul(2.0)).abs() >= 1e-7 {
            panic!("Halp!");
        } */

        // let height_scale = 1.0; // 1.0 / CONFIG.mountain_scale;
        let mut alt = CONFIG.sea_level.add(alt_pre /*.div(height_scale)*/);
        let basement = CONFIG.sea_level.add(basement_pre /*.div(height_scale)*/);
        let water_alt = CONFIG.sea_level.add(water_alt_pre /*.div(height_scale)*/);
        let downhill = if downhill_pre == -2 {
            None
        } else if downhill_pre < 0 {
            panic!("Uh... shouldn't this never, ever happen?");
        } else {
            Some(
                uniform_idx_as_vec2(downhill_pre as usize)
                    * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
            )
        };

        let cliff = gen_ctx.cliff_nz.get((wposf.div(2048.0)).into_array()) as f32 + chaos * 0.2;

        // Logistic regression.  Make sure x ∈ (0, 1).
        let logit = |x: f64| x.ln() - x.neg().ln_1p();
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
        let logistic_2_base = 3.0f64.sqrt().mul(f64::consts::FRAC_2_PI);
        // Assumes μ = 0, σ = 1
        let logistic_cdf = |x: f64| x.div(logistic_2_base).tanh().mul(0.5).add(0.5);

        let is_underwater = match river.river_kind {
            Some(RiverKind::Ocean) | Some(RiverKind::Lake { .. }) => true,
            Some(RiverKind::River { .. }) => false, // TODO: inspect width
            None => false,
        };
        let river_xy = Vec2::new(river.velocity.x, river.velocity.y).magnitude();
        let river_slope = river.velocity.z / river_xy;
        match river.river_kind {
            Some(RiverKind::River { cross_section }) => {
                if cross_section.x >= 0.5 && cross_section.y >= CONFIG.river_min_height {
                    /* println!(
                        "Big area! Pos area: {:?}, River data: {:?}, slope: {:?}",
                        wposf, river, river_slope
                    ); */
                }
                if river_slope.abs() >= /*1.0*//*3.0.sqrt() / 3.0*/0.25 && cross_section.x >= 1.0 {
                    log::debug!(
                        "Big waterfall! Pos area: {:?}, River data: {:?}, slope: {:?}",
                        wposf,
                        river,
                        river_slope
                    );
                }
            }
            Some(RiverKind::Lake { .. }) => {
                // Forces lakes to be downhill from the land around them, and adds some noise to
                // the lake bed to make sure it's not too flat.
                let lake_bottom_nz = (gen_ctx.small_nz.get((wposf.div(20.0)).into_array()) as f32)
                    .max(-1.0)
                    .min(1.0)
                    .mul(3.0);
                alt = alt.min(water_alt - 5.0) + lake_bottom_nz;
            }
            _ => {}
        }

        // No trees in the ocean, with zero humidity (currently), or directly on bedrock.
        let tree_density = if is_underwater
        /* || alt - basement.min(alt) < 2.0 */
        {
            0.0
        } else {
            let tree_density = (gen_ctx.tree_nz.get((wposf.div(1024.0)).into_array()))
                .mul(1.5)
                .add(1.0)
                .mul(0.5)
                .mul(1.2 - chaos as f64 * 0.95)
                .add(0.05)
                .max(0.0)
                .min(1.0);
            // Tree density should go (by a lot) with humidity.
            if humidity <= 0.0 || tree_density <= 0.0 {
                0.0
            } else if humidity >= 1.0 || tree_density >= 1.0 {
                1.0
            } else {
                // Weighted logit sum.
                logistic_cdf(logit(humidity as f64) + 0.5 * logit(tree_density))
            }
            // rescale to (-0.95, 0.95)
            .sub(0.5)
            .mul(0.95)
            .add(0.5)
        } as f32;

        Self {
            chaos,
            flux,
            alt,
            basement: basement.min(alt),
            water_alt,
            downhill,
            temp,
            humidity,
            rockiness: if true {
                (gen_ctx.rock_nz.get((wposf.div(1024.0)).into_array()) as f32)
                    .sub(0.1)
                    .mul(1.3)
                    .max(0.0)
            } else {
                0.0
            },
            is_underwater,
            is_cliffs: cliff > 0.5 && !is_underwater,
            near_cliffs: cliff > 0.2,
            tree_density,
            forest_kind: if temp > CONFIG.temperate_temp {
                if temp > CONFIG.desert_temp {
                    if humidity > CONFIG.jungle_hum {
                        // Forests in desert temperatures with extremely high humidity
                        // should probably be different from palm trees, but we use them
                        // for now.
                        ForestKind::Palm
                    } else if humidity > CONFIG.forest_hum {
                        ForestKind::Palm
                    } else if humidity > CONFIG.desert_hum {
                        // Low but not desert humidity, so we should really have some other
                        // terrain...
                        ForestKind::Savannah
                    } else {
                        ForestKind::Savannah
                    }
                } else if temp > CONFIG.tropical_temp {
                    if humidity > CONFIG.jungle_hum {
                        if tree_density > 0.0 {
                            // println!("Mangrove: {:?}", wposf);
                        }
                        ForestKind::Mangrove
                    } else if humidity > CONFIG.forest_hum {
                        // NOTE: Probably the wrong kind of tree for this climate.
                        ForestKind::Oak
                    } else if humidity > CONFIG.desert_hum {
                        // Low but not desert... need something besides savannah.
                        ForestKind::Savannah
                    } else {
                        ForestKind::Savannah
                    }
                } else {
                    if humidity > CONFIG.jungle_hum {
                        // Temperate climate with jungle humidity...
                        // https://en.wikipedia.org/wiki/Humid_subtropical_climates are often
                        // densely wooded and full of water.  Semitropical rainforests, basically.
                        // For now we just treet them like other rainforests.
                        ForestKind::Oak
                    } else if humidity > CONFIG.forest_hum {
                        // Moderate climate, moderate humidity.
                        ForestKind::Oak
                    } else if humidity > CONFIG.desert_hum {
                        // With moderate temperature and low humidity, we should probably see
                        // something different from savannah, but oh well...
                        ForestKind::Savannah
                    } else {
                        ForestKind::Savannah
                    }
                }
            } else {
                // For now we don't take humidity into account for cold climates (but we really
                // should!) except that we make sure we only have snow pines when there is snow.
                if temp <= CONFIG.snow_temp {
                    ForestKind::SnowPine
                } else if humidity > CONFIG.desert_hum {
                    ForestKind::Pine
                } else {
                    // Should really have something like tundra.
                    ForestKind::Pine
                }
            },
            spawn_rate: 1.0,
            location: None,
            river,
            structures: Structures { town: None },
        }
    }

    pub fn get_base_z(&self) -> f32 {
        self.alt - self.chaos * 50.0 - 16.0
    }

    pub fn get_name(&self, world: &WorldSim) -> Option<String> {
        if let Some(loc) = &self.location {
            Some(world.locations[loc.loc_idx].name().to_string())
        } else {
            None
        }
    }

    pub fn get_biome(&self) -> BiomeKind {
        if self.alt < CONFIG.sea_level {
            BiomeKind::Ocean
        } else if self.chaos > 0.6 {
            BiomeKind::Mountain
        } else if self.temp > CONFIG.desert_temp {
            BiomeKind::Desert
        } else if self.temp < CONFIG.snow_temp {
            BiomeKind::Snowlands
        } else if self.tree_density > 0.65 {
            BiomeKind::Forest
        } else {
            BiomeKind::Grassland
        }
    }
}
