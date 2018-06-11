#![feature(nll)]

#[macro_use]
extern crate enum_map;
extern crate rand;
extern crate noise;
extern crate nalgebra;
extern crate coord;

mod block;
mod chunk;
mod entity;

// Reexports
pub use block::{Block, BlockMaterial};
pub use chunk::Chunk;
pub use entity::Entity;

use coord::vec3::Vec3;

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
    fn empty_with_size_offset(size: Vec3<i64>, offset: Vec3<i64>) -> Self;

    fn size(&self) -> Vec3<i64>;
    fn offset(&self) -> Vec3<i64>;
    fn at(&self, pos: Vec3<i64>) -> Option<Self::VoxelType>;
}
