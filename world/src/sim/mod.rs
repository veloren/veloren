mod erosion;
mod location;
mod settlement;
mod util;

// Reexports
pub use self::erosion::{
    do_erosion, fill_sinks, get_drainage, get_lakes, get_rivers, RiverData, RiverKind,
};
pub use self::location::Location;
pub use self::settlement::Settlement;
pub use self::util::{
    cdf_irwin_hall, downhill, get_oceans, local_cells, map_edge_factor, neighbors,
    uniform_idx_as_vec2, uniform_noise, uphill, vec2_as_uniform_idx, InverseCdf,
};

use crate::{
    all::ForestKind,
    column::ColumnGen,
    generator::TownState,
    util::{seed_expan, FastNoise, RandomField, Sampler, StructureGen2d},
    CONFIG,
};
use common::{
    terrain::{BiomeKind, TerrainChunkSize},
    vol::RectVolSize,
};
use noise::{
    BasicMulti, Billow, Fbm, HybridMulti, MultiFractal, NoiseFn, RidgedMulti, Seedable,
    SuperSimplex,
};
use num::{Float, Signed};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use rayon::prelude::*;
use std::{
    collections::HashMap,
    f32, f64,
    ops::{Add, Div, Mul, Neg, Sub},
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
    x: 1024 * 2,
    y: 1024 * 2,
};

/// A structure that holds cached noise values and cumulative distribution functions for the input
/// that led to those values.  See the definition of InverseCdf for a description of how to
/// interpret the types of its fields.
struct GenCdf {
    humid_base: InverseCdf,
    temp_base: InverseCdf,
    chaos: InverseCdf,
    alt: Box<[f32]>,
    water_alt: Box<[f32]>,
    dh: Box<[isize]>,
    /// NOTE: Until we hit 4096 × 4096, this should suffice since integers with an absolute value
    /// under 2^24 can be exactly represented in an f32.
    flux: Box<[f32]>,
    pure_flux: InverseCdf,
    alt_no_water: InverseCdf,
    rivers: Box<[RiverData]>,
}

pub(crate) struct GenCtx {
    pub turb_x_nz: SuperSimplex,
    pub turb_y_nz: SuperSimplex,
    pub chaos_nz: RidgedMulti,
    pub alt_nz: HybridMulti,
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
}

pub struct WorldSim {
    pub seed: u32,
    pub(crate) chunks: Vec<SimChunk>,
    pub(crate) locations: Vec<Location>,

    pub(crate) gen_ctx: GenCtx,
    pub rng: ChaChaRng,
}

