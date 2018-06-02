#![feature(nll)]

#[macro_use]
extern crate enum_map;

mod block;
mod chunk;

// Reexports
pub use block::{Block, BlockMaterial};
pub use chunk::{Chunk, TerrainChunk};

pub trait Voxel: Copy + Clone {
    type Material: Copy + Clone;
    fn empty() -> Self;
    fn is_solid(&self) -> bool;
    fn material(&self) -> Self::Material;
}

pub trait Volume {
    type VoxelType: Copy + Clone;
    fn empty() -> Self;
}
