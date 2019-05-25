use crate::{structure::StructureGen2d, Cache};
use common::{
    assets,
    terrain::{Block, Structure, TerrainChunkSize},
    vol::{ReadVol, VolSize, Vox},
};
use lazy_static::lazy_static;
use noise::{
    BasicMulti, HybridMulti, MultiFractal, NoiseFn, OpenSimplex, RidgedMulti, Seedable,
    SuperSimplex,
};
use std::{
    f32,
    ops::{Add, Div, Mul, Neg, Sub},
    sync::Arc,
};
use vek::*;

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

pub struct WorldSim {
    pub seed: u32,
    chunks: Vec<SimChunk>,
    gen_ctx: GenCtx,
    tree_gen: StructureGen2d,
}

impl WorldSim {
    pub fn generate(seed: u32) -> Self {
        let mut gen_ctx = GenCtx {
            turb_x_nz: BasicMulti::new().set_seed(seed + 0),
            turb_y_nz: BasicMulti::new().set_seed(seed + 1),
            chaos_nz: RidgedMulti::new().set_octaves(7).set_seed(seed + 2),
            hill_nz: SuperSimplex::new().set_seed(seed + 3),
            alt_nz: HybridMulti::new()
                .set_octaves(7)
                .set_persistence(0.1)
                .set_seed(seed + 4),
            temp_nz: SuperSimplex::new().set_seed(seed + 5),
            small_nz: BasicMulti::new().set_octaves(2).set_seed(seed + 6),
            rock_nz: HybridMulti::new().set_persistence(0.3).set_seed(seed + 7),
            warp_nz: BasicMulti::new().set_octaves(3).set_seed(seed + 8),
            tree_nz: BasicMulti::new().set_octaves(6).set_seed(seed + 9),
        };

        let mut chunks = Vec::new();
        for x in 0..WORLD_SIZE.x as u32 {
            for y in 0..WORLD_SIZE.y as u32 {
                chunks.push(SimChunk::generate(Vec2::new(x, y), &mut gen_ctx));
            }
        }

        Self {
            seed,
            chunks,
            gen_ctx,
            tree_gen: StructureGen2d::new(seed, 32, 32),
        }
    }

