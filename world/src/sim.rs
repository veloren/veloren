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

    cave_0_nz: SuperSimplex,
    cave_1_nz: SuperSimplex,
}

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
                .set_octaves(8)
                .set_persistence(0.1)
                .set_seed(seed + 4),
            temp_nz: SuperSimplex::new().set_seed(seed + 5),
            small_nz: BasicMulti::new().set_octaves(2).set_seed(seed + 6),
            rock_nz: HybridMulti::new().set_persistence(0.3).set_seed(seed + 7),
            warp_nz: BasicMulti::new().set_octaves(3).set_seed(seed + 8),
            tree_nz: BasicMulti::new()
                .set_octaves(12)
                .set_persistence(0.75)
                .set_seed(seed + 9),
            cave_0_nz: SuperSimplex::new().set_seed(seed + 10),
            cave_1_nz: SuperSimplex::new().set_seed(seed + 11),
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
            tree_gen: StructureGen2d::new(seed, 24, 16),
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
            .sub(0.35)
            .max(0.0)
            .mul(6.0);

        let alt = sim.get_interpolated(wpos, |chunk| chunk.alt)?
            + sim.gen_ctx.small_nz.get((wposf.div(256.0)).into_array()) as f32
                * chaos.max(0.2)
                * 64.0;

        let wposf3d = Vec3::new(wposf.x, wposf.y, alt as f64);

        let marble = (sim.gen_ctx.hill_nz.get((wposf3d.div(48.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5);

        // Colours
        let cold_grass = Rgb::new(0.1, 0.6, 0.3);
        let warm_grass = Rgb::new(0.25, 0.8, 0.05);
        let cold_stone = Rgb::new(0.55, 0.7, 0.75);
        let warm_stone = Rgb::new(0.65, 0.65, 0.35);
        let beach_sand = Rgb::new(0.93, 0.84, 0.33);
        let desert_sand = Rgb::new(0.97, 0.84, 0.23);
        let snow = Rgb::broadcast(1.0);

        let grass = Rgb::lerp(cold_grass, warm_grass, marble);
        let grassland = grass; //Rgb::lerp(grass, warm_stone, rock.mul(5.0).min(0.8));
        let cliff = Rgb::lerp(cold_stone, warm_stone, marble);

        let ground = Rgb::lerp(
            Rgb::lerp(snow, grassland, temp.add(0.4).mul(32.0).sub(0.4)),
            desert_sand,
            temp.sub(0.4).mul(32.0).add(0.4),
        );

        // Caves
        let cave_at = |wposf: Vec2<f64>| {
            (sim.gen_ctx.cave_0_nz.get(
                Vec3::new(wposf.x, wposf.y, alt as f64 * 8.0)
                    .div(800.0)
                    .into_array(),
            ) as f32)
                .powf(2.0)
                .neg()
                .add(1.0)
                .mul((1.35 - chaos).min(1.0))
        };
        let cave_xy = cave_at(wposf);
        let cave_alt = alt - 32.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(48.0).into_array()) as f32)
                * 8.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(300.0).into_array()) as f32)
                .add(1.0)
                .mul(0.5)
                .powf(8.0)
                .mul(256.0);

        Some(Sample2d {
            alt,
            chaos,
            surface_color: Rgb::lerp(
                beach_sand,
                // Land
                Rgb::lerp(
                    ground,
                    // Mountain
                    Rgb::lerp(
                        cliff,
                        snow,
                        (alt - SEA_LEVEL
                            - 0.3 * MOUNTAIN_HEIGHT
                            - alt_base
                            - temp * 96.0
                            - marble * 24.0)
                            / 12.0,
                    ),
                    (alt - SEA_LEVEL - 0.15 * MOUNTAIN_HEIGHT) / 180.0,
                ),
                // Beach
                (alt - SEA_LEVEL - 2.0) / 5.0,
            ),
            tree_density,
            close_trees: sim.tree_gen.sample(wpos),
            cave_xy,
            cave_alt,
            rock,
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
            cave_xy,
            cave_alt,
            rock,
        } = *self.sample_2d(wpos2d)?;

        // Apply warping

        let warp = (self
            .sim
            .gen_ctx
            .warp_nz
            .get((wposf.div(Vec3::new(120.0, 120.0, 150.0))).into_array())
            as f32)
            .mul((chaos - 0.1).max(0.0))
            .mul(110.0);

        let height = alt + warp;

        // Sample blocks

        let air = Block::empty();
        let stone = Block::new(2, Rgb::new(200, 220, 255));
        let dirt = Block::new(1, Rgb::new(128, 90, 0));
        let sand = Block::new(1, Rgb::new(180, 150, 50));
        let water = Block::new(1, Rgb::new(100, 150, 255));
        let warm_stone = Block::new(1, Rgb::new(165, 165, 90));

        let block = if (wposf.z as f32) < height - 4.0 {
            // Underground
            Some(stone)
        } else if (wposf.z as f32) < height {
            // Surface
            Some(Block::new(1, surface_color.map(|e| (e * 255.0) as u8)))
        } else if (wposf.z as f32) < SEA_LEVEL {
            // Ocean
            Some(water)
        } else {
            None
        };

        // Caves
        let block = block.and_then(|block| {
            // Underground
            let cave = cave_xy.powf(2.0)
                * (wposf.z as f32 - cave_alt)
                    .div(40.0)
                    .powf(4.0)
                    .neg()
                    .add(1.0)
                > 0.9993;

            if cave {
                None
            } else {
                Some(block)
            }
        });

        // Rocks
        let block = block.or_else(|| {
            if (height + 2.5 - wposf.z as f32).div(7.5).abs().powf(2.0) < rock {
                Some(warm_stone)
            } else {
                None
            }
        });

        let block = match block {
            Some(block) => block,
            None => (&close_trees)
                .iter()
                .fold(air, |block, (tree_pos, tree_seed)| {
                    match self.sample_2d(*tree_pos) {
                        Some(tree_sample)
                            if tree_sample.tree_density
                                > 0.5 + (*tree_seed as f32 / 1000.0).fract() * 0.2 =>
                        {
                            let tree_pos3d =
                                Vec3::new(tree_pos.x, tree_pos.y, tree_sample.alt as i32);
                            let rpos = wpos - tree_pos3d;
                            block.or(TREES[*tree_seed as usize % TREES.len()]
                                .get((rpos * 160) / 128) // Scaling
                                .map(|b| b.clone())
                                .unwrap_or(Block::empty()))
                        }
                        _ => block,
                    }
                }),
        };

        Some(Sample3d { block })
    }
}

