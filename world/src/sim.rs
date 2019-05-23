use common::{terrain::TerrainChunkSize, vol::VolSize};
use noise::{
    BasicMulti, HybridMulti, MultiFractal, NoiseFn, OpenSimplex, RidgedMulti, Seedable,
    SuperSimplex,
};
use std::{
    f32,
    ops::{Add, Div, Mul, Neg, Sub},
};
use vek::*;

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

pub struct WorldSim {
    pub seed: u32,
    chunks: Vec<SimChunk>,
    gen_ctx: GenCtx,
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
            let y0 = f(self.get(pos.map2(Vec2::new(j, -1), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let y1 = f(self.get(pos.map2(Vec2::new(j,  0), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let y2 = f(self.get(pos.map2(Vec2::new(j,  1), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let y3 = f(self.get(pos.map2(Vec2::new(j,  2), |e, q| (e.max(0.0) as i32 + q) as u32))?);

            x[x_idx] = cubic(y0, y1, y2, y3, pos.y.fract() as f32);
        }

        Some(cubic(x[0], x[1], x[2], x[3], pos.x.fract() as f32))
    }

    pub fn sample(&self, pos: Vec2<i32>) -> Option<Sample> {
        let wposf = pos.map(|e| e as f64);

        let alt_base = self.get_interpolated(pos, |chunk| chunk.alt_base)?;
        let chaos = self.get_interpolated(pos, |chunk| chunk.chaos)?;
        let temp = self.get_interpolated(pos, |chunk| chunk.temp)?;
        let rockiness = self.get_interpolated(pos, |chunk| chunk.rockiness)?;

        let rock = (self.gen_ctx.small_nz.get((wposf.div(100.0)).into_array()) as f32)
            .mul(rockiness)
            .sub(0.2)
            .max(0.0)
            .mul(2.0);

        let alt = self.get_interpolated(pos, |chunk| chunk.alt)?
            + self.gen_ctx.small_nz.get((wposf.div(128.0)).into_array()) as f32
                * chaos.max(0.15)
                * 32.0
            + rock * 15.0;

        let wposf3d = Vec3::new(wposf.x, wposf.y, alt as f64);

        let marble = (self.gen_ctx.hill_nz.get((wposf3d.div(64.0)).into_array()) as f32)
            .mul(0.5)
            .add(1.0).mul(0.5);

        // Colours
        let cold_grass = Rgb::new(0.0, 0.75, 0.25);
        let warm_grass = Rgb::new(0.55, 0.9, 0.0);
        let cold_stone = Rgb::new(0.65, 0.7, 0.85);
        let warm_stone = Rgb::new(0.8, 0.6, 0.28);
        let sand = Rgb::new(0.93, 0.84, 0.33);
        let snow = Rgb::broadcast(1.0);

        let grass = Rgb::lerp(cold_grass, warm_grass, temp);
        let ground = Rgb::lerp(grass, warm_stone, rock.mul(5.0).min(0.8));
        let cliff = Rgb::lerp(cold_stone, warm_stone, marble);

        Some(Sample {
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
                        (alt - SEA_LEVEL - 180.0 - alt_base - temp * 48.0) / 8.0,
                    ),
                    (alt - SEA_LEVEL - 100.0) / 100.0
                ),
                // Beach
                (alt - SEA_LEVEL - 2.0) / 5.0,
            ),
        })
    }
}

pub struct Sample {
    pub alt: f32,
    pub chaos: f32,
    pub surface_color: Rgb<f32>,
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
}

const Z_TOLERANCE: (f32, f32) = (32.0, 64.0);
pub const SEA_LEVEL: f32 = 64.0;

pub struct SimChunk {
    pub chaos: f32,
    pub alt_base: f32,
    pub alt: f32,
    pub temp: f32,
    pub rockiness: f32,
}

impl SimChunk {
    fn generate(pos: Vec2<u32>, gen_ctx: &mut GenCtx) -> Self {
        let wposf = (pos * Vec2::from(TerrainChunkSize::SIZE)).map(|e| e as f64);

        let hill = (gen_ctx.hill_nz.get((wposf.div(3500.0)).into_array()) as f32).max(0.0);

        let chaos = (gen_ctx.chaos_nz.get((wposf.div(3500.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5)
            .powf(1.9)
            .add(0.25 * hill);

        let chaos = chaos + chaos.mul(20.0).sin().mul(0.05);

        let alt_base = gen_ctx.alt_nz.get((wposf.div(5000.0)).into_array()) as f32;
        let alt_base = alt_base
            .mul(0.4)
            .add(alt_base.mul(16.0).sin().mul(0.01))
            .mul(750.0);

        let alt_main = gen_ctx.alt_nz.get((wposf.div(750.0)).into_array()) as f32;

        Self {
            chaos,
            alt_base,
            alt: SEA_LEVEL + alt_base
                + (0.0
                    + alt_main
                    + gen_ctx.small_nz.get((wposf.div(300.0)).into_array()) as f32
                        * alt_main.max(0.05)
                        * chaos
                        * 1.3)
                    .add(1.0)
                    .mul(0.5)
                    .mul(chaos)
                    .mul(750.0),
            temp: (gen_ctx.temp_nz.get((wposf.div(48.0)).into_array()) as f32)
                .add(1.0)
                .mul(0.5),
            rockiness: (gen_ctx.rock_nz.get((wposf.div(1024.0)).into_array()) as f32)
                .sub(0.1)
                .mul(1.2)
                .max(0.0),
        }
    }

    pub fn get_base_z(&self) -> f32 {
        self.alt - Z_TOLERANCE.0 * (self.chaos + 0.1)
    }

    pub fn get_max_z(&self) -> f32 {
        self.alt + Z_TOLERANCE.1
    }
}

trait Hsv {
    fn into_hsv(self) -> Self;
    fn into_rgb(self) -> Self;
}

impl Hsv for Rgb<f32> {
    fn into_hsv(mut self) -> Self {
        unimplemented!()
    }

    fn into_rgb(mut self) -> Self {
        unimplemented!()
    }
}
