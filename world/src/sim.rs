use std::ops::{Mul, Div};
use noise::{NoiseFn, OpenSimplex, Seedable};
use vek::*;
use common::{
    terrain::TerrainChunkSize,
    vol::VolSize,
};
use crate::WORLD_SIZE;

pub struct WorldSim {
    seed: u32,
    chunks: Vec<SimChunk>,
}

impl WorldSim {
    pub fn generate(seed: u32) -> Self {
        let mut gen_ctx = GenCtx {
            alt_nz: OpenSimplex::new()
                .set_seed(seed),
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
}

struct GenCtx {
    alt_nz: OpenSimplex,
}

pub struct SimChunk {
    pub alt: f32,
}

impl SimChunk {
    fn generate(pos: Vec2<u32>, gen_ctx: &mut GenCtx) -> Self {
        let wposf = (pos * Vec2::from(TerrainChunkSize::SIZE)).map(|e| e as f64);

        Self {
            alt: gen_ctx.alt_nz
                .get((wposf.div(2048.0)).into_array())
                .mul(512.0) as f32,
        }
    }
}
