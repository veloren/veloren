pub mod biome;
pub mod block;
pub mod chonk;
pub mod structure;

// Reexports
pub use self::{
    biome::BiomeKind,
    block::{Block, BlockKind},
    structure::Structure,
};

use crate::{vol::VolSize, volumes::vol_map_2d::VolMap2d};
use serde_derive::{Deserialize, Serialize};
use vek::*;

// TerrainChunkSize

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkSize;

impl VolSize for TerrainChunkSize {
    const SIZE: Vec3<u32> = Vec3 {
        x: 32,
        y: 32,
        z: std::i32::MAX as u32,
    };
}

// TerrainChunkMeta

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkMeta {
    name: Option<String>,
    biome: BiomeKind,
}

impl TerrainChunkMeta {
    pub fn new(name: Option<String>, biome: BiomeKind) -> Self {
        Self { name, biome }
    }

    pub fn void() -> Self {
        Self {
            name: None,
            biome: BiomeKind::Void,
        }
    }

    pub fn name(&self) -> &str {
        self.name
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("Wilderness")
    }

    pub fn biome(&self) -> BiomeKind {
        self.biome
    }
}

// Terrain type aliases

pub type TerrainChunk = chonk::Chonk; //Chunk<Block, TerrainChunkSize, TerrainChunkMeta>;
pub type TerrainMap = VolMap2d<TerrainChunk, TerrainChunkSize>;
