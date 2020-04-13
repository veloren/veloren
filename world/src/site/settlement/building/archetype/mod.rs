pub mod house;
pub mod keep;

use vek::*;
use rand::prelude::*;
use common::{terrain::Block, vol::Vox};
use super::skeleton::*;

#[derive(Copy, Clone)]
pub struct BlockMask {
    block: Block,
    priority: i32,
}

impl BlockMask {
    pub fn new(block: Block, priority: i32) -> Self {
        Self { block, priority }
    }

    pub fn nothing() -> Self {
        Self {
            block: Block::empty(),
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn resolve_with(self, dist_self: i32, other: Self, dist_other: i32) -> Self {
        if self.priority == other.priority {
            if dist_self <= dist_other {
                self
            } else {
                other
            }
        } else if self.priority >= other.priority {
            self
        } else {
            other
        }
    }

    pub fn finish(self) -> Option<Block> {
        if self.priority > 0 {
            Some(self.block)
        } else {
            None
        }
    }
}

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
    ) -> BlockMask;
}
