mod location;
mod settlement;

// Reexports
pub use self::location::Location;
pub use self::settlement::Settlement;

use crate::{
    all::ForestKind,
    util::{seed_expan, Sampler, StructureGen2d},
    CONFIG,
};
use common::{
    terrain::{BiomeKind, TerrainChunkSize},
    vol::VolSize,
};
use noise::{BasicMulti, Billow, HybridMulti, MultiFractal, NoiseFn, RidgedMulti, Seedable, SuperSimplex};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use statrs::distribution::{
    InverseGamma,
    LogNormal,
    Gamma,
    Normal,
    Univariate,
};
use std::{
    f32,
    ops::{Add, Div, Mul, Neg, Sub},
};
use vek::*;

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

/// Computes the cumulative distribution function of the weighted sum of k independent,
/// uniformly distributed random variables between 0 and 1.  For each variable i, we use weights[i]
/// as the weight to give samples[i] (the weights should all be positive).
///
/// If the precondition is met, the distribution of the result of calling this function will be
/// uniformly distributed while preserving the same information that was in the original average.
///
/// NOTE: For N > 33 the function will no longer return correct results since we will overflow u32.
fn cdf_irwin_hall<const N : usize>(weights: &[f32; N], samples: [f32; N]) -> f32 {
    // Take the average of the weights
    // (to scale the weights down so their sum is in the (0..=N) range).
    let avg = weights.iter().sum::<f32>() / N as f32;
    // Take the sum.
    let x : f32 =
        weights.iter().zip(samples.iter()).map(|(weight, sample)| weight / avg * sample).sum();
    // CDF = 1 / N! * Σ{k = 0 to floor(x)} ((-1)^k (N choose k) (x - k) ^ N)
    let mut binom = 1; // (-1)^0 * (n choose 0) = 1 * 1 = 1
    let mut y = x.powi(N as i32); // 1 * (x - 0)^N = x ^N
    // 1..floor(x)
    for k in (1..=x.floor() as i32) {
        // (-1)^k (N choose k) = ((-1)^(k-1) (N choose (k - 1))) * -(N + 1 - k) / k for k ≥ 1.
        binom *= -(N as i32 + 1 - k) / k;
        y += binom as f32 * (x - k as f32).powi(N as i32);
    }
    // Remember to multiply by 1 / N! at the end.
    y / (1..=N as i32).product::<i32>() as f32
}

pub(crate) struct GenCtx {
    pub turb_x_nz: SuperSimplex,
    pub turb_y_nz: SuperSimplex,
    pub chaos_nz: RidgedMulti,
    pub alt_nz: HybridMulti,
    pub hill_nz: SuperSimplex,
    pub temp_nz: SuperSimplex,
    // Fresh groundwater (currently has no effect, but should influence humidity)
    pub dry_nz: BasicMulti,
    // Humidity noise
    pub humid_nz : Billow,
    // Small amounts of noise for simulating rough terrain.
    pub small_nz: BasicMulti,
    pub rock_nz: HybridMulti,
    pub cliff_nz: HybridMulti,
    pub warp_nz: BasicMulti,
    pub tree_nz: BasicMulti,

    pub cave_0_nz: SuperSimplex,
    pub cave_1_nz: SuperSimplex,

    pub structure_gen: StructureGen2d,
    pub region_gen: StructureGen2d,
    pub cliff_gen: StructureGen2d,
}

pub struct WorldSim {
    pub seed: u32,
    pub(crate) chunks: Vec<SimChunk>,
    pub(crate) locations: Vec<Location>,

    pub(crate) gen_ctx: GenCtx,
    pub rng: ChaChaRng,
}

