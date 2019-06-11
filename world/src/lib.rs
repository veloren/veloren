#![feature(euclidean_division, bind_by_move_pattern_guards)]

mod config;
mod all;
mod util;
mod block;
mod column;
mod sim;

// Reexports
pub use crate::config::CONFIG;

use common::{
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    vol::{VolSize, Vox, WriteVol},
};
use std::time::Duration;
use vek::*;
use crate::{
    util::{Sampler, HashCache},
    column::ColumnGen,
    block::BlockGen,
};

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

    pub fn tick(&self, dt: Duration) {
        // TODO
    }

    pub fn sample(&self) -> impl Sampler<Index=Vec3<i32>, Sample=Option<Block>> + '_ {
        BlockGen::new(self, ColumnGen::new(self))
    }

    pub fn generate_chunk(&self, chunk_pos: Vec2<i32>) -> TerrainChunk {
        let air = Block::empty();
        let stone = Block::new(1, Rgb::new(200, 220, 255));
        let water = Block::new(5, Rgb::new(100, 150, 255));

        let chunk_size2d = Vec2::from(TerrainChunkSize::SIZE);
        let base_z = match self.sim.get_interpolated(
            chunk_pos.map2(chunk_size2d, |e, sz: u32| e * sz as i32 + sz as i32 / 2),
            |chunk| chunk.get_base_z(),
        ) {
            Some(base_z) => base_z as i32,
            None => return TerrainChunk::new(0, water, air, TerrainChunkMeta::void()),
        };

        let mut chunk = TerrainChunk::new(base_z - 8, stone, air, TerrainChunkMeta::void());

        let mut sampler = self.sample();

        for x in 0..TerrainChunkSize::SIZE.x as i32 {
            for y in 0..TerrainChunkSize::SIZE.y as i32 {
                let wpos2d = Vec2::new(x, y)
                    + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);
                let wposf2d = wpos2d.map(|e| e as f64);

                let min_z = self
                    .sim
                    .get_interpolated(wpos2d, |chunk| chunk.get_min_z())
                    .unwrap_or(0.0) as i32;

                let max_z = self
                    .sim
                    .get_interpolated(wpos2d, |chunk| chunk.get_max_z())
                    .unwrap_or(0.0) as i32;

                for z in min_z..max_z {
                    let lpos = Vec3::new(x, y, z);
                    let wpos =
                        lpos + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);

                    if let Some(block) = sampler.get(wpos) {
                        let _ = chunk.set(lpos, block);
                    }
                }
            }
        }

        chunk
    }
}
