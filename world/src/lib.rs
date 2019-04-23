// Library
use vek::*;
use noise::{NoiseFn, Perlin};

// Project
use common::{
    vol::{Vox, SizedVol, WriteVol},
    terrain::{
        Block,
        TerrainChunk,
        TerrainChunkMeta,
    },
};

#[derive(Debug)]
pub enum Error {
    Other(String),
}

pub struct World;

impl World {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_chunk(chunk_pos: Vec3<i32>) -> TerrainChunk {
        // TODO: This is all test code, remove/improve this later

        let mut chunk = TerrainChunk::filled(Block::empty(), TerrainChunkMeta::void());

        let air = Block::empty();
        let stone = Block::new(1, Rgb::new(200, 220, 255));
        let grass = Block::new(2, Rgb::new(50, 255, 0));
        let sand = Block::new(3, Rgb::new(180, 150, 50));

        let perlin_nz = Perlin::new();

        for lpos in chunk.iter_positions() {
            let wpos = lpos + chunk_pos * chunk.get_size().map(|e| e as i32);
            let wposf = wpos.map(|e| e as f64);

            let freq = 1.0 / 64.0;
            let ampl = 12.0;
            let offs = 16.0;
            let height = perlin_nz.get(Vec2::from(wposf * freq).into_array()) * ampl + offs;

            chunk.set(lpos, if wposf.z < height {
                if wposf.z < height - 1.0 {
                    stone
                } else {
                    grass
                }
            } else {
                air
            }).unwrap();
        }

        chunk
    }
}
