pub mod house;
pub mod keep;

use vek::*;
use rand::prelude::*;
use common::terrain::Block;
use super::skeleton::*;

pub trait Archetype {
    type Attr;

    fn generate<R: Rng>(rng: &mut R) -> (Self, Skeleton<Self::Attr>) where Self: Sized;
    fn draw(
        &self,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        branch: &Branch<Self::Attr>,
    ) -> Option<Option<Block>>;
}
