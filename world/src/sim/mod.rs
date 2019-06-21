mod location;

// Reexports
pub use self::location::Location;

use crate::{all::ForestKind, util::StructureGen2d, CONFIG};
use common::{
    terrain::{BiomeKind, TerrainChunkSize},
    vol::VolSize,
};
use noise::{BasicMulti, HybridMulti, MultiFractal, NoiseFn, RidgedMulti, Seedable, OpenSimplex, SuperSimplex};
use rand::{prng::XorShiftRng, Rng, SeedableRng};
use std::{
    ops::{Add, Div, Mul, Neg, Sub},
    sync::Arc,
};
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

    pub tree_gen: StructureGen2d,
    pub cliff_gen: StructureGen2d,
}

pub struct WorldSim {
    pub seed: u32,
    pub(crate) chunks: Vec<SimChunk>,
    pub(crate) gen_ctx: GenCtx,
    pub rng: XorShiftRng,
}

impl WorldSim {
    pub fn generate(seed: u32) -> Self {
        let mut gen_ctx = GenCtx {
            turb_x_nz: SuperSimplex::new().set_seed(seed + 0),
            turb_y_nz: SuperSimplex::new().set_seed(seed + 1),
            chaos_nz: RidgedMulti::new().set_octaves(7).set_seed(seed + 2),
            hill_nz: SuperSimplex::new().set_seed(seed + 3),
            alt_nz: HybridMulti::new()
                .set_octaves(8)
                .set_persistence(0.1)
                .set_seed(seed + 4),
            temp_nz: SuperSimplex::new().set_seed(seed + 5),
            dry_nz: BasicMulti::new().set_seed(seed + 6),
            small_nz: BasicMulti::new().set_octaves(2).set_seed(seed + 7),
            rock_nz: HybridMulti::new().set_persistence(0.3).set_seed(seed + 8),
            cliff_nz: HybridMulti::new().set_persistence(0.3).set_seed(seed + 9),
            warp_nz: BasicMulti::new().set_octaves(3).set_seed(seed + 10),
            tree_nz: BasicMulti::new()
                .set_octaves(12)
                .set_persistence(0.75)
                .set_seed(seed + 12),
            cave_0_nz: SuperSimplex::new().set_seed(seed + 13),
            cave_1_nz: SuperSimplex::new().set_seed(seed + 14),

            tree_gen: StructureGen2d::new(seed, 32, 24),
            cliff_gen: StructureGen2d::new(seed, 80, 56),
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
            gen_ctx,
            rng: XorShiftRng::from_seed([
                (seed >> 0) as u8,
                0,
                0,
                0,
                (seed >> 8) as u8,
                0,
                0,
                0,
                (seed >> 16) as u8,
                0,
                0,
                0,
                (seed >> 24) as u8,
                0,
                0,
                0,
            ]),
        };

        this.seed_elements();
        this.simulate(0);

        this
    }

    /// Prepare the world for simulation
    pub fn seed_elements(&mut self) {
        let mut rng = self.rng.clone();

        for _ in 0..250 {
            let loc_center = Vec2::new(
                self.rng.gen::<i32>() % WORLD_SIZE.x as i32,
                self.rng.gen::<i32>() % WORLD_SIZE.y as i32,
            );

            if let Some(chunk) = self.get_mut(loc_center) {
                chunk.location = Some(Location::generate(loc_center, &mut rng).into());
            }
        }

        self.rng = rng;
    }

    pub fn simulate(&mut self, cycles: usize) {
        let mut rng = self.rng.clone();

        for _ in 0..cycles {
            for i in 0..WORLD_SIZE.x as i32 {
                for j in 0..WORLD_SIZE.y as i32 {
                    let pos = Vec2::new(i, j);

                    let location = self.get(pos).unwrap().location.clone();

                    let rpos =
                        Vec2::new(rng.gen::<i32>(), rng.gen::<i32>()).map(|e| e.abs() % 3 - 1);

                    if let Some(other) = &mut self.get_mut(pos + rpos) {
                        if other.location.is_none()
                            && rng.gen::<f32>() > other.chaos * 1.5
                            && other.alt > CONFIG.sea_level
                        {
                            other.location = location;
                        }
                    }
                }
            }
        }

        self.rng = rng;
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

const Z_TOLERANCE: (f32, f32) = (100.0, 128.0);

pub struct SimChunk {
    pub chaos: f32,
    pub alt_base: f32,
    pub alt: f32,
    pub temp: f32,
    pub dryness: f32,
    pub rockiness: f32,
    pub cliffs: bool,
    pub near_cliffs: bool,
    pub tree_density: f32,
    pub forest_kind: ForestKind,
    pub location: Option<Arc<Location>>,
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

        let dryness = (gen_ctx.dry_nz.get(
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
        ) as f32);

        let chaos = (gen_ctx.chaos_nz.get((wposf.div(4_000.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5)
            .mul(
                (gen_ctx.chaos_nz.get((wposf.div(6_000.0)).into_array()) as f32)
                    .powf(2.0)
                    .add(0.5)
                    .min(1.0),
            )
            .powf(1.5)
            .add(0.1 * hill);

        let chaos = chaos + chaos.mul(16.0).sin().mul(0.02);

        let alt_base = gen_ctx.alt_nz.get((wposf.div(6_000.0)).into_array()) as f32;
        let alt_base = alt_base
            .mul(0.4)
            .add(alt_base.mul(128.0).sin().mul(0.005))
            .mul(400.0);

        let alt_main = (gen_ctx.alt_nz.get((wposf.div(2_000.0)).into_array()) as f32)
            .abs()
            .powf(1.8);

        let alt = CONFIG.sea_level
            + alt_base
            + (0.0
                + alt_main
                + gen_ctx.small_nz.get((wposf.div(300.0)).into_array()) as f32
                    * alt_main.max(0.1)
                    * chaos
                    * 1.6)
                .add(1.0)
                .mul(0.5)
                .mul(chaos)
                .mul(CONFIG.mountain_scale);

        let temp = (gen_ctx.temp_nz.get((wposf.div(8192.0)).into_array()) as f32);

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
            cliffs: cliff > 0.5 && dryness > 0.05,
            near_cliffs: cliff > 0.4,
            tree_density: (gen_ctx.tree_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .add(1.0)
                .mul(0.5)
                .mul(1.2 - chaos * 0.95)
                .add(0.1)
                .mul(if alt > CONFIG.sea_level + 5.0 {
                    1.0
                } else {
                    0.0
                }),
            forest_kind: if temp > 0.0 {
                if temp > CONFIG.desert_temp {
                    ForestKind::Palm
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
            location: None,
        }
    }

    pub fn get_base_z(&self) -> f32 {
        self.alt - Z_TOLERANCE.0 * self.chaos
    }

    pub fn get_min_z(&self) -> f32 {
        self.alt - Z_TOLERANCE.0 * (self.chaos * 1.2 + 0.3)
    }

    pub fn get_max_z(&self) -> f32 {
        (self.alt + Z_TOLERANCE.1 * if self.near_cliffs { 1.0 } else { 0.5 }).max(CONFIG.sea_level + 2.0)
    }

    pub fn get_name(&self) -> Option<String> {
        self.location.as_ref().map(|l| l.name().to_string())
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