    pub fn get(&self, chunk_pos: Vec2<u32>) -> Option<&SimChunk> {
        if chunk_pos
            .map2(WORLD_SIZE, |e, sz| e < sz as u32)
            .reduce_and()
        {
            Some(&self.chunks[chunk_pos.y as usize * WORLD_SIZE.x + chunk_pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_base_z(&self, chunk_pos: Vec2<u32>) -> Option<f32> {
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
            let y0 =
                f(self.get(pos.map2(Vec2::new(j, -1), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let y1 = f(self.get(pos.map2(Vec2::new(j, 0), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let y2 = f(self.get(pos.map2(Vec2::new(j, 1), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let y3 = f(self.get(pos.map2(Vec2::new(j, 2), |e, q| (e.max(0.0) as i32 + q) as u32))?);

            x[x_idx] = cubic(y0, y1, y2, y3, pos.y.fract() as f32);
        }

        Some(cubic(x[0], x[1], x[2], x[3], pos.x.fract() as f32))
    }

    pub fn sampler(&self) -> Sampler {
        Sampler {
            sim: self,
            sample2d_cache: Cache::with_capacity(1024),
        }
    }
}

pub struct Sampler<'a> {
    sim: &'a WorldSim,
    sample2d_cache: Cache<Vec2<i32>, Option<Sample2d>>,
}

impl<'a> Sampler<'a> {
    fn sample_2d_impl(sim: &WorldSim, wpos: Vec2<i32>) -> Option<Sample2d> {
        let wposf = wpos.map(|e| e as f64);

        let alt_base = sim.get_interpolated(wpos, |chunk| chunk.alt_base)?;
        let chaos = sim.get_interpolated(wpos, |chunk| chunk.chaos)?;
        let temp = sim.get_interpolated(wpos, |chunk| chunk.temp)?;
        let rockiness = sim.get_interpolated(wpos, |chunk| chunk.rockiness)?;
        let tree_density = sim.get_interpolated(wpos, |chunk| chunk.tree_density)?;

        let rock = (sim.gen_ctx.small_nz.get((wposf.div(100.0)).into_array()) as f32)
            .mul(rockiness)
            .sub(0.2)
            .max(0.0)
            .mul(2.0);

        let alt = sim.get_interpolated(wpos, |chunk| chunk.alt)?
            + sim.gen_ctx.small_nz.get((wposf.div(256.0)).into_array()) as f32
                * chaos.max(0.2)
                * 64.0
            + rock * 15.0;

        let wposf3d = Vec3::new(wposf.x, wposf.y, alt as f64);

        let marble = (sim.gen_ctx.hill_nz.get((wposf3d.div(64.0)).into_array()) as f32)
            .mul(0.5)
            .add(1.0)
            .mul(0.5);

        // Colours
        let cold_grass = Rgb::new(0.05, 0.5, 0.3);
        let warm_grass = Rgb::new(0.4, 1.0, 0.05);
        let cold_stone = Rgb::new(0.55, 0.75, 0.9);
        let warm_stone = Rgb::new(0.75, 0.6, 0.35);
        let sand = Rgb::new(0.93, 0.84, 0.33);
        let snow = Rgb::broadcast(1.0);

        let grass = Rgb::lerp(cold_grass, warm_grass, temp);
        let ground = Rgb::lerp(grass, warm_stone, rock.mul(5.0).min(0.8));
        let cliff = Rgb::lerp(cold_stone, warm_stone, marble);

        Some(Sample2d {
            alt,
            chaos,
            surface_color: Rgb::lerp(
                sand,
                // Land
                Rgb::lerp(
                    ground,
                    // Mountain
                    Rgb::lerp(
                        cliff,
                        snow,
                        (alt - SEA_LEVEL - 350.0 - alt_base - temp * 48.0) / 12.0,
                    ),
                    (alt - SEA_LEVEL - 150.0) / 180.0,
                ),
                // Beach
                (alt - SEA_LEVEL - 2.0) / 5.0,
            ),
            tree_density,
            close_trees: sim.tree_gen.sample(wpos),
        })
    }

    pub fn sample_2d(&mut self, wpos2d: Vec2<i32>) -> Option<&Sample2d> {
        let sim = &self.sim;
        self.sample2d_cache
            .get(wpos2d, |wpos2d| Self::sample_2d_impl(sim, wpos2d))
            .as_ref()
    }

    pub fn sample_3d(&mut self, wpos: Vec3<i32>) -> Option<Sample3d> {
        let wpos2d = Vec2::from(wpos);
        let wposf = wpos.map(|e| e as f64);

        // Sample 2D terrain attributes

        let Sample2d {
            alt,
            chaos,
            surface_color,
            tree_density,
            close_trees,
        } = *self.sample_2d(wpos2d)?;

        // Apply warping

        let warp = (self
            .sim
            .gen_ctx
            .warp_nz
            .get((wposf.div(Vec3::new(120.0, 120.0, 150.0))).into_array())
            as f32)
            .mul((chaos - 0.1).max(0.0))
            .mul(90.0);

        let height = alt + warp;
        let temp = 0.0;

        // Sample blocks

        let air = Block::empty();
        let stone = Block::new(1, Rgb::new(200, 220, 255));
        let grass = Block::new(2, Rgb::new(75, 150, 0));
        let dirt = Block::new(3, Rgb::new(128, 90, 0));
        let sand = Block::new(4, Rgb::new(180, 150, 50));
        let water = Block::new(5, Rgb::new(100, 150, 255));

        let above_ground =
            (&close_trees)
                .iter()
                .fold(air, |block, (tree_pos, tree_seed)| {
                    match self.sample_2d(*tree_pos) {
                        Some(tree_sample) if tree_sample.tree_density > 0.5 => {
                            let tree_pos3d =
                                Vec3::new(tree_pos.x, tree_pos.y, tree_sample.alt as i32);
                            block.or(TREES[*tree_seed as usize % TREES.len()]
                                .get(wpos - tree_pos3d)
                                .map(|b| b.clone())
                                .unwrap_or(Block::empty()))
                        }
                        _ => block,
                    }
                });

        let z = wposf.z as f32;
        Some(Sample3d {
            block: if z < height - 4.0 {
                stone
            } else if z < height {
                Block::new(1, surface_color.map(|e| (e * 255.0) as u8))
            } else if z < SEA_LEVEL {
                water
            } else {
                above_ground
            },
        })
    }
}

lazy_static! {
    static ref TREES: [Arc<Structure>; 12] = [
        assets::load_map("world/tree/oak/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak/3.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine/3.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine/4.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine/5.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/temperate/1.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate/2.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate/3.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate/4.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate/5.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate/6.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
    ];
}

#[derive(Copy, Clone)]
pub struct Sample2d {
    pub alt: f32,
    pub chaos: f32,
    pub surface_color: Rgb<f32>,
    pub tree_density: f32,
    pub close_trees: [(Vec2<i32>, u32); 9],
}

#[derive(Copy, Clone)]
pub struct Sample3d {
    pub block: Block,
}

struct GenCtx {
    turb_x_nz: BasicMulti,
    turb_y_nz: BasicMulti,
    chaos_nz: RidgedMulti,
    alt_nz: HybridMulti,
    hill_nz: SuperSimplex,
    temp_nz: SuperSimplex,
    small_nz: BasicMulti,
    rock_nz: HybridMulti,
    warp_nz: BasicMulti,
    tree_nz: BasicMulti,
}

const Z_TOLERANCE: (f32, f32) = (48.0, 64.0);
pub const SEA_LEVEL: f32 = 128.0;

pub struct SimChunk {
    pub chaos: f32,
    pub alt_base: f32,
    pub alt: f32,
    pub temp: f32,
    pub rockiness: f32,
    pub tree_density: f32,
}

impl SimChunk {
    fn generate(pos: Vec2<u32>, gen_ctx: &mut GenCtx) -> Self {
        let wposf = (pos * Vec2::from(TerrainChunkSize::SIZE)).map(|e| e as f64);

        let hill = (0.0
            + gen_ctx
                .hill_nz
                .get((wposf.div(3_500.0)).into_array())
                .mul(1.0) as f32
            + gen_ctx
                .hill_nz
                .get((wposf.div(1_000.0)).into_array())
                .mul(0.3) as f32)
            .add(0.3)
            .max(0.0);

        let chaos = (gen_ctx.chaos_nz.get((wposf.div(4_000.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5)
            .powf(1.9)
            .add(0.25 * hill);

        let chaos = chaos + chaos.mul(16.0).sin().mul(0.02);

        let alt_base = gen_ctx.alt_nz.get((wposf.div(6_000.0)).into_array()) as f32;
        let alt_base = alt_base
            .mul(0.4)
            .add(alt_base.mul(128.0).sin().mul(0.004))
            .mul(600.0);

        let alt_main = gen_ctx.alt_nz.get((wposf.div(1_500.0)).into_array()) as f32;

        Self {
            chaos,
            alt_base,
            alt: SEA_LEVEL
                + alt_base
                + (0.0
                    + alt_main
                    + gen_ctx.small_nz.get((wposf.div(300.0)).into_array()) as f32
                        * alt_main.max(0.05)
                        * chaos
                        * 1.3)
                    .add(1.0)
                    .mul(0.5)
                    .mul(chaos)
                    .mul(1200.0),
            temp: (gen_ctx.temp_nz.get((wposf.div(48.0)).into_array()) as f32)
                .add(1.0)
                .mul(0.5),
            rockiness: (gen_ctx.rock_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .sub(0.1)
                .mul(1.2)
                .max(0.0),
            tree_density: (gen_ctx.tree_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .add(1.0)
                .mul(0.5)
                .mul(1.0 - chaos),
        }
    }

    pub fn get_base_z(&self) -> f32 {
        self.alt - Z_TOLERANCE.0 * (self.chaos + 0.1) - 3.0
    }

    pub fn get_max_z(&self) -> f32 {
        self.alt + Z_TOLERANCE.1
    }
}
