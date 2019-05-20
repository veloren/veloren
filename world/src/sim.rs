use std::{
    ops::{Add, Mul, Div},
    f32,
};
use noise::{NoiseFn, OpenSimplex, Seedable};
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
            alt_nz: OpenSimplex::new()
                .set_seed(seed + 0),
            chaos_nz: OpenSimplex::new()
                .set_seed(seed + 1),
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

    pub fn get_interpolated<F: FnMut(&SimChunk) -> f32>(&self, pos: Vec2<i32>, mut f: F) -> Option<f32> {
        let pos = pos.map2(TerrainChunkSize::SIZE.into(), |e, sz: u32| e as f64 / sz as f64);

        fn cubic(a: f32, b: f32, c: f32, d: f32, x: f32) -> f32 {
            let x2 = x * x;

            // Catmull-Rom splines
            let co0 = -0.5 * a + 1.5 * b - 1.5 * c + 0.5 * d;
            let co1 = a - 2.5 * b + 2.0 * c - 0.5 * d;
            let co2 = -0.5 * a + 0.5 * c;
            let co3 = b;

            co0 * x2 * x + co1 * x2 + co2 * x + co3
        }

        let mut y = [0.0; 4];

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
    chaos_nz: OpenSimplex,
    alt_nz: OpenSimplex,
}

pub struct SimChunk {
    pub chaos: f32,
    pub alt: f32,
}

impl SimChunk {
    fn generate(pos: Vec2<u32>, gen_ctx: &mut GenCtx) -> Self {
        let wposf = (pos * Vec2::from(TerrainChunkSize::SIZE)).map(|e| e as f64);

        let chaos = (gen_ctx.chaos_nz
            .get((wposf.div(400.0)).into_array()) as f32)
            .max(0.0)
            .add(0.15)
            .powf(2.0);

        Self {
            chaos,
            alt: (gen_ctx.alt_nz
                .get((wposf.div(125.0)).into_array()) as f32)
                .add(1.0).mul(0.5)
                .mul(750.0)
                .mul(chaos.max(0.05)),
        }
    }

    pub fn get_base_z(&self) -> f32 {
        const BASE_Z_TOLERANCE: f32 = 32.0;

        self.alt - BASE_Z_TOLERANCE
    }
}
