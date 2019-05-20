use std::{
    ops::{Add, Mul, Div, Neg},
    f32,
};
use noise::{NoiseFn, SuperSimplex, OpenSimplex, Seedable};
use vek::*;
use common::{
    terrain::TerrainChunkSize,
    vol::VolSize,
};

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

pub struct WorldSim {
    pub seed: u32,
    chunks: Vec<SimChunk>,
}

impl WorldSim {
    pub fn generate(seed: u32) -> Self {
        let mut gen_ctx = GenCtx {
            base_nz: OpenSimplex::new()
                .set_seed(seed + 0),
            damp_nz: SuperSimplex::new()
                .set_seed(seed + 1),
            chaos_nz: SuperSimplex::new()
                .set_seed(seed + 2),
            alt_nz: OpenSimplex::new()
                .set_seed(seed + 3),
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
        }
    }

    pub fn get(&self, chunk_pos: Vec2<u32>) -> Option<&SimChunk> {
        if chunk_pos.map2(WORLD_SIZE, |e, sz| e < sz as u32).reduce_and() {
            Some(&self.chunks[chunk_pos.y as usize * WORLD_SIZE.x + chunk_pos.x as usize])
        } else {
            None
        }
    }

    pub fn get_base_z(&self, chunk_pos: Vec2<u32>) -> Option<f32> {
        self
            .get(chunk_pos)
            .and_then(|_| (0..2)
                .map(|i| (0..2)
                    .map(move |j| (i, j)))
                .flatten()
                .map(|(i, j)| self
                    .get(chunk_pos + Vec2::new(i, j))
                    .map(|c| c.get_base_z()))
                .flatten()
                .fold(None, |a: Option<f32>, x| a.map(|a| a.min(x)).or(Some(x))))
    }

    pub fn get_interpolated<T, F>(&self, pos: Vec2<i32>, mut f: F) -> Option<T>
        where
            T: Copy + Default + Add<Output=T> + Mul<f32, Output=T>,
            F: FnMut(&SimChunk) -> T,
    {
        let pos = pos.map2(TerrainChunkSize::SIZE.into(), |e, sz: u32| e as f64 / sz as f64);

        let cubic = |a: T, b: T, c: T, d: T, x: f32| -> T {
            let x2 = x * x;

            // Catmull-Rom splines
            let co0 = a * -0.5 + b * 1.5 + c * -1.5 + d * 0.5;
            let co1 = a + b * -2.5 + c * 2.0 + d * -0.5;
            let co2 = a * -0.5 + c * 0.5;
            let co3 = b;

            co0 * x2 * x + co1 * x2 + co2 * x + co3
        };

        let mut y = [T::default(); 4];

        for (y_idx, j) in (-1..3).enumerate() {
            let x0 = f(self.get(pos.map2(Vec2::new(-1, j), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let x1 = f(self.get(pos.map2(Vec2::new( 0, j), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let x2 = f(self.get(pos.map2(Vec2::new( 1, j), |e, q| (e.max(0.0) as i32 + q) as u32))?);
            let x3 = f(self.get(pos.map2(Vec2::new( 2, j), |e, q| (e.max(0.0) as i32 + q) as u32))?);

            y[y_idx] = cubic(x0, x1, x2, x3, pos.x.fract() as f32);
        }

        /*
        fn cosine_interp (a: f32, b: f32, x: f32) -> f32 {
            let x2 = x;//(1.0 - (x * f32::consts::PI).cos()) / 2.0;
            a * (1.0 - x2) + b * x2
        }
        */

        Some(cubic(y[0], y[1], y[2], y[3], pos.y.fract() as f32))
    }
}

struct GenCtx {
    base_nz: OpenSimplex,
    damp_nz: SuperSimplex,
    chaos_nz: SuperSimplex,
    alt_nz: OpenSimplex,
}

const Z_TOLERANCE: f32 = 32.0;

pub struct SimChunk {
    pub damp: f32,
    pub chaos: f32,
    pub alt: f32,
}

impl SimChunk {
    fn generate(pos: Vec2<u32>, gen_ctx: &mut GenCtx) -> Self {
        let wposf = (pos * Vec2::from(TerrainChunkSize::SIZE)).map(|e| e as f64);

        let base = (gen_ctx.base_nz.get((wposf.div(6000.0)).into_array()) as f32)
            .add(1.0).mul(0.5)
            .mul(100.0)
            .add(32.0);

        let damp = (0.0
            + gen_ctx.damp_nz.get((wposf.div(2000.0)).into_array())
            + gen_ctx.damp_nz.get((wposf.div(250.0)).into_array()) * 0.25
            ) as f32;

        let chaos = (gen_ctx.chaos_nz
            .get((wposf.div(1000.0)).into_array()) as f32)
            .add(1.0).mul(0.5)
            .mul(damp.max(0.0))
            .add(0.15)
            .powf(2.0)
            .min(1.0);

        Self {
            damp,
            chaos,
            alt: base + ((0.0
                + gen_ctx.alt_nz.get((wposf.div(650.0)).into_array()) * 0.7
                + gen_ctx.alt_nz.get((wposf.div(100.0)).into_array()) * 0.3
                ) as f32)
                .add(1.0).mul(0.5)
                .mul(750.0)
                .mul(chaos.max(0.05)),
        }
    }

    pub fn get_base_z(&self) -> f32 {
        self.alt - Z_TOLERANCE
    }

    pub fn get_max_z(&self) -> f32 {
        self.alt + Z_TOLERANCE
    }
}
