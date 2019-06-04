#![feature(euclidean_division)]

mod sim;
mod structure;

use common::{
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    vol::{SizedVol, VolSize, Vox, WriteVol},
};
use fxhash::FxHashMap;
use noise::{BasicMulti, MultiFractal, NoiseFn, Perlin, Seedable};
use std::{
    hash::Hash,
    ops::{Add, Div, Mul, Neg, Sub},
    time::Duration,
};
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

    pub fn tick(&self, dt: Duration) {
        // TODO
    }

    pub fn generate_chunk(&self, chunk_pos: Vec2<i32>) -> TerrainChunk {
        // TODO: This is all test code, remove/improve this later.

        let air = Block::empty();
        let stone = Block::new(1, Rgb::new(200, 220, 255));
        let water = Block::new(5, Rgb::new(100, 150, 255));

        let warp_nz = BasicMulti::new().set_octaves(3).set_seed(self.sim.seed + 0);

        let chunk_size2d = Vec2::from(TerrainChunkSize::SIZE);
        let base_z = match self.sim.get_interpolated(
            chunk_pos.map2(chunk_size2d, |e, sz: u32| e * sz as i32 + sz as i32 / 2),
            |chunk| chunk.get_base_z(),
        ) {
            Some(base_z) => base_z as i32,
            None => return TerrainChunk::new(0, water, air, TerrainChunkMeta::void()),
        };

        let mut chunk = TerrainChunk::new(base_z - 8, stone, air, TerrainChunkMeta::void());

        let mut world_sampler = self.sim.sampler();

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

                    let sim::Sample3d { block } =
                        if let Some(sample) = world_sampler.sample_3d(wpos) {
                            sample
                        } else {
                            continue;
                        };

                    let _ = chunk.set(lpos, block);
                }
            }
        }

        chunk
    }
}

struct Cache<K: Hash + Eq + Copy, V> {
    capacity: usize,
    map: FxHashMap<K, (usize, V)>,
    counter: usize,
}

impl<K: Hash + Eq + Copy, V> Cache<K, V> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            map: FxHashMap::default(),
            counter: 0,
        }
    }

    pub fn maintain(&mut self) {
        let (capacity, counter) = (self.capacity, self.counter);
        self.map.retain(|_, (c, _)| *c + capacity > counter);
    }

    pub fn get<F: FnOnce(K) -> V>(&mut self, k: K, f: F) -> &V {
        let mut counter = &mut self.counter;
        &self
            .map
            .entry(k)
            .or_insert_with(|| {
                *counter += 1;
                (*counter, f(k))
            })
            .1
    }
}
