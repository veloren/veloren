pub mod biome;
pub mod block;

// Reexports
pub use self::{biome::BiomeKind, block::Block};

use crate::{
    vol::VolSize,
    volumes::{chunk::Chunk, vol_map::VolMap},
};
use std::sync::{Arc, RwLock};
use serde_derive::{Deserialize, Serialize};
use vek::*;

// TerrainChunkSize

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkSize;

impl VolSize for TerrainChunkSize {
    const SIZE: Vec3<u32> = Vec3 {
        x: 32,
        y: 32,
        z: 32,
    };
}

// TerrainChunkMeta

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub type TerrainMapData = VolMap<Block, TerrainChunkSize, TerrainChunkMeta>;
pub type TerrainMap = Arc<RwLock<TerrainMapData>>;
