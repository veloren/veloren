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
        let archetype = A::generate(rng);
        Self {
            skel: Skeleton {
                offset: -len / 2,
                ori: Ori::East,
                root: Branch {
                    len,
                    attr: A::Attr::default(),
                    locus: 3 + rng.gen_range(0, 6),
                    children: (0..rng.gen_range(1, 3))
                        .map(|_| (rng.gen_range(0, len + 1), Branch {
                            len: rng.gen_range(5, 12) * if rng.gen() { 1 } else { -1 },
                            attr: A::Attr::default(),
                            locus: 1 + rng.gen_range(0, 3),
                            children: Vec::new(),
                        }))
                        .collect(),
                },
            },
            archetype,
            origin,
        }
    }

    pub fn bounds_2d(&self) -> Aabr<i32> {
        let b = self.skel.bounds();
        Aabr {
            min: Vec2::from(self.origin) + b.min - 12,
            max: Vec2::from(self.origin) + b.max + 12,
        }
    }

    pub fn bounds(&self) -> Aabb<i32> {
        let aabr = self.bounds_2d();
        Aabb {
            min: Vec3::from(aabr.min) + Vec3::unit_z() * (self.origin.z - 5),
            max: Vec3::from(aabr.max) + Vec3::unit_z() * (self.origin.z + 32),
        }
    }

    pub fn sample(&self, pos: Vec3<i32>) -> Option<Block> {
        let rpos = pos - self.origin;
        let (dist, offset, branch) = self.skel.closest(rpos.into());

        self.archetype.draw(dist, offset, rpos.z, branch)
    }
}
