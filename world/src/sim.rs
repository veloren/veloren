use noise::{NoiseFn, OpenSimplex, Seedable};
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
        for x in 0..WORLD_SIZE.x {
            for y in 0..WORLD_SIZE.y {
                chunks.push(SimChunk::generate(&mut gen_ctx));
            }
        }

        Self {
            seed,
            chunks,
        }
    }
}

struct GenCtx {
    alt_nz: OpenSimplex,
}

struct SimChunk {
    alt: f32,
}

impl SimChunk {
    pub fn generate(gen_ctx: &mut GenCtx) -> Self {
        Self {
            alt: 0.0
        }
    }
}
