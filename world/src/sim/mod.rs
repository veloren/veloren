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
use noise::{BasicMulti, HybridMulti, MultiFractal, NoiseFn, RidgedMulti, Seedable, SuperSimplex};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use std::ops::{Add, Div, Mul, Neg, Sub};
use vek::*;

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

pub(crate) struct GenCtx {
    pub turb_x_nz: SuperSimplex,
    pub turb_y_nz: SuperSimplex,
    pub chaos_nz: RidgedMulti,
    pub alt_nz: HybridMulti,
    pub hill_nz: SuperSimplex,
    pub temp_nz: SuperSimplex,
    pub dry_nz: BasicMulti,
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
        let mut gen_seed = || {
            seed = seed_expan::diffuse(seed + 1);
            seed
        };

        let mut gen_ctx = GenCtx {
            turb_x_nz: SuperSimplex::new().set_seed(gen_seed()),
            turb_y_nz: SuperSimplex::new().set_seed(gen_seed()),
            chaos_nz: RidgedMulti::new()
                .set_octaves(7)
                .set_seed(gen_seed()),
            hill_nz: SuperSimplex::new().set_seed(gen_seed()),
            alt_nz: HybridMulti::new()
                .set_octaves(8)
                .set_persistence(0.1)
                .set_seed(gen_seed()),
            temp_nz: SuperSimplex::new().set_seed(gen_seed()),
            dry_nz: BasicMulti::new().set_seed(gen_seed()),
            small_nz: BasicMulti::new()
                .set_octaves(2)
                .set_seed(gen_seed()),
            rock_nz: HybridMulti::new()
                .set_persistence(0.3)
                .set_seed(gen_seed()),
            cliff_nz: HybridMulti::new()
                .set_persistence(0.3)
                .set_seed(gen_seed()),
            warp_nz: BasicMulti::new()
                .set_octaves(3)
                .set_seed(gen_seed()),
            tree_nz: BasicMulti::new()
                .set_octaves(12)
                .set_persistence(0.75)
                .set_seed(gen_seed()),
            cave_0_nz: SuperSimplex::new().set_seed(gen_seed()),
            cave_1_nz: SuperSimplex::new().set_seed(gen_seed()),

            structure_gen: StructureGen2d::new(gen_seed(), 32, 24),
            region_gen: StructureGen2d::new(gen_seed(), 400, 96),
            cliff_gen: StructureGen2d::new(gen_seed(), 80, 56),
        };

        let mut chunks = Vec::new();
        for x in 0..WORLD_SIZE.x as i32 {
            for y in 0..WORLD_SIZE.y as i32 {
                chunks.push(SimChunk::generate(Vec2::new(x, y), &mut gen_ctx));
            }
        }

        let mut this = Self {
            seed,
            chunks,
            locations: Vec::new(),
            gen_ctx,
            rng: ChaChaRng::from_seed(seed_expan::rng_state(seed)),
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

        let temp = gen_ctx.temp_nz.get((wposf.div(12000.0)).into_array()) as f32;

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

        let chaos = (gen_ctx.chaos_nz.get((wposf.div(3_000.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5)
            .mul(
                (gen_ctx.chaos_nz.get((wposf.div(6_000.0)).into_array()) as f32)
                    .abs()
                    .max(0.25)
                    .min(1.0),
            )
            .add(0.15 * hill)
            .mul(
                temp.sub(CONFIG.desert_temp)
                    .neg()
                    .mul(12.0)
                    .max(0.35)
                    .min(1.0),
            )
            .max(0.1);

        let alt_base = (gen_ctx.alt_nz.get((wposf.div(12_000.0)).into_array()) as f32)
            .mul(250.0)
            .sub(25.0);

        let alt_main = (gen_ctx.alt_nz.get((wposf.div(2_000.0)).into_array()) as f32)
            .abs()
            .powf(1.35);

        let map_edge_factor = pos
            .map2(WORLD_SIZE.map(|e| e as i32), |e, sz| {
                (sz / 2 - (e - sz / 2).abs()) as f32 / 16.0
            })
            .reduce_partial_min()
            .max(0.0)
            .min(1.0);

        let alt = (CONFIG.sea_level
            + alt_base
            + (0.0
                + alt_main
                + (gen_ctx.small_nz.get((wposf.div(300.0)).into_array()) as f32)
                    .mul(alt_main.max(0.25))
                    .mul(1.6))
            .add(1.0)
            .mul(0.5)
            .mul(chaos)
            .mul(CONFIG.mountain_scale))
            * map_edge_factor;

        let cliff = gen_ctx.cliff_nz.get((wposf.div(2048.0)).into_array()) as f32 + chaos * 0.2;

        Self {
            chaos,
            alt_base,
            alt,
            temp,
            dryness,
            rockiness: (gen_ctx.rock_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .sub(0.1)
                .mul(1.3)
                .max(0.0),
            is_cliffs: cliff > 0.5
                && dryness > 0.05
                && alt > CONFIG.sea_level + 5.0
                && dryness.abs() > 0.075,
            near_cliffs: cliff > 0.25,
            tree_density: (gen_ctx.tree_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .mul(1.5)
                .add(1.0)
                .mul(0.5)
                .mul(1.2 - chaos * 0.95)
                .add(0.05)
                .mul(if alt > CONFIG.sea_level + 5.0 {
                    1.0
                } else {
                    0.0
                })
                .max(0.0),
            forest_kind: if temp > 0.0 {
                if temp > CONFIG.desert_temp {
                    ForestKind::Palm
                } else if temp > CONFIG.tropical_temp {
                    ForestKind::Savannah
                } else {
                    ForestKind::Oak
                }
            } else {
                if temp > CONFIG.snow_temp {
                    ForestKind::Pine
                } else {
                    ForestKind::SnowPine
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