impl WorldSim {
    pub fn generate(seed: u32) -> Self {
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
        let continent_scale = 5_000.0/*32768.0*/;

        let gen_ctx = GenCtx {
            turb_x_nz: SuperSimplex::new().set_seed(rng.gen()),
            turb_y_nz: SuperSimplex::new().set_seed(rng.gen()),
            chaos_nz: RidgedMulti::new()
                .set_octaves(/*7*//*3*/ 7)
                .set_frequency(
                    /*RidgedMulti::DEFAULT_FREQUENCY **/ 3_000.0 * 8.0 / continent_scale,
                )
                .set_seed(rng.gen()),
            hill_nz: SuperSimplex::new().set_seed(rng.gen()),
            alt_nz: HybridMulti::new()
                .set_octaves(/*3*//*2*/ 8)
                // 1/2048*32*1024 = 16
                .set_frequency(
                    /*HybridMulti::DEFAULT_FREQUENCY*/
                    (10_000.0/* * 2.0*/ / continent_scale) as f64,
                )
                // .set_frequency(1.0 / ((1 << 0) as f64))
                // .set_lacunarity(1.0)
                .set_persistence(/*0.5*//*0.5*/ 0.5)
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
        };

        let river_seed = RandomField::new(rng.gen());
        let rock_strength_nz = Fbm::new()
            .set_octaves(8)
            .set_persistence(/*0.9*/ 2.0)
            .set_frequency(/*0.9*/ Fbm::DEFAULT_FREQUENCY / (64.0 * 32.0))
            .set_seed(rng.gen());

        let max_erosion_per_delta_t = 32.0 / CONFIG.mountain_scale as f64;
        let erosion_pow_low = /*0.25*//*1.5*//*2.0*//*0.5*//*4.0*//*0.25*//*1.0*//*2.0*//*1.5*//*1.5*//*0.35*//*0.43*//*0.5*//*0.45*//*0.37*/1.002;
        let erosion_pow_high = /*1.5*//*1.0*//*0.55*//*0.51*//*2.0*/1.002;
        let erosion_center = /*0.45*//*0.75*//*0.75*//*0.5*//*0.75*/0.5;
        let n_steps = 150; //150;//200;

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
                            .max(-1.0))
                        .sub(0.05)
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
                            .get((wposf.div(1_500.0)).into_array())
                            .min(1.0)
                            .max(-1.0)
                            .mul(1.0)
                        + gen_ctx
                            .hill_nz
                            .get((wposf.div(400.0)).into_array())
                            .min(1.0)
                            .max(-1.0)
                            .mul(0.3))
                    .add(0.3)
                    .max(0.0);

                    // chaos produces a value in [0.12, 1.24].  It is a meta-level factor intended to
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
                        // [0, 1] + 0.15 * [0, 1.6] = [0, 1.24]
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
                .powf(1.35);

                fn spring(x: f64, pow: f64) -> f64 {
                    x.abs().powf(pow) * x.signum()
                }

                (0.0 + alt_main/*0.4*/
                    + (gen_ctx
                        .small_nz
                        .get((wposf.div(300.0)).into_array())
                        .min(1.0)
                        .max(-1.0))
                    .mul(alt_main.powf(0.8).max(/*0.25*/ 0.15))
                    .mul(0.3)
                    .add(1.0)
                    .mul(0.4)
                    /*0.52*/
                    + spring(alt_main.abs().powf(0.5).min(0.75).mul(60.0).sin(), 4.0).mul(0.045))
            };

            // Now we can compute the final altitude using chaos.
            // We multiply by chaos clamped to [0.1, 1.24] to get a value between [0.03, 2.232]
            // for alt_pre, then multiply by CONFIG.mountain_scale and add to the base and sea
            // level to get an adjusted value, then multiply the whole thing by map_edge_factor
            // (TODO: compute final bounds).
            //
            // [-.3675, .3325] + [-0.445, 0.565] * [0.12, 1.24]^1.2
            // ~ [-.3675, .3325] + [-0.445, 0.565] * [_, 1.30]
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
        let old_height = |posi: usize| alt_old[posi].1;
        let is_ocean = get_oceans(old_height);
        let is_ocean_fn = |posi: usize| is_ocean[posi];

        // Recalculate altitudes without oceans.
        // NaNs in these uniform vectors wherever pure_water() returns true.
        let (alt_old_no_ocean, alt_old_inverse) = uniform_noise(|posi, _| {
            if is_ocean_fn(posi) {
                None
            } else {
                Some(old_height(posi) /*.abs()*/)
            }
        });

        let old_height_uniform = |posi: usize| alt_old_no_ocean[posi].0;
        let alt_old_min_uniform = 0.0;
        let alt_old_max_uniform = 1.0;
        let alt_old_center_uniform = erosion_center;
        let (_alt_old_min_index, alt_old_min) = alt_old_inverse.first().unwrap();
        let (_alt_old_max_index, alt_old_max) = alt_old_inverse.last().unwrap();
        let (_alt_old_mid_index, alt_old_mid) =
            alt_old_inverse[(alt_old_inverse.len() as f64 * erosion_center) as usize];
        let alt_old_center =
            ((alt_old_mid - alt_old_min) as f64 / (alt_old_max - alt_old_min) as f64);

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

        // Logistic regression.  Make sure x ∈ (0, 1).
        let logit = |x: f64| x.ln() - (-x).ln_1p();
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
        let logistic_2_base = 3.0f64.sqrt() * f64::consts::FRAC_2_PI;
        let logistic_base = /*3.0f64.sqrt() * f64::consts::FRAC_1_PI*/1.0f64;
        // Assumes μ = 0, σ = 1
        let logistic_cdf = |x: f64| (x / logistic_2_base).tanh() * 0.5 + 0.5;

        let exp_inverse_cdf = |x: f64/*, pow: f64*/| -(-x).ln_1p()/* / ln(pow)*/;
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
        let min_epsilon =
            1.0 / (WORLD_SIZE.x as f64 * WORLD_SIZE.y as f64).max(f64::EPSILON as f64 * 0.5);
        let max_epsilon = (1.0 - 1.0 / (WORLD_SIZE.x as f64 * WORLD_SIZE.y as f64))
            .min(1.0 - f64::EPSILON as f64 * 0.5);
        let alt_exp_min_uniform = exp_inverse_cdf(min_epsilon);
        let alt_exp_max_uniform = exp_inverse_cdf(max_epsilon);

        // let erosion_pow = 2.0;
        // let n_steps = 100;//150;
        // let erosion_factor = |x: f64| logistic_cdf(erosion_pow * logit(x));
        let log_odds = |x: f64| {
            logit(x)
                - logit(
                    /*erosion_center*/ alt_old_center_uniform, /*alt_old_center*/
                )
        };
        /* let erosion_factor = |x: f64| logistic_cdf(logistic_base * if x <= /*erosion_center*/alt_old_center_uniform/*alt_old_center*/ { erosion_pow_low.ln() } else { erosion_pow_high.ln() } * log_odds(x))/*0.5 + (x - 0.5).signum() * ((x - 0.5).mul(2.0).abs(
        ).powf(erosion_pow).mul(0.5))*/; */
        let erosion_factor = |x: f64| (/*if x <= /*erosion_center*/alt_old_center_uniform/*alt_old_center*/ { erosion_pow_low.ln() } else { erosion_pow_high.ln() } * */(exp_inverse_cdf(x) - alt_exp_min_uniform) / (alt_exp_max_uniform - alt_exp_min_uniform))/*0.5 + (x - 0.5).signum() * ((x - 0.5).mul(2.0).abs(
).powf(erosion_pow).mul(0.5))*/;
        let alt = do_erosion(
            0.0,
            max_erosion_per_delta_t as f32,
            n_steps,
            &river_seed,
            &rock_strength_nz,
            |posi| {
                if is_ocean_fn(posi) {
                    old_height(posi)
                } else {
                    let wposf = (uniform_idx_as_vec2(posi)
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
                            .mul(0.4)
                            + spring(alt_main.abs().powf(0.5).min(0.75).mul(60.0).sin(), 4.0)
                                .mul(0.045))
                    };
                    // old_height_uniform(posi) *
                    (/*((old_height(posi) - alt_old_min) as f64 / (alt_old_max - alt_old_min) as f64) **/(((6.0 / 360.0 * 2.0 * f64::consts::PI).tan()
                        * TerrainChunkSize::RECT_SIZE.reduce_partial_min() as f64)
                        .floor()
                        / CONFIG.mountain_scale as f64)) as f32
                    // 5.0 / CONFIG.mountain_scale
                }
            },
            is_ocean_fn,
            |posi| {
                if is_ocean_fn(posi) {
                    return 0.0;
                }
                let wposf = (uniform_idx_as_vec2(posi)
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
                        .mul(0.4)
                        + spring(alt_main.abs().powf(0.5).min(0.75).mul(60.0).sin(), 4.0)
                            .mul(0.045))
                };
                let height =
                    ((old_height_uniform(posi) - alt_old_min_uniform) as f64
                    / (alt_old_max_uniform - alt_old_min_uniform) as f64)
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
                /*height < f32::EPSILON as f64 * 0.5/*false*/*/
                false {
                    (
                        /*(alt_main./*to_le_bytes()[7]*/to_bits() & 1) as f64*/
                        (alt_main / CONFIG.mountain_scale as f64 * 128.0).mul(0.1).powf(1.2) * /*(1.0 / CONFIG.mountain_scale as f64)*/(f32::EPSILON * 0.5) as f64,
                        (f32::EPSILON * 0.5) as f64,
                    )
                } else {
                    (0.0, 0.0)
                };
                let height = height
                    .mul(max_erosion_per_delta_t * 7.0 / 8.0)
                    .add(max_erosion_per_delta_t / 8.0)
                    .sub(/*1.0 / CONFIG.mountain_scale as f64*/ bump_max)
                    .add(bump_factor);
                /* .sub(/*1.0 / CONFIG.mountain_scale as f64*/(f32::EPSILON * 0.5) as f64)
                .add(bump_factor); */
                height as f32
            },
        );
        let is_ocean = get_oceans(|posi| alt[posi]);
        let is_ocean_fn = |posi: usize| is_ocean[posi];
        let mut dh = downhill(&alt, /*old_height*/ is_ocean_fn);
        let (boundary_len, indirection, water_alt_pos) = get_lakes(&/*water_alt*/alt, &mut dh);
        let flux_old = get_drainage(&water_alt_pos, &dh, boundary_len);

        let water_height_initial = |chunk_idx| {
            let indirection_idx = indirection[chunk_idx];
            // Find the lake this point is flowing into.
            let lake_idx = if indirection_idx < 0 {
                chunk_idx
            } else {
                indirection_idx as usize
            };
            // Find the pass this lake is flowing into (i.e. water at the lake bottom gets
            // pushed towards the point identified by pass_idx).
            let neighbor_pass_idx = dh[lake_idx];
            let chunk_water_alt = if neighbor_pass_idx < 0 {
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

                // Find the height of the pass into which our lake is flowing.
                let pass_height_j = alt[neighbor_pass_idx as usize];
                // Find the height of "our" side of the pass (the part of it that drains into this
                // chunk's lake).
                let pass_idx = -indirection[lake_idx];
                let pass_height_i = alt[pass_idx as usize];
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
        let rivers = get_rivers(&water_alt_pos, &water_alt, &dh, &indirection, &flux_old);

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
                // Find the pass this lake is flowing into (i.e. water at the lake bottom gets
                // pushed towards the point identified by pass_idx).
                let neighbor_pass_idx = dh[lake_idx];
                if neighbor_pass_idx < 0 {
                    // This is either a boundary node (dh[chunk_idx] == -2, i.e. water is at sea level)
                    // or part of a lake that flows directly into the ocean.  In the former case, water
                    // is at sea level so we just return 0.0.  In the latter case, the lake bottom must
                    // have been a boundary node in the first place--meaning this node flows directly
                    // into the ocean.  In that case, its lake bottom is ocean, meaning its water is
                    // also at sea level.  Thus, we return 0.0 in both cases.
                    0.0
                } else {
                    // This is not flowing into the ocean, so we can use the existing water_alt.
                    water_alt[chunk_idx]
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
                                Some(alt[posi])
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

        this.seed_elements();

        this
    }

    /// Draw a map of the world based on chunk information.  Returns a buffer of u32s.
    pub fn get_map(&self) -> Vec<u32> {
        (0..WORLD_SIZE.x * WORLD_SIZE.y)
            .into_par_iter()
            .map(|chunk_idx| {
                let pos = uniform_idx_as_vec2(chunk_idx);

                let (alt, water_alt, river_kind) = self
                    .get(pos)
                    .map(|sample| (sample.alt, sample.water_alt, sample.river.river_kind))
                    .unwrap_or((CONFIG.sea_level, CONFIG.sea_level, None));
                let alt = ((alt - CONFIG.sea_level) / CONFIG.mountain_scale)
                    .min(1.0)
                    .max(0.0);
                let water_alt = ((alt.max(water_alt) - CONFIG.sea_level) / CONFIG.mountain_scale)
                    .min(1.0)
                    .max(0.0);
                match river_kind {
                    Some(RiverKind::Ocean) => u32::from_le_bytes([64, 32, 0, 255]),
                    Some(RiverKind::Lake { .. }) => u32::from_le_bytes([
                        64 + (water_alt * 191.0) as u8,
                        32 + (water_alt * 95.0) as u8,
                        0,
                        255,
                    ]),
                    Some(RiverKind::River { .. }) => u32::from_le_bytes([
                        64 + (alt * 191.0) as u8,
                        32 + (alt * 95.0) as u8,
                        0,
                        255,
                    ]),
                    None => u32::from_le_bytes([0, (alt * 255.0) as u8, 0, 255]),
                }
            })
            .collect()
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
        for _ in 0..loc_count {
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
        }

        // Find neighbours
        let mut loc_clone = locations
            .iter()
            .map(|l| l.center)
            .enumerate()
            .collect::<Vec<_>>();
        for i in 0..locations.len() {
            let pos = locations[i].center.map(|e| e as i64);

            loc_clone.sort_by_key(|(_, l)| l.map(|e| e as i64).distance_squared(pos));

            loc_clone.iter().skip(1).take(2).for_each(|(j, _)| {
                locations[i].neighbours.insert(*j);
                locations[*j].neighbours.insert(i);
            });
        }

        // Simulate invasion!
        let invasion_cycles = 25;
        for _ in 0..invasion_cycles {
            for i in 0..grid_size.x {
                for j in 0..grid_size.y {
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
                }
            }
        }

        // Place the locations onto the world
        let gen = StructureGen2d::new(self.seed, cell_size as u32, cell_size as u32 / 2);
        for i in 0..WORLD_SIZE.x {
            for j in 0..WORLD_SIZE.y {
                let chunk_pos = Vec2::new(i as i32, j as i32);
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
                    self.get_mut(chunk_pos).unwrap().location = loc_grid
                        .get(nearest_cell_pos.y * grid_size.x + nearest_cell_pos.x)
                        .cloned()
                        .unwrap_or(None)
                        .map(|loc_idx| LocationInfo { loc_idx, near });

                    let town_size = 200;
                    let in_town = self
                        .get(chunk_pos)
                        .unwrap()
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
                        self.get_mut(chunk_pos).unwrap().spawn_rate = 0.0;
                    }
                }
            }
        }

        // Stage 2 - towns!
        let mut maybe_towns = HashMap::new();
        for i in 0..WORLD_SIZE.x {
            for j in 0..WORLD_SIZE.y {
                let chunk_pos = Vec2::new(i as i32, j as i32);
                let wpos = chunk_pos.map2(Vec2::from(TerrainChunkSize::RECT_SIZE), |e, sz: u32| {
                    e * sz as i32 + sz as i32 / 2
                });

                let near_towns = self.gen_ctx.town_gen.get(wpos);
                let town = near_towns
                    .iter()
                    .min_by_key(|(pos, _seed)| wpos.distance_squared(*pos));

                if let Some((pos, _)) = town {
                    let maybe_town = maybe_towns
                        .entry(*pos)
                        .or_insert_with(|| {
                            // println!("Town: {:?}", town);
                            TownState::generate(*pos, &mut ColumnGen::new(self), &mut rng)
                                .map(|t| Arc::new(t))
                        })
                        .as_mut()
                        // Only care if we're close to the town
                        .filter(|town| {
                            Vec2::from(town.center()).distance_squared(wpos)
                                < town.radius().add(64).pow(2)
                        })
                        .cloned();
                    self.get_mut(chunk_pos).unwrap().structures.town = maybe_town;
                }
            }
        }

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
        let alt_pre = gen_cdf.alt[posi];
        let water_alt_pre = gen_cdf.water_alt[posi];
        let downhill_pre = gen_cdf.dh[posi];
        let flux = gen_cdf.flux[posi];
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
        const HUMID_WEIGHTS: [f32; /*3*/2] = [4.0, 1.0/*, 1.0*/];
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

        let mut alt = CONFIG.sea_level.add(alt_pre.mul(CONFIG.mountain_scale));
        let water_alt = CONFIG
            .sea_level
            .add(water_alt_pre.mul(CONFIG.mountain_scale));
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
                if river_slope.abs() >= 1.0 && cross_section.x >= 1.0 {
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

        // No trees in the ocean or with zero humidity (currently)
        let tree_density = if is_underwater {
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
            forest_kind: if temp > 0.0 {
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
