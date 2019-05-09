// Library
use noise::{NoiseFn, Perlin, Seedable};
use vek::*;

// Project
use common::{
    terrain::{Block, TerrainChunk, TerrainChunkMeta},
    vol::{SizedVol, Vox, WriteVol},
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
        let grass = Block::new(2, Rgb::new(75, 150, 0));
        //let grass = Block::new(2, Rgb::new(50, 255, 0));
        let dirt = Block::new(3, Rgb::new(128, 90, 0));
        let sand = Block::new(4, Rgb::new(180, 150, 50));

        let perlin_nz = Perlin::new().set_seed(1);
        let temp_nz = Perlin::new().set_seed(2);
        let chaos_nz = Perlin::new().set_seed(3);

        for lpos in chunk.iter_positions() {
            let wpos = lpos + chunk_pos * chunk.get_size().map(|e| e as i32);
            let wposf = wpos.map(|e| e as f64);

            let chaos_freq = 1.0 / 100.0;
            let freq = 1.0 / 128.0;
            let ampl = 32.0;
            let small_freq = 1.0 / 32.0;
            let small_ampl = 6.0;
            let offs = 32.0;

            let chaos = chaos_nz
                .get(Vec2::from(wposf * chaos_freq).into_array())
                .max(0.0)
                + 0.5;

            let height = perlin_nz.get(Vec2::from(wposf * freq).into_array()) * ampl * chaos
                + perlin_nz.get((wposf * small_freq).into_array())
                    * small_ampl
                    * 2.0
                    * chaos.powf(2.0)
                + offs;
            let temp = (temp_nz.get(Vec2::from(wposf * (1.0 / 64.0)).into_array()) + 1.0) * 0.5;

            chunk
                .set(
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
                )
                .unwrap();
        }

        chunk
    }
}
