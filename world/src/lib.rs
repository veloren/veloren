#![feature(euclidean_division, bind_by_move_pattern_guards, option_flattening)]

mod all;
mod block;
mod column;
pub mod config;
pub mod sim;
pub mod util;

// Reexports
pub use crate::config::CONFIG;

use crate::{
    block::BlockGen,
    column::{ColumnGen, ColumnSample},
    util::{Sampler, SamplerMut},
};
use common::{
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    vol::{VolSize, Vox, WriteVol},
};
use std::time::Duration;
use vek::*;

#[derive(Debug)]
pub enum Error {
    Other(String),
}

pub struct World {
    sim: sim::WorldSim,
}

impl World {
    pub fn generate(seed: u32) -> Self {
        Self {
            sim: sim::WorldSim::generate(seed),
        }
    }

    pub fn sim(&self) -> &sim::WorldSim {
        &self.sim
    }

    pub fn tick(&self, _dt: Duration) {
        // TODO
    }

    pub fn sample_columns(
        &self,
    ) -> impl Sampler<Index = Vec2<i32>, Sample = Option<ColumnSample>> + '_ {
        ColumnGen::new(self)
    }

    pub fn sample_blocks(&self) -> BlockGen {
        BlockGen::new(self, ColumnGen::new(self))
    }

    pub fn generate_chunk(&self, chunk_pos: Vec2<i32>) -> TerrainChunk {
        let air = Block::empty();
        let stone = Block::new(2, Rgb::new(200, 220, 255));
        let water = Block::new(5, Rgb::new(100, 150, 255));

        let chunk_size2d = Vec2::from(TerrainChunkSize::SIZE);
        let (base_z, sim_chunk) = match self
            .sim
            .get_interpolated(
                chunk_pos.map2(chunk_size2d, |e, sz: u32| e * sz as i32 + sz as i32 / 2),
                |chunk| chunk.get_base_z(),
            )
            .and_then(|base_z| self.sim.get(chunk_pos).map(|sim_chunk| (base_z, sim_chunk)))
        {
            Some((base_z, sim_chunk)) => (base_z as i32, sim_chunk),
            None => return TerrainChunk::new(0, water, air, TerrainChunkMeta::void()),
        };

        let meta = TerrainChunkMeta::new(sim_chunk.get_name(&self.sim), sim_chunk.get_biome());

        let mut chunk = TerrainChunk::new(base_z, stone, air, meta);

        let mut sampler = self.sample_blocks();

        for x in 0..TerrainChunkSize::SIZE.x as i32 {
            for y in 0..TerrainChunkSize::SIZE.y as i32 {
                let wpos2d = Vec2::new(x, y)
                    + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);

                let z_cache = match sampler.get_z_cache(wpos2d) {
                    Some(z_cache) => z_cache,
                    None => continue,
                };

                let (min_z, max_z) = z_cache.get_z_limits();

                for z in base_z..min_z as i32 {
                    let _ = chunk.set(Vec3::new(x, y, z), stone);
                }

                for z in min_z as i32..max_z as i32 {
                    let lpos = Vec3::new(x, y, z);
                    let wpos =
                        lpos + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);

                    if let Some(block) = sampler.get_with_z_cache(wpos, Some(&z_cache)) {
                        let _ = chunk.set(lpos, block);
                    }
                }
            }
        }

        chunk
    }
}
