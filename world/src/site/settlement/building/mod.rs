mod skeleton;
mod archetype;

// Reexports
pub use self::archetype::Archetype;

use vek::*;
use rand::prelude::*;
use self::skeleton::*;
use common::terrain::Block;

pub type HouseBuilding = Building<archetype::house::House>;

pub struct Building<A: Archetype> {
    skel: Skeleton<A::Attr>,
    archetype: A,
    origin: Vec3<i32>,
}

impl<A: Archetype> Building<A> {
    pub fn generate(rng: &mut impl Rng, origin: Vec3<i32>) -> Self
        where A: Sized
    {
        let len = rng.gen_range(-8, 12).max(0);
        let (archetype, skel) = A::generate(rng);
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
            max: Vec3::from(aabr.max) + Vec3::unit_z() * (self.origin.z + 32),
        }
    }

    pub fn sample(&self, pos: Vec3<i32>) -> Option<Block> {
        let rpos = pos - self.origin;
        self.skel.sample_closest(rpos.into(), |dist, bound_offset, center_offset, branch| {
            self.archetype.draw(dist, bound_offset, center_offset, rpos.z, branch)
        }).finish()
    }
}
