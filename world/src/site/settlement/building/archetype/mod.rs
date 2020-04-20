pub mod house;
pub mod keep;

use super::skeleton::*;
use crate::site::BlockMask;
use rand::prelude::*;
use vek::*;

pub trait Archetype {
    type Attr;

    fn generate<R: Rng>(rng: &mut R) -> (Self, Skeleton<Self::Attr>)
    where
        Self: Sized;
    fn draw(
        &self,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        ori: Ori,
        branch: &Branch<Self::Attr>,
    ) -> BlockMask;
}
