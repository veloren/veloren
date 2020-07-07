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
use serde::{Deserialize, Serialize};

use crate::{vol::RectVolSize, volumes::vol_grid_2d::VolGrid2d};
use vek::*;

// TerrainChunkSize

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkSize;

impl RectVolSize for TerrainChunkSize {
    const RECT_SIZE: Vec2<u32> = Vec2 { x: 32, y: 32 };
}

// TerrainChunkMeta

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkMeta {
    name: Option<String>,
    biome: BiomeKind,
}

impl TerrainChunkMeta {
    pub fn new(name: Option<String>, biome: BiomeKind) -> Self { Self { name, biome } }

    pub fn void() -> Self {
        Self {
            name: None,
            biome: BiomeKind::Void,
        }
    }

    pub fn name(&self) -> &str { self.name.as_deref().unwrap_or("Wilderness") }

    pub fn biome(&self) -> BiomeKind { self.biome }
}

// Terrain type aliases

pub type TerrainChunk = chonk::Chonk<Block, TerrainChunkSize, TerrainChunkMeta>;
pub type TerrainGrid = VolGrid2d<TerrainChunk>;
