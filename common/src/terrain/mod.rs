pub mod block;
pub mod biome;

// Reexports
pub use self::{
    block::Block,
    biome::BiomeKind,
};

use vek::*;
use serde_derive::{Serialize, Deserialize};
use crate::{
    vol::VolSize,
    volumes::{
        vol_map::VolMap,
        chunk::Chunk,
    },
};

// TerrainChunkSize

#[derive(Clone, Serialize, Deserialize)]
pub struct TerrainChunkSize;

impl VolSize for TerrainChunkSize {
    const SIZE: Vec3<u32> = Vec3 { x: 32, y: 32, z: 32 };
}

// TerrainChunkMeta

#[derive(Clone, Serialize, Deserialize)]
pub struct TerrainChunkMeta {
    biome: BiomeKind,
}

impl TerrainChunkMeta {
    pub fn void() -> Self {
        Self {
            biome: BiomeKind::Void,
        }
    }
}

// Terrain type aliases

pub type TerrainChunk = Chunk<Block, TerrainChunkSize, TerrainChunkMeta>;
pub type TerrainMap = VolMap<Block, TerrainChunkSize, TerrainChunkMeta>;
