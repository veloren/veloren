mod sim;

use std::time::Duration;
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

pub const WORLD_SIZE: Vec2<usize> = Vec2 { x: 1024, y: 1024 };

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

        let offset_nz = Perlin::new().set_seed(3);
        let perlin_nz = Perlin::new().set_seed(1);
        let temp_nz = Perlin::new().set_seed(2);
        let chaos_nz = Perlin::new().set_seed(3);

        let get_offset = |pos: Vec2<i32>| {
            ((offset_nz.get(pos.map(|e| e as f64 / 4000.0).into_array()) + 1.0)
            * 1000.0)
        };

        let offset_z = self.sim.get(chunk_pos.map(|e| e as u32))
            .map(|chunk| chunk.alt as i32)
            .unwrap_or(0);

        //get_offset(chunk_pos * Vec2::from(TerrainChunkSize::SIZE).map(|e: u32| e as i32));

        let mut chunk = TerrainChunk::new(offset_z as i32, stone, air, TerrainChunkMeta::void());

        for x in 0..TerrainChunkSize::SIZE.x as i32 {
            for y in 0..TerrainChunkSize::SIZE.y as i32 {
                for z in offset_z as i32..offset_z as i32 + 256 {
                    let lpos = Vec3::new(x, y, z);
                    let wpos = lpos
                        + Vec3::from(chunk_pos) * TerrainChunkSize::SIZE.map(|e| e as i32);
                    let wposf = wpos.map(|e| e as f64);

                    let chaos_freq = 1.0 / 100.0;
                    let freq = 1.0 / 128.0;
                    let ampl = 75.0;
                    let small_freq = 1.0 / 32.0;
                    let small_ampl = 6.0;
                    let offs = 128.0;

                    let chaos = chaos_nz
                        .get(Vec2::from(wposf * chaos_freq).into_array())
                        .max(0.0)
                        + 0.5;

                    let height =
                        perlin_nz.get(Vec2::from(wposf * freq).into_array()) * ampl * chaos
                            + perlin_nz.get((wposf * small_freq).into_array())
                                * small_ampl
                                * 3.0
                                * chaos.powf(2.0)
                            + offs
                            + get_offset(Vec2::from(wpos)) as f64;
                    let temp =
                        (temp_nz.get(Vec2::from(wposf * (1.0 / 64.0)).into_array()) + 1.0) * 0.5;

                    let _ = chunk.set(
                        lpos,
                        if wposf.z < height - 4.0 {
                            stone
                        } else if wposf.z < height - 2.0 {
                            dirt
                        } else if wposf.z < height {
                            Block::new(2, Rgb::new(10 + (150.0 * temp) as u8, 150, 0))
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
