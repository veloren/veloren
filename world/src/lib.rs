mod sim;

use std::{
    ops::{Add, Sub, Mul, Div, Neg},
    time::Duration,
};
use noise::{NoiseFn, BasicMulti, Perlin, Seedable, MultiFractal};
use vek::*;
use common::{
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    vol::{SizedVol, VolSize, Vox, WriteVol},
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
        Self { sim: sim::WorldSim::generate(seed) }
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
        let grass = Block::new(2, Rgb::new(75, 150, 0));
        let dirt = Block::new(3, Rgb::new(128, 90, 0));
        let sand = Block::new(4, Rgb::new(180, 150, 50));
        let water = Block::new(5, Rgb::new(100, 150, 255));

        let warp_nz = Perlin::new().set_seed(self.sim.seed + 0);
        let temp_nz = Perlin::new().set_seed(self.sim.seed + 1);
        /*
        let cliff_nz = BasicMulti::new()
            .set_octaves(2)
            .set_seed(self.sim.seed + 2);
        let cliff_mask_nz = BasicMulti::new()
            .set_octaves(4)
            .set_seed(self.sim.seed + 3);
        */

        let base_z = match self.sim.get_base_z(chunk_pos.map(|e| e as u32)) {
            Some(base_z) => base_z as i32,
            None => return TerrainChunk::new(0, air, air, TerrainChunkMeta::void()),
        };

        let mut chunk = TerrainChunk::new(base_z, stone, air, TerrainChunkMeta::void());

        for x in 0..TerrainChunkSize::SIZE.x as i32 {
            for y in 0..TerrainChunkSize::SIZE.y as i32 {
                let wpos2d = Vec2::new(x, y) + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);
                let wposf2d = wpos2d.map(|e| e as f64);

                let sim::Sample {
                    alt,
                    surface_color
                } = if let Some(sample) = self.sim.sample(wpos2d) {
                    sample
                } else {
                    continue
                };

                let max_z = self.sim
                    .get_interpolated(wpos2d, |chunk| chunk.get_max_z())
                    .unwrap_or(0.0) as i32;

                for z in base_z..max_z {
                    let lpos = Vec3::new(x, y, z);
                    let wpos = lpos
                        + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);
                    let wposf = wpos.map(|e| e as f64);

                    /*
                    let cliff_mask = cliff_mask_nz.get((wposf.div(Vec3::new(512.0, 512.0, 2048.0))).into_array())
                        .sub(0.1)
                        .max(0.0)
                        .mul(1.5)
                        .round() as f32;
                    let cliff = (cliff_nz.get((wposf.div(Vec3::new(256.0, 256.0, 128.0))).into_array()) as f32)
                        .mul(cliff_mask)
                        //.mul((30.0).div((wposf.z as f32 - alt)).max(0.0))
                        .mul(150.0)
                        .min(64.0);
                    */

                    let height = alt;// + cliff;
                    let temp = 0.0;

                    let z = wposf.z as f32;
                    let _ = chunk.set(
                        lpos,
                        if z < height - 6.0 {
                            stone
                        } else if z < height - 2.0 {
                            dirt
                        } else if z < height {
                            Block::new(1, surface_color.map(|e| (e * 255.0) as u8))
                        } else {
                            air
                        },
                    );
                }
            }
        }

        chunk

        // */
    }
}
