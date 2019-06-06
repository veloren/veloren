use crate::{
    structure::StructureGen2d,
    Cache,
    sampler::Sampler,
};
use common::{
    terrain::{Block, Structure, TerrainChunkSize},
    vol::{ReadVol, VolSize, Vox},
};
use lazy_static::lazy_static;
use noise::{BasicMulti, HybridMulti, MultiFractal, NoiseFn, RidgedMulti, Seedable, SuperSimplex};
use std::{
    f32,
    ops::{Add, Div, Mul, Neg, Sub},
};
use vek::*;

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

pub(crate) struct GenCtx {
    pub turb_x_nz: BasicMulti,
    pub turb_y_nz: BasicMulti,
    pub chaos_nz: RidgedMulti,
    pub alt_nz: HybridMulti,
    pub hill_nz: SuperSimplex,
    pub temp_nz: SuperSimplex,
    pub small_nz: BasicMulti,
    pub rock_nz: HybridMulti,
    pub warp_nz: BasicMulti,
    pub tree_nz: BasicMulti,

    pub cave_0_nz: SuperSimplex,
    pub cave_1_nz: SuperSimplex,
}

pub struct WorldSim {
    pub seed: u32,
    pub(crate) chunks: Vec<SimChunk>,
    pub(crate) gen_ctx: GenCtx,
    pub(crate) tree_gen: StructureGen2d,
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
        Sampler::new(self)
    }
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
                .add(0.1)
                .mul(if alt > SEA_LEVEL + 2.0 { 1.0 } else { 0.0 }),
        }
    }

    pub fn get_base_z(&self) -> f32 {
        self.alt - Z_TOLERANCE.0 * self.chaos
    }

    pub fn get_min_z(&self) -> f32 {
        self.alt - Z_TOLERANCE.0 * (self.chaos + 0.5)
    }

    pub fn get_max_z(&self) -> f32 {
        (self.alt + Z_TOLERANCE.1).max(SEA_LEVEL + 1.0)
    }
}
