mod location;
mod settlement;
mod util;

// Reexports
pub use self::location::Location;
pub use self::settlement::Settlement;
use self::util::{
    cdf_irwin_hall, uniform_idx_as_vec2, uniform_noise, vec2_as_uniform_idx, InverseCdf,
};

use crate::{
    all::ForestKind,
    generator::TownState,
    util::{seed_expan, FastNoise, Sampler, StructureGen2d},
    CONFIG,
};
use common::{
    terrain::{BiomeKind, TerrainChunkSize},
    vol::VolSize,
};
use noise::{
    BasicMulti, Billow, HybridMulti, MultiFractal, NoiseFn, RidgedMulti, Seedable, SuperSimplex,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::{
    f32,
    ops::{Add, Div, Mul, Neg, Sub},
    sync::Arc,
};
use vek::*;

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

/// Calculates the smallest distance along an axis (x, y) from an edge of
/// the world.  This value is maximal at WORLD_SIZE / 2 and minimized at the extremes
/// (0 or WORLD_SIZE on one or more axes).  It then divides the quantity by cell_size,
/// so the final result is 1 when we are not in a cell along the edge of the world, and
/// ranges between 0 and 1 otherwise (lower when the chunk is closer to the edge).
fn map_edge_factor(posi: usize) -> f32 {
    uniform_idx_as_vec2(posi)
        .map2(WORLD_SIZE.map(|e| e as i32), |e, sz| {
            (sz / 2 - (e - sz / 2).abs()) as f32 / 16.0
        })
        .reduce_partial_min()
        .max(0.0)
        .min(1.0)
}

/// A structure that holds cached noise values and cumulative distribution functions for the input
/// that led to those values.  See the definition of InverseCdf for a description of how to
/// interpret the types of its fields.
struct GenCdf {
    humid_base: InverseCdf,
    temp_base: InverseCdf,
    alt_base: InverseCdf,
    chaos: InverseCdf,
    alt: InverseCdf,
    alt_no_seawater: InverseCdf,
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
            warp_nz: FastNoise::new(gen_seed()), //BasicMulti::new().set_octaves(3).set_seed(gen_seed()),
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

            fast_turb_x_nz: FastNoise::new(gen_seed()),
            fast_turb_y_nz: FastNoise::new(gen_seed()),
        };

        // "Base" of the chunk, to be multiplied by CONFIG.mountain_scale (multiplied value is
        // from -0.25 * (CONFIG.mountain_scale * 1.1) to 0.25 * (CONFIG.mountain_scale * 0.9),
        // but value here is from -0.275 to 0.225).
        let alt_base = uniform_noise(|_, wposf| {
            Some(
                (gen_ctx.alt_nz.get((wposf.div(12_000.0)).into_array()) as f32)
                    .sub(0.1)
                    .mul(0.25),
            )
        });

        // chaos produces a value in [0.1, 1.24].  It is a meta-level factor intended to reflect how
        // "chaotic" the region is--how much weird stuff is going on on this terrain.
        let chaos = uniform_noise(|posi, wposf| {
            // From 0 to 1.6, but the distribution before the max is from -1 and 1, so there is a
            // 50% chance that hill will end up at 0.
            let hill = (0.0
                + gen_ctx
                    .hill_nz
                    .get((wposf.div(1_500.0)).into_array())
                    .mul(1.0) as f32
                + gen_ctx
                    .hill_nz
                    .get((wposf.div(400.0)).into_array())
                    .mul(0.3) as f32)
                .add(0.3)
                .max(0.0);

            Some(
                (gen_ctx.chaos_nz.get((wposf.div(3_000.0)).into_array()) as f32)
                    .add(1.0)
                    .mul(0.5)
                    // [0, 1] * [0.25, 1] = [0, 1] (but probably towards the lower end)
                    .mul(
                        (gen_ctx.chaos_nz.get((wposf.div(6_000.0)).into_array()) as f32)
                            .abs()
                            .max(0.25)
                            .min(1.0),
                    )
                    // Chaos is always increased by a little when we're on a hill (but remember that
                    // hill is 0 about 50% of the time).
                    // [0, 1] + 0.15 * [0, 1.6] = [0, 1.24]
                    .add(0.2 * hill)
                    // We can't have *no* chaos!
                    .max(0.1),
            )
        });

        // We ignore sea level because we actually want to be relative to sea level here and want
        // things in CONFIG.mountain_scale units, but otherwise this is a correct altitude
        // calculation.  Note that this is using the "unadjusted" temperature.
        let alt = uniform_noise(|posi, wposf| {
            // This is the extension upwards from the base added to some extra noise from -1 to 1.
            // The extra noise is multiplied by alt_main (the mountain part of the extension)
            // clamped to [0.25, 1], and made 60% larger (so the extra noise is between [-1.6, 1.6],
            // and the final noise is never more than 160% or less than 40% of the original noise,
            // depending on altitude).
            // Adding this to alt_main thus yields a value between -0.4 (if alt_main = 0 and
            // gen_ctx = -1) and 2.6 (if alt_main = 1 and gen_ctx = 1).  When the generated small_nz
            // value hits -0.625 the value crosses 0, so most of the points are above 0.
            //
            // Then, we add 1 and divide by 2 to get a value between 0.3 and 1.8.
            let alt_main = {
                // Extension upwards from the base.  A positive number from 0 to 1 curved to be
                // maximal at 0.  Also to be multiplied by CONFIG.mountain_scale.
                let alt_main = (gen_ctx.alt_nz.get((wposf.div(2_000.0)).into_array()) as f32)
                    .abs()
                    .powf(1.45);

                (0.0 + alt_main
                    + (gen_ctx.small_nz.get((wposf.div(300.0)).into_array()) as f32)
                        .mul(alt_main.max(0.25))
                        .mul(0.3)
                        .add(1.0)
                        .mul(0.5))
            };

            // Now we can compute the final altitude using chaos.
            // We multiply by chaos clamped to [0.1, 1.24] to get a value between 0.03 and 2.232 for
            // alt_pre, then multiply by CONFIG.mountain_scale and add to the base and sea level to
            // get an adjusted value, then multiply the whole thing by map_edge_factor
            // (TODO: compute final bounds).
            Some((alt_base[posi].1 + alt_main.mul(chaos[posi].1)).mul(map_edge_factor(posi)))
        });

        // Check whether any tiles around this tile are not water (since Lerp will ensure that they
        // are included).
        let pure_water = |posi| {
            let pos = uniform_idx_as_vec2(posi);
            for x in pos.x - 1..=pos.x + 1 {
                for y in pos.y - 1..=pos.y + 1 {
                    if x >= 0 && y >= 0 && x < WORLD_SIZE.x as i32 && y < WORLD_SIZE.y as i32 {
                        let posi = vec2_as_uniform_idx(Vec2::new(x, y));
                        if alt[posi].1.mul(CONFIG.mountain_scale) > 0.0 {
                            return false;
                        }
                    }
                }
            }
            true
        };

        // A version of alt that is uniform over *non-seawater* (or land-adjacent seawater) chunks.
        let alt_no_seawater = uniform_noise(|posi, wposf| {
            if pure_water(posi) {
                None
            } else {
                Some(alt[posi].1)
            }
        });

        // -1 to 1.
        let temp_base = uniform_noise(|posi, wposf| {
            if pure_water(posi) {
                None
            } else {
                Some(gen_ctx.temp_nz.get((wposf.div(12000.0)).into_array()) as f32)
            }
        });

        // 0 to 1, hopefully.
        let humid_base = uniform_noise(|posi, wposf| {
            // Check whether any tiles around this tile are water.
            if pure_water(posi) {
                None
            } else {
                Some(
                    (gen_ctx.humid_nz.get(wposf.div(1024.0).into_array()) as f32)
                        .add(1.0)
                        .mul(0.5),
                )
            }
        });

        let gen_cdf = GenCdf {
            humid_base,
            temp_base,
            alt_base,
            chaos,
            alt,
            alt_no_seawater,
        };

        let mut chunks = Vec::new();
        for i in 0..WORLD_SIZE.x * WORLD_SIZE.y {
            chunks.push(SimChunk::generate(i, &mut gen_ctx, &gen_cdf));
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
            Some(&self.chunks[vec2_as_uniform_idx(chunk_pos)])
        } else {
            None
        }
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
    pub humidity: f32,
    pub rockiness: f32,
    pub is_cliffs: bool,
    pub near_cliffs: bool,
    pub tree_density: f32,
    pub forest_kind: ForestKind,
    pub spawn_rate: f32,
    pub location: Option<LocationInfo>,

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
    fn generate(posi: usize, gen_ctx: &mut GenCtx, gen_cdf: &GenCdf) -> Self {
        let pos = uniform_idx_as_vec2(posi);
        let wposf = (pos * TerrainChunkSize::SIZE.map(|e| e as i32)).map(|e| e as f64);

        let (_, alt_base) = gen_cdf.alt_base[posi];
        let map_edge_factor = map_edge_factor(posi);
        let (_, chaos) = gen_cdf.chaos[posi];
        let (humid_uniform, _) = gen_cdf.humid_base[posi];
        let (_, alt_pre) = gen_cdf.alt[posi];
        let (alt_uniform, _) = gen_cdf.alt_no_seawater[posi];
        let (temp_uniform, _) = gen_cdf.temp_base[posi];

        // Take the weighted average of our randomly generated base humidity, the scaled
        // negative altitude, and other random variable (to add some noise) to yield the
        // final humidity.  Note that we are using the "old" version of chaos here.
        const HUMID_WEIGHTS: [f32; 2] = [1.0, 1.0];
        let humidity = cdf_irwin_hall(&HUMID_WEIGHTS, [humid_uniform, 1.0 - alt_uniform]);

        // We also correlate temperature negatively with altitude using different weighting than we
        // use for humidity.
        const TEMP_WEIGHTS: [f32; 2] = [2.0, 1.0];
        let temp = cdf_irwin_hall(&TEMP_WEIGHTS, [temp_uniform, 1.0 - alt_uniform])
            // Convert to [-1, 1]
            .sub(0.5)
            .mul(2.0);

        let alt_base = alt_base.mul(CONFIG.mountain_scale);
        let alt = CONFIG
            .sea_level
            .mul(map_edge_factor)
            .add(alt_pre.mul(CONFIG.mountain_scale));

        let cliff = gen_ctx.cliff_nz.get((wposf.div(2048.0)).into_array()) as f32 + chaos * 0.2;

        // Logistic regression.  Make sure x ∈ (0, 1).
        let logit = |x: f32| x.ln() - x.neg().ln_1p();
        // 0.5 + 0.5 * tanh(ln(1 / (1 - 0.1) - 1) / (2 * (sqrt(3)/pi)))
        let logistic_2_base = 3.0f32.sqrt().mul(f32::consts::FRAC_2_PI);
        // Assumes μ = 0, σ = 1
        let logistic_cdf = |x: f32| x.div(logistic_2_base).tanh().mul(0.5).add(0.5);

        // No trees in the ocean or with zero humidity (currently)
        let tree_density = if alt <= CONFIG.sea_level + 5.0 {
            0.0
        } else {
            let tree_density = (gen_ctx.tree_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .mul(1.5)
                .add(1.0)
                .mul(0.5)
                .mul(1.2 - chaos * 0.95)
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
                logistic_cdf(logit(humidity) + 0.5 * logit(tree_density))
            }
            // rescale to (-0.95, 0.95)
            .sub(0.5)
            .mul(0.95)
            .add(0.5)
        };

        Self {
            chaos,
            alt_base,
            alt,
            temp,
            humidity,
            rockiness: (gen_ctx.rock_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .sub(0.1)
                .mul(1.3)
                .max(0.0),
            is_cliffs: cliff > 0.5 && alt > CONFIG.sea_level + 5.0,
            near_cliffs: cliff > 0.25,
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
                if temp <= CONFIG.snow_temp && humidity > CONFIG.forest_hum {
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
