use common::{
    generation::{ChunkSupplement, NpcInfo},
    terrain::{Block, BlockKind, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    vol::{ReadVol, RectVolSize, Vox, WriteVol},
};
use rand::{prelude::*, rngs::SmallRng};
use std::time::Duration;
use vek::*;

pub const WORLD_SIZE: Vec2<usize> = Vec2 {
    x: 1024 * 1,
    y: 1024 * 1,
};

pub struct World;

impl World {
    pub fn generate(_seed: u32) -> Self {
        Self
    }

    pub fn tick(&self, dt: Duration) {}

    pub fn generate_chunk(
        &self,
        chunk_pos: Vec2<i32>,
        _should_continue: impl FnMut() -> bool,
    ) -> Result<(TerrainChunk, ChunkSupplement), ()> {
        let (x, y) = chunk_pos.map(|e| e.to_le_bytes()).into_tuple();
        let mut rng = SmallRng::from_seed([
            x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], x[0], x[1], x[2], x[3], y[0], y[1],
            y[2], y[3],
        ]);
        let height = rng.gen::<i32>() % 8;

        Ok((
            TerrainChunk::new(
                256 + if rng.gen::<u8>() < 64 { height } else { 0 },
                Block::new(BlockKind::Dense, Rgb::new(200, 220, 255)),
                Block::empty(),
                TerrainChunkMeta::void(),
            ),
            ChunkSupplement::default(),
        ))
    }
}
