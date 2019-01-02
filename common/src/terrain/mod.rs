pub mod block;
pub mod biome;

// Library
use vek::*;

// Crate
use crate::{
    vol::VolSize,
    volumes::vol_map::VolMap,
};

// Local
use self::{
    block::Block,
    biome::BiomeKind,
};

// ChunkSize

pub struct ChunkSize;

impl VolSize for ChunkSize {
    const SIZE: Vec3<u32> = Vec3 { x: 32, y: 32, z: 32 };
}

// ChunkMeta

pub struct ChunkMeta {
    biome: BiomeKind,
}

// TerrainMap

pub type TerrainMap = VolMap<Block, ChunkSize, ChunkMeta>;
