mod sim;

use std::{
    ops::{Add, Neg},
    time::Duration,
};
use noise::{NoiseFn, Perlin, Seedable};
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

        let warp_nz = Perlin::new().set_seed(self.sim.seed + 0);
        let temp_nz = Perlin::new().set_seed(self.sim.seed + 1);
        let ridge_nz = Perlin::new().set_seed(self.sim.seed + 2);

        let base_z = match self.sim.get_base_z(chunk_pos.map(|e| e as u32)) {
            Some(base_z) => base_z as i32,
            None => return TerrainChunk::new(0, air, air, TerrainChunkMeta::void()),
        };

        let mut chunk = TerrainChunk::new(base_z, stone, air, TerrainChunkMeta::void());

        for x in 0..TerrainChunkSize::SIZE.x as i32 {
            for y in 0..TerrainChunkSize::SIZE.y as i32 {
                let wpos2d = Vec2::new(x, y) + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);
                let wposf2d = wpos2d.map(|e| e as f64);

                let chaos = self.sim
                    .get_interpolated(wpos2d, |chunk| chunk.chaos)
                    .unwrap_or(0.0);

                let ridge_freq = 1.0 / 128.0;
                let ridge_ampl = 96.0;

                let ridge = ridge_nz
                    .get((wposf2d * ridge_freq).into_array()).abs().neg().add(1.0) as f32 * ridge_ampl * chaos.powf(8.0);

                let height_z = self.sim
                    .get_interpolated(wpos2d, |chunk| chunk.alt)
                    .unwrap_or(0.0)
                    + ridge;

                for z in base_z..base_z + 256 {
                    let lpos = Vec3::new(x, y, z);
                    let wpos = lpos
                        + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);
                    let wposf = wpos.map(|e| e as f64);

                    let warp_freq = 1.0 / 48.0;
                    let warp_ampl = 24.0;

                    let height = height_z
                        + warp_nz.get((wposf * warp_freq).into_array()) as f32 * warp_ampl * (chaos + 0.05);

                    let temp =
                        (temp_nz.get(Vec2::from(wposf * (1.0 / 64.0)).into_array()) as f32 + 1.0) * 0.5;

                    let z = wposf.z as f32;
                    let _ = chunk.set(
                        lpos,
                        if z < height - 4.0 {
                            stone
                        } else if z < height - 2.0 {
                            dirt
                        } else if z < height {
                            Block::new(2, Rgb::new(10 + (75.0 * temp) as u8, 180, 50 - (50.0 * temp) as u8))
                        } else {
                            air
                        },
                    );
                }
            }
        }

        chunk
    }
}