impl WorldSim {
    pub fn generate(mut seed: u32) -> Self {
        let mut seed = &mut seed;
        let mut gen_seed = || {
            *seed = seed_expan::diffuse(*seed);
            *seed
        };

        let mut gen_ctx = GenCtx {
            turb_x_nz: SuperSimplex::new().set_seed(gen_seed()),
            turb_y_nz: SuperSimplex::new().set_seed(gen_seed()),
            chaos_nz: RidgedMulti::new().set_octaves(7).set_seed(gen_seed()),
            hill_nz: SuperSimplex::new().set_seed(gen_seed()),
            alt_nz: HybridMulti::new()
                .set_octaves(8)
                .set_persistence(0.1)
                .set_seed(gen_seed()),
            temp_nz: SuperSimplex::new().set_seed(gen_seed()),
            dry_nz: BasicMulti::new().set_seed(gen_seed()),
            small_nz: BasicMulti::new().set_octaves(2).set_seed(gen_seed()),
            rock_nz: HybridMulti::new().set_persistence(0.3).set_seed(gen_seed()),
            cliff_nz: HybridMulti::new().set_persistence(0.3).set_seed(gen_seed()),
            warp_nz: BasicMulti::new().set_octaves(3).set_seed(gen_seed()),
            tree_nz: BasicMulti::new()
                .set_octaves(12)
                .set_persistence(0.75)
                .set_seed(gen_seed()),
            cave_0_nz: SuperSimplex::new().set_seed(gen_seed()),
            cave_1_nz: SuperSimplex::new().set_seed(gen_seed()),

            structure_gen: StructureGen2d::new(gen_seed(), 32, 24),
            region_gen: StructureGen2d::new(gen_seed(), 400, 96),
            cliff_gen: StructureGen2d::new(gen_seed(), 80, 56),
            humid_nz: Billow::new()
                .set_octaves(12)
                .set_persistence(0.125)
                .set_frequency(1.0)
                // .set_octaves(6)
                // .set_persistence(0.5)
                .set_seed(gen_seed()),
        };

        let mut chunks = Vec::new();
        for x in 0..WORLD_SIZE.x as i32 {
            for y in 0..WORLD_SIZE.y as i32 {
                chunks.push(SimChunk::generate(Vec2::new(x, y), &mut gen_ctx));
            }
        }

        let mut this = Self {
            seed: *seed,
            chunks,
            locations: Vec::new(),
            gen_ctx,
            rng: ChaChaRng::from_seed(seed_expan::rng_state(*seed)),
        };

        this.seed_elements();

        this
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
                .map2(Vec2::from(TerrainChunkSize::SIZE), |e, sz: u32| {
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
            let pos = locations[i].center;

            loc_clone.sort_by_key(|(_, l)| l.distance_squared(pos));

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
                        let loc = Vec2::new(i as i32 + R_COORDS[idx], j as i32 + R_COORDS[idx + 1])
                            .map(|e| e as usize);

                        loc_grid[j * grid_size.x + i] =
                            loc_grid.get(loc.y * grid_size.x + loc.x).cloned().flatten();
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
                    chunk_pos.x * TerrainChunkSize::SIZE.x as i32,
                    chunk_pos.y * TerrainChunkSize::SIZE.y as i32,
                );
                let _cell_pos = Vec2::new(i / cell_size, j / cell_size);

                // Find the distance to each region
                let near = gen.get(chunk_pos);
                let mut near = near
                    .iter()
                    .map(|(pos, seed)| RegionInfo {
                        chunk_pos: *pos,
                        block_pos: pos.map2(Vec2::from(TerrainChunkSize::SIZE), |e, sz: u32| {
                            e * sz as i32
                        }),
                        dist: (pos - chunk_pos).map(|e| e as f32).magnitude(),
                        seed: *seed,
                    })
                    .collect::<Vec<_>>();

                // Sort regions based on distance
                near.sort_by(|a, b| a.dist.partial_cmp(&b.dist).unwrap());

                let nearest_cell_pos = near[0].chunk_pos.map(|e| e as usize) / cell_size;
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
                        locations[l.loc_idx].center.distance_squared(block_pos)
                            < town_size * town_size
                    })
                    .unwrap_or(false);
                if in_town {
                    self.get_mut(chunk_pos).unwrap().spawn_rate = 0.0;
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
            Some(&self.chunks[chunk_pos.y as usize * WORLD_SIZE.x + chunk_pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, chunk_pos: Vec2<i32>) -> Option<&mut SimChunk> {
        if chunk_pos
            .map2(WORLD_SIZE, |e, sz| e >= 0 && e < sz as i32)
            .reduce_and()
        {
            Some(&mut self.chunks[chunk_pos.y as usize * WORLD_SIZE.x + chunk_pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_base_z(&self, chunk_pos: Vec2<i32>) -> Option<f32> {
        self.get(chunk_pos).and_then(|_| {
            (0..2)
                .map(|i| (0..2).map(move |j| (i, j)))
                .flatten()
                .map(|(i, j)| {
                    self.get(chunk_pos + Vec2::new(i, j))
                        .map(|c| c.get_base_z())
                })
                .flatten()
                .fold(None, |a: Option<f32>, x| a.map(|a| a.min(x)).or(Some(x)))
        })
    }

    pub fn get_interpolated<T, F>(&self, pos: Vec2<i32>, mut f: F) -> Option<T>
    where
        T: Copy + Default + Add<Output = T> + Mul<f32, Output = T>,
        F: FnMut(&SimChunk) -> T,
    {
        let pos = pos.map2(TerrainChunkSize::SIZE.into(), |e, sz: u32| {
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
}

pub struct SimChunk {
    pub chaos: f32,
    pub alt_base: f32,
    pub alt: f32,
    pub temp: f32,
    pub dryness: f32,
    pub humidity: f32,
    pub rockiness: f32,
    pub is_cliffs: bool,
    pub near_cliffs: bool,
    pub tree_density: f32,
    pub forest_kind: ForestKind,
    pub spawn_rate: f32,
    pub location: Option<LocationInfo>,
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

impl SimChunk {
    fn generate(pos: Vec2<i32>, gen_ctx: &mut GenCtx) -> Self {
        let wposf = (pos * TerrainChunkSize::SIZE.map(|e| e as i32)).map(|e| e as f64);

        // From 0 to 1.6, but the distribution before the max is from -1 and 1, so there is a 50%
        // chance that hill will end up at 0.
        let hill = (0.0
            + gen_ctx
                .hill_nz
                .get((wposf.div(1_500.0)).into_array())
                .mul(1.0) as f32
            + gen_ctx
                .hill_nz
                .get((wposf.div(500.0)).into_array())
                .mul(0.3) as f32)
            .add(0.3)
            .max(0.0);

        // FIXME: Currently unused, but should represent fresh groundwater level.
        // Should be correlated a little with humidity, somewhat negatively with altitude,
        // and very negatively with difference in temperature from zero.
        let dryness = gen_ctx.dry_nz.get(
            (wposf
                .add(Vec2::new(
                    gen_ctx
                        .dry_nz
                        .get((wposf.add(10000.0).div(500.0)).into_array())
                        * 150.0,
                    gen_ctx.dry_nz.get((wposf.add(0.0).div(500.0)).into_array()) * 150.0,
                ))
                .div(2_000.0))
            .into_array(),
        ) as f32;

        // "Base" of the chunk, to be multiplied by CONFIG.mountain_scale (multiplied value is
        // from -0.25 * (CONFIG.mountain_scale * 1.1) to 0.25 * (CONFIG.mountain_scale * 0.9),
        // but value here is from -0.275 to 0.225).
        let alt_base_pre = (gen_ctx.alt_nz.get((wposf.div(12_000.0)).into_array()) as f32);

        let alt_base = alt_base_pre
            .sub(0.1)
            .mul(0.25);

        // Extension upwards from the base.  A positive number from 0 to 1 curved to be maximal at
        // 0.
        let alt_main_pre = (gen_ctx.alt_nz.get((wposf.div(2_000.0)).into_array()) as f32);
        let alt_main = alt_main_pre
            .abs()
            .powf(1.35);

        // Calculates the smallest distance along an axis (x, y) from an edge of
        // the world.  This value is maximal at WORLD_SIZE / 2 and minimized at the extremes
        // (0 or WORLD_SIZE on one or more axes).  It then divides the quantity by cell_size,
        // so the final result is 1 when we are not in a cell along the edge of the world, and
        // ranges between 0 and 1 otherwise (lower when the chunk is closer to the edge).
        let map_edge_factor = pos
            .map2(WORLD_SIZE.map(|e| e as i32), |e, sz| {
                (sz / 2 - (e - sz / 2).abs()) as f32 / 16.0
            })
            .reduce_partial_min()
            .max(0.0)
            .min(1.0);

        // chaos produces a value in [0.1, 1.24].  It is a meta-level factor intended to reflect how
        // "chaotic" the region is--how much weird stuff is going on on this terrain.
        //
        // First, we calculate chaos_pre, which is chaos with no filter and no temperature
        // flattening (so it is between [0, 1.24] instead of [0.1, 1.24].  This is used to break
        // the cyclic dependency between temperature and altitude (altitude relies on chaos, which
        // relies on temperature, but we also want temperature to rely on altitude.  We recompute
        // altitude with the temperature incorporated after we figure out temperature).
        let chaos_pre = (gen_ctx.chaos_nz.get((wposf.div(3_000.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5)
            // [0, 1] * [0.25, 1] = [0, 1] (but probably towards the lower end)
            .mul(
                (gen_ctx.chaos_nz.get((wposf.div(6_000.0)).into_array()) as f32)
                    .abs()
                    .max(0.25)
                    .min(1.0),
            )
            // Chaos is always increased by a little when we're on a hill (but remember that hill
            // is 0 about 50% of the time).
            // [0, 1] + 0.15 * [0, 1.6] = [0, 1.24]
            .add(0.15 * hill);

        // This is the extension upwards from the base added to some extra noise from -1 to 1.
        // The extra noise is multiplied by alt_main (the base part of the extension) clamped to
        // be between 0.25 and 1, and made 60% larger (so the extra noise is between -1.6 and 1.6,
        // and the final noise is never more than 160% or less than 40% of the original noise,
        // depending on altitude).
        // Adding this to alt_main thus yields a value between -0.4 (if alt_main = 0 and
        // gen_ctx = -1) and 2.6 (if alt_main = 1 and gen_ctx = 1).  When the generated small_nz
        // value hits -0.625 the value crosses 0, so most of the points are above 0.
        //
        // Then, we add 1 and divide by 2 to get a value between 0.3 and 1.8.
        let alt_pre = (0.0
                + alt_main
                + (gen_ctx.small_nz.get((wposf.div(300.0)).into_array()) as f32)
                    .mul(alt_main.max(0.25))
                    .mul(1.6))
            .add(1.0)
            .mul(0.5);

        // 0 to 1, hopefully.
        let humid_base =
            (gen_ctx.humid_nz.get(wposf.div(1024.0).into_array()) as f32)
            .add(1.0)
            .mul(0.5)
            as f32;

        // Ideally, humidity is correlated negatively with altitude and slightly positively with
        // dryness.  For now we just do "negatively with altitude."  We currently opt not to have
        // it affected by temperature.  Negative humidity is lower, positive humidity is higher.
        //
        // Because we want to start at 0, rise, and then saturate at 1, we use a cumulative logistic
        // distribution, calculated as:
        //
        // 1/2 + 1/2 * tanh((x - μ) / (2s))
        //
        // where x is the random variable (altitude relative to sea level without mountain
        // scaling), μ is the altitude where humidity should be at its midpoint (currently set to 0.125),
        // and s is the scale parameter proportional to the standard deviation σ of the humidity
        // function of altitude (s = √3/π * σ).  Currently we set σ to -0.0625, so we get ~ 68% of
        // the variation due to altitude between .0625 * mountain_scale above sea level and
        // 0.1875 * mountain_scale above sea level (it is negative to make the distribution higher when
        // the altitude is lower).
        let humid_alt_sigma = -0.0625;
        let humid_alt_2s = 3.0f32.sqrt().mul(f32::consts::FRAC_2_PI).mul(humid_alt_sigma);
        let humid_alt_mu = 0.125;
        // We ignore sea level because we actually want to be relative to sea level here and want
        // things in CONFIG.mountain_scale units, and we are using the version of chaos that doesn't
        // know about temperature.  Otherwise, this is a correct altitude calculation.
        let humid_alt_pre = (alt_base + alt_pre.mul(chaos_pre.max(0.1))) * map_edge_factor;
        let humid_alt = humid_alt_pre
            .sub(humid_alt_mu)
            .div(humid_alt_2s)
            .tanh()
            .mul(0.5)
            .add(0.5);

        // The log-logistic distribution (a variable whose logarithm has a logistic distribution) is often
        // used to model stream flow rates and precipitation as a tractable analogue of a log-normal
        // distribution.  We use it here for humidity.
        //
        // Specifically, we treat altitude
        //
        // For a log-logistic distribution, you have
        //
        // X = e^
        //
        // where α is a scale parameter (the median of the distribution, where μ = ln(α)), β is a
        // shape parameter related to the standard deviation (s = 1 / β)
        //
        // Start with e^(altitude difference) to get values in (0, 1) for low altitudes (-∞, e) and
        // in [1, ∞) for high altitudes [e, ∞).
        //
        // The produced variable is in a log-normal distribution (that is, X's *logarithm* is
        // normally distributed).
        //
        // https://en.wikipedia.org/wiki/Log-logistic_distribution
        //
        // A log-logistic distribution represents the probability distribution of a random variable
        // whose logarithm has a logistic distribution.
        //
        // That is, ln X varies smoothly from 0 to 1 along an S-curve.
        //
        // Now we can
        //
        // 1 to
        // for high.
        // We want negative values for altitude to represent
        //
        // e^-2
        //
        // (alt mag)^(climate mag)
        //
        // (2)^(-1)
        //

        // Take the weighted average of our randomly generated base humidity, the scaled
        // negative altitude, and other random variable (to add some noise) to yield the
        // final humidity.
        const WEIGHTS : [f32; 4] = [3.0, 1.0, 1.0, 1.0];
        let humidity = cdf_irwin_hall(
            &WEIGHTS,
            [humid_base,
             alt_main_pre.mul(0.5).add(0.5),
             alt_base_pre.mul(0.5).add(0.5),
             (gen_ctx.small_nz.get((wposf.div(500.0)).into_array()) as f32)
                                         .mul(0.5)
                                         .add(0.5)]
        );
        /* // Now we just take a (currently) unweighted average of our randomly generated base humidity
        // (from scaled to be from 0 to 1) and our randomly generated "base" humidity.  We can
        // adjust this weighting factor as desired.
        let humid_weight = 3.0;
        let humid_alt_weight = 1.0;
        let humidity =
            humid_base.mul(humid_weight)
            .add(humid_alt
                    .mul(humid_alt_weight)
                    // Adds some noise to the humidity effect of altitude to dampen it.
                    .mul((gen_ctx.small_nz.get((wposf.div(500.0)).into_array()) as f32)
                         .mul(0.5)
                         .add(0.5)))
            .div(humid_weight + humid_alt_weight); */

        let temp_base =
            gen_ctx.temp_nz.get((wposf.div(12000.0)).into_array()) as f32;
        // We also correlate temperature negatively with altitude using a different computed factor
        // that we use for humidity (and with different weighting).  We could definitely make the
        // distribution different for temperature as well.
        let temp_alt_sigma = -0.0625;
        let temp_alt_2s = 3.0f32.sqrt().mul(f32::consts::FRAC_2_PI).mul(temp_alt_sigma);
        let temp_alt_mu = 0.0625;
        // Scaled to [-1, 1] already.
        let temp_alt = humid_alt_pre
            .sub(temp_alt_mu)
            .div(temp_alt_2s)
            .tanh();
        let temp_weight = 2.0;
        let temp_alt_weight = 1.0;
        let temp =
            temp_base.mul(temp_weight)
            .add(temp_alt.mul(temp_alt_weight))
            .div(temp_weight + temp_alt_weight);

        // Now, we finish the computation of chaos incorporating temperature information, producing
        // a value in [0.1, 1.24].
        let chaos = chaos_pre
            // [0, 1.24] * [0.35, 1.0] = [0, 1.24].
            // Sharply decreases (towards 0.35) when temperature is near desert_temp (from below),
            // then saturates just before it actually becomes desert.  Otherwise stays at 1.
            .mul(
                temp.sub(CONFIG.desert_temp)
                    .neg()
                    .mul(12.0)
                    .max(0.35)
                    .min(1.0),
            )
            // We can't have *no* chaos!
            .max(0.1);

        // Now we can recompute altitude using the correct verison of chaos.
        // We multiply by chaos clamped to [0.1, 1.24] to get a value between 0.03 and 2.232 for
        // alt_pre, then multiply by CONFIG.mountain_scale and add to the base and sea level to get
        // an adjusted value, then multiply the whole thing by map_edge_factor (TODO: compute final bounds).
        let alt_base = alt_base.mul(CONFIG.mountain_scale);
        let alt =
            CONFIG.sea_level
            .add(alt_base)
            .add(alt_pre.mul(chaos).mul(CONFIG.mountain_scale))
            .mul(map_edge_factor);

        let cliff = gen_ctx.cliff_nz.get((wposf.div(2048.0)).into_array()) as f32 + chaos * 0.2;

        let tree_density =
            (gen_ctx.tree_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .mul(1.5)
                .add(1.0)
                .mul(0.5)
                .mul(1.2 - chaos * 0.95)
                .add(0.05)
                .max(0.0)
                .min(1.0)
                .mul(0.4)
                // Tree density should go (by a lot) with humidity.
                .add(humidity.mul(0.6))
                // No trees in the ocean (currently), no trees in true deserts.
                .mul(if alt > CONFIG.sea_level + 5.0 && humidity > CONFIG.desert_hum {
                    1.0
                } else {
                    0.0
                })
                .max(0.0);

        // let humid_normal = InverseGamma::new(4.0, 0.1).unwrap();
        // let humid_normal = LogNormal::new(0.0, 0.1).unwrap();
        let humid_normal = Gamma::new(1.0, 0.5).unwrap();
        // let humid_normal = Gamma::new(0.1, 1.0).unwrap();
        // let humid_normal = Normal::new(0.5, 0.05).unwrap();
        /*if humid_normal.cdf(humid_base as f64) > 0.9 *//* {
            println!("HIGH HUMIDITY: {:?}", humid_base);
        } */
        if pos == Vec2::new(1023, 1023) {
            let mut noise = (0..1024*1024).map( |i| {
                let wposf = Vec2::new(i as f64 / 1024.0, i as f64 % 1024.0);
                gen_ctx.humid_nz.get(wposf.div(1024.0).into_array()) as f32
            } ).collect::<Vec<_>>();
            noise.sort_unstable_by( |f, g| f.partial_cmp(g).unwrap() );
            for (k, f) in noise.iter().enumerate().step_by(1024 * 1024 / 100) {
                println!("{:?}%: {:?}, ", k / (1024 * 1024 / 100), f);
            }
        }
        /* if alt_main_pre.mul(0.5).add(0.5) > 0.7 {
            println!("HIGH: {:?}", alt_main_pre);
        } */
        /* if humidity > CONFIG.jungle_hum {
            println!("JUNGLE");
        } */

        Self {
            chaos,
            alt_base,
            alt,
            temp,
            dryness,
            humidity,
            rockiness: (gen_ctx.rock_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .sub(0.1)
                .mul(1.3)
                .max(0.0),
            is_cliffs: cliff > 0.5
                && dryness > 0.05
                && alt > CONFIG.sea_level + 5.0
                && dryness.abs() > 0.075,
            near_cliffs: cliff > 0.25,
            tree_density,
            forest_kind: if temp > 0.0 {
                if temp > CONFIG.desert_temp {
                    // println!("Any desert: {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                    if humidity > CONFIG.jungle_hum {
                        // Forests in desert temperatures with extremely high humidity
                        // should probably be different from palm trees, but we use them
                        // for now.
                        /* /*if tree_density > 0.0 */{
                            println!("Palm trees (jungle): {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                        } */
                        ForestKind::Palm
                     } else if humidity > CONFIG.forest_hum {
                        /* /*if tree_density > 0.0 */{
                            println!("Palm trees (forest): {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                        } */
                        ForestKind::Palm
                    } else {
                        // Low but not desert humidity, so we should really have some other
                        // terrain...
                        /* if humidity < CONFIG.desert_hum {
                            println!("True desert: {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                        } else {
                            println!("Savannah (desert): {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                        } */
                        ForestKind::Savannah
                    }
                } else if temp > CONFIG.tropical_temp {
                    if humidity > CONFIG.jungle_hum {
                        /* if tree_density > 0.0 {
                            println!("Mangroves: {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                        } */
                        ForestKind::Mangrove
                    } else if humidity > CONFIG.forest_hum {
                        // NOTE: Probably the wrong kind of tree for this climate.
                        ForestKind::Oak
                    } else {
                        // Low but not desert... need something besides savannah.
                        ForestKind::Savannah
                    }
                } else {
                    if humidity > CONFIG.jungle_hum {
                        // Temperate climate with jungle humidity...
                        // https://en.wikipedia.org/wiki/Humid_subtropical_climates are often
                        // densely wooded and full of water.  Semitropical rainforests, basically.
                        // For now we just treet them like other rainforests.
                        /* if tree_density > 0.0 {
                            println!("Mangroves (forest): {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                        } */
                        ForestKind::Mangrove
                    } else if humidity > CONFIG.forest_hum {
                        // Moderate climate, moderate humidity.
                        ForestKind::Oak
                    } else {
                        /* if humidity < CONFIG.desert_hum {
                            println!("True desert: {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                        } */
                        // With moderate temperature and low humidity, we should probably see
                        // something different from savannah, but oh well...
                        ForestKind::Savannah
                    }
                }
            } else {
                // For now we don't take humidity into account for cold climates (but we really
                // should!) except that we make sure we only have snow pines when there is snow.
                if temp <= CONFIG.snow_temp && humidity > CONFIG.forest_hum {
                    /* if tree_density > 0.0 {
                        println!("SnowPine: {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                    } */
                    ForestKind::SnowPine
                } else {
                    /* if humidity < CONFIG.desert_hum {
                        println!("True desert: {:?}, altitude: {:?}, humidity: {:?}, temperature: {:?}, density: {:?}", wposf, alt, humidity, temp, tree_density);
                    } */
                    ForestKind::Pine
                }
            },
            spawn_rate: 1.0,
            location: None,
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
