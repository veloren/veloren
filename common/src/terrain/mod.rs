pub mod block;
pub mod biome;

// Reexports
pub use self::{
    block::Block,
    biome::BiomeKind,
};

// Library
use vek::*;

// Crate
use crate::{
    vol::VolSize,
    volumes::{
        vol_map::VolMap,
        chunk::Chunk,
    },
};

// TerrainChunkSize

pub struct TerrainChunkSize;

impl VolSize for TerrainChunkSize {
    const SIZE: Vec3<u32> = Vec3 { x: 32, y: 32, z: 32 };
}

// TerrainChunkMeta

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
