use vek::*;
use rand::prelude::*;
use common::{
    terrain::{Block, BlockKind},
    vol::Vox,
};
use super::{
    Archetype,
    super::skeleton::*,
};
use crate::site::BlockMask;

pub struct Keep;

impl Archetype for Keep {
    type Attr = ();

    fn generate<R: Rng>(rng: &mut R) -> (Self, Skeleton<Self::Attr>) {
        let len = rng.gen_range(-8, 12).max(0);
        let skel = Skeleton {
            offset: -rng.gen_range(0, len + 7).clamped(0, len),
            ori: if rng.gen() { Ori::East } else { Ori::North },
            root: Branch {
                len,
                attr: Self::Attr::default(),
                locus: 5 + rng.gen_range(0, 5),
                border: 3,
                children: (0..rng.gen_range(0, 4))
                    .map(|_| (rng.gen_range(-5, len + 5).clamped(0, len.max(1) - 1), Branch {
                        len: rng.gen_range(5, 12) * if rng.gen() { 1 } else { -1 },
                        attr: Self::Attr::default(),
                        locus: 5 + rng.gen_range(0, 3),
                        border: 3,
                        children: Vec::new(),
                    }))
                    .collect(),
            },
        };

        (Self, skel)
    }

    fn draw(
        &self,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        branch: &Branch<Self::Attr>,
    ) -> BlockMask {
        let profile = Vec2::new(bound_offset.x, z);

        let make_block = |r, g, b| {
            BlockMask::new(Block::new(BlockKind::Normal, Rgb::new(r, g, b)), 2)
        };

        let foundation = make_block(100, 100, 100);
        let log = make_block(60, 45, 30);
        let wall = make_block(75, 100, 125);
        let roof = make_block(150, 120, 50);
        let empty = BlockMask::new(Block::empty(), 2);

        let width = branch.locus;
        let rampart_width = 5 + branch.locus;
        let roof_height = 12 + width;
        let ceil_height = 16;

        if profile.y <= 1 - (dist - width - 1).max(0) && dist < width + 3 { // Foundations
            foundation
        } else if profile.y == ceil_height && dist < rampart_width {
            roof
        } else if dist == rampart_width && profile.y >= ceil_height && profile.y < ceil_height + 4 {
            wall
        } else if dist == width && profile.y <= ceil_height {
            wall
        } else {
            empty
        }
    }
}
