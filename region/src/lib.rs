#![feature(nll)]

#[macro_use]
extern crate enum_map;
extern crate rand;
extern crate noise;

mod block;
mod chunk;

// Reexports
pub use block::{Block, BlockMaterial};
pub use chunk::Chunk;

pub trait Voxel: Copy + Clone {
    type Material: Copy + Clone;
    fn empty() -> Self;
    fn new(mat: Self::Material) -> Self;
    fn is_solid(&self) -> bool;
    fn material(&self) -> Self::Material;
}

pub trait Volume {
    type VoxelType: Voxel + Copy + Clone;

    fn empty() -> Self;
    fn empty_with_size(size: (i32, i32, i32)) -> Self;

    fn size(&self) -> (i32, i32, i32);
    fn at(&self, pos: (i32, i32, i32)) -> Option<Self::VoxelType>;
}
