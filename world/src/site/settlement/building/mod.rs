pub mod archetype;
pub mod skeleton;

// Reexports
pub use self::{
    archetype::{house::House, keep::Keep, Archetype},
    skeleton::*,
};

use crate::IndexRef;
use common::{calendar::Calendar, terrain::Block};
use rand::prelude::*;
use serde::Deserialize;
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    pub archetype: archetype::Colors,
}

pub struct Building<A: Archetype> {
    skel: Skeleton<A::Attr>,
    archetype: A,
    origin: Vec3<i32>,
}

impl<A: Archetype> Building<A> {
    pub fn generate(rng: &mut impl Rng, origin: Vec3<i32>, calendar: Option<&Calendar>) -> Self
    where
        A: Sized,
    {
        let (archetype, skel) = A::generate(rng, calendar);
        Self {
            skel,
            archetype,
            origin,
        }
    }

    pub fn bounds_2d(&self) -> Aabr<i32> {
        let b = self.skel.bounds();
        Aabr {
            min: Vec2::from(self.origin) + b.min,
            max: Vec2::from(self.origin) + b.max,
        }
    }

    pub fn bounds(&self) -> Aabb<i32> {
        let aabr = self.bounds_2d();
        Aabb {
            min: Vec3::from(aabr.min) + Vec3::unit_z() * (self.origin.z - 8),
            max: Vec3::from(aabr.max) + Vec3::unit_z() * (self.origin.z + 48),
        }
    }

    pub fn sample(&self, index: IndexRef, pos: Vec3<i32>) -> Option<Block> {
        let rpos = pos - self.origin;
        self.skel
            .sample_closest(
                rpos,
                |pos, dist, bound_offset, center_offset, ori, branch| {
                    self.archetype.draw(
                        index,
                        pos,
                        dist,
                        bound_offset,
                        center_offset,
                        rpos.z,
                        ori,
                        branch.locus,
                        branch.len,
                        &branch.attr,
                    )
                },
            )
            .finish()
    }
}