#[derive(Copy, Clone)]
pub struct Sample2d {
    pub alt: f32,
    pub chaos: f32,
    pub surface_color: Rgb<f32>,
    pub tree_density: f32,
    pub close_trees: [(Vec2<i32>, u32); 9],
    pub cave_xy: f32,
    pub cave_alt: f32,
    pub rock: f32,
}

#[derive(Copy, Clone)]
pub struct Sample3d {
    pub block: Block,
}

pub const SEA_LEVEL: f32 = 128.0;
pub const MOUNTAIN_HEIGHT: f32 = 900.0;

const Z_TOLERANCE: (f32, f32) = (64.0, 64.0);

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

        let chaos = (gen_ctx.chaos_nz.get((wposf.div(2_000.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5)
            .powf(1.5)
            .add(0.1 * hill);

        let chaos = chaos + chaos.mul(16.0).sin().mul(0.02);

        let alt_base = gen_ctx.alt_nz.get((wposf.div(6_000.0)).into_array()) as f32;
        let alt_base = alt_base
            .mul(0.4)
            .add(alt_base.mul(128.0).sin().mul(0.005))
            .mul(800.0);

        let alt_main = gen_ctx.alt_nz.get((wposf.div(3_000.0)).into_array()) as f32;

        let alt = SEA_LEVEL
            + alt_base
            + (0.0
                + alt_main
                + gen_ctx.small_nz.get((wposf.div(300.0)).into_array()) as f32
                    * alt_main.max(0.05)
                    * chaos
                    * 1.6)
                .add(1.0)
                .mul(0.5)
                .mul(chaos)
                .mul(MOUNTAIN_HEIGHT);

        Self {
            chaos,
            alt_base,
            alt,
            temp: (gen_ctx.temp_nz.get((wposf.div(8192.0)).into_array()) as f32),
            rockiness: (gen_ctx.rock_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .sub(0.1)
                .mul(1.2)
                .max(0.0),
            tree_density: (gen_ctx.tree_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .add(1.0)
                .mul(0.5)
                .mul(1.0 - chaos * 0.85)
                .mul(1.2)
                .add(0.1)
                .mul(if alt > SEA_LEVEL + 2.0 { 1.0 } else { 0.0 }),
        }
    }

    pub fn get_base_z(&self) -> f32 {
        self.alt
    }

    pub fn get_min_z(&self) -> f32 {
        self.alt - Z_TOLERANCE.0 * (self.chaos + 0.5)
    }

    pub fn get_max_z(&self) -> f32 {
        (self.alt + Z_TOLERANCE.1).max(SEA_LEVEL + 1.0)
    }
}

lazy_static! {
    static ref TREES: [Arc<Structure>; 61] = [
        // green oaks
        assets::load_map("world/tree/oak_green/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/3.vox", |s: Structure| s
            .with_center(Vec3::new(16, 20, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/4.vox", |s: Structure| s
            .with_center(Vec3::new(18, 21, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/5.vox", |s: Structure| s
            .with_center(Vec3::new(18, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/6.vox", |s: Structure| s
            .with_center(Vec3::new(16, 21, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/7.vox", |s: Structure| s
            .with_center(Vec3::new(20, 19, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/8.vox", |s: Structure| s
            .with_center(Vec3::new(22, 20, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/9.vox", |s: Structure| s
            .with_center(Vec3::new(26, 26, 14)))
        .unwrap(),
        // green pines
        assets::load_map("world/tree/pine_green/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // green pines 2
         assets::load_map("world/tree/pine_green_2/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // blue pines
        assets::load_map("world/tree/pine_blue/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // temperate small
        assets::load_map("world/tree/temperate_small/1.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/2.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/3.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/4.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/5.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/6.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        // birch
        assets::load_map("world/tree/birch/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/3.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/4.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/5.vox", |s: Structure| s
            .with_center(Vec3::new(9, 11, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/6.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/8.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/9.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/11.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/12.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 5)))
        .unwrap(),
        // poplar
        assets::load_map("world/tree/poplar/1.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/2.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/3.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/4.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/5.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/6.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/7.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/8.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/9.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/10.vox", |s: Structure| s
            .with_center(Vec3::new(7, 7, 10)))
        .unwrap(),
        // palm trees
        /*assets::load_map("world/tree/desert_palm/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/2.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/3.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/5.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/6.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/8.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/9.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        // snow pines
        assets::load_map("world/tree/snow_pine/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // snow birches -> need roots!
        assets::load_map("world/tree/snow_birch/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/3.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/4.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/5.vox", |s: Structure| s
            .with_center(Vec3::new(9, 11, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/6.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/8.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/9.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/11.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/12.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        // willows
        assets::load_map("world/tree/willow/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 14, 1)))
        .unwrap(),
        assets::load_map("world/tree/willow/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 12, 1)))
        .unwrap(),
        */

    ];
}
