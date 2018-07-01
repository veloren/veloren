#![feature(nll, euclidean_division)]

#[macro_use] extern crate enum_map;
extern crate rand;
extern crate noise;
extern crate nalgebra;
#[macro_use] extern crate coord;

mod block;
mod chunk;
mod cell;
mod model;
mod entity;
mod vol_mgr;

// Reexports
pub use block::{Block, BlockMaterial};
pub use chunk::Chunk;
pub use cell::{Cell};
pub use model::Model;
pub use entity::Entity;

use coord::vec3::Vec3;

pub trait Voxel: Copy + Clone {
    type Material: Copy + Clone;
    fn empty() -> Self;
    fn new(mat: Self::Material) -> Self;
    fn is_solid(&self) -> bool;
    fn material(&self) -> Self::Material;
}

pub trait Volume: Send + Sync {
    type VoxelType: Voxel + Copy + Clone;

    fn new() -> Self;
    fn fill(&mut self, block: Self::VoxelType);

    fn size(&self) -> Vec3<i64>;
    fn offset(&self) -> Vec3<i64>;
    fn rotation(&self) -> Vec3<f64>;
    fn scale(&self) -> Vec3<f64>;

    fn set_size(&mut self, size: Vec3<i64>);
    fn set_offset(&mut self, offset: Vec3<i64>);

    fn at(&self, pos: Vec3<i64>) -> Option<Self::VoxelType>;
    fn set(&mut self, pos: Vec3<i64>, vt: Self::VoxelType);
}
