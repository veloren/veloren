// Library
use vek::*;

// Project
use common::{
    vol::Vox,
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

    pub fn generate_chunk(&self, pos: Vec3<i32>) -> TerrainChunk {
        TerrainChunk::filled(Block::empty(), TerrainChunkMeta::void())
    }
}
