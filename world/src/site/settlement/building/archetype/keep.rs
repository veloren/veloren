use super::{super::skeleton::*, Archetype};
use crate::site::BlockMask;
use common::{
    terrain::{Block, BlockKind},
    vol::Vox,
};
use rand::prelude::*;
use vek::*;

pub struct Keep;

impl Archetype for Keep {
    type Attr = ();

    fn generate<R: Rng>(rng: &mut R) -> (Self, Skeleton<Self::Attr>) {
        let len = rng.gen_range(-8, 20).max(0);
        let skel = Skeleton {
            offset: -rng.gen_range(0, len + 7).clamped(0, len),
            ori: if rng.gen() { Ori::East } else { Ori::North },
            root: Branch {
                len,
                attr: Self::Attr::default(),
                locus: 6 + rng.gen_range(0, 5),
                border: 3,
                children: (0..1)
                    .map(|_| {
                        (
                            rng.gen_range(-5, len + 5).clamped(0, len.max(1) - 1),
                            Branch {
                                len: rng.gen_range(5, 12) * if rng.gen() { 1 } else { -1 },
                                attr: Self::Attr::default(),
                                locus: 5 + rng.gen_range(0, 3),
                                border: 3,
                                children: Vec::new(),
                            },
                        )
                    })
                    .collect(),
            },
        };

        (Self, skel)
    }

    #[allow(clippy::if_same_then_else)] // TODO: Pending review in #587
    fn draw(
        &self,
        pos: Vec3<i32>,
        dist: i32,
        bound_offset: Vec2<i32>,
        _center_offset: Vec2<i32>,
        z: i32,
        ori: Ori,
        branch: &Branch<Self::Attr>,
    ) -> BlockMask {
        let profile = Vec2::new(bound_offset.x, z);

        let weak_layer = 1;
        let normal_layer = weak_layer + 1;
        let important_layer = normal_layer + 1;
        let internal_layer = important_layer + 1;

        let make_block =
            |r, g, b| BlockMask::new(Block::new(BlockKind::Normal, Rgb::new(r, g, b)), normal_layer);

        let foundation = make_block(100, 100, 100);
        let wall = make_block(100, 100, 110);
        let floor = make_block(120, 80, 50).with_priority(important_layer);
        let internal = BlockMask::new(Block::empty(), internal_layer);
        let empty = BlockMask::nothing();

        let width = branch.locus;
        let rampart_width = 2 + branch.locus;
        let ceil_height = 12;
        let door_height = 6;
        let edge_pos = if (bound_offset.x == rampart_width) ^ (ori == Ori::East) {
            pos.y
        } else {
            pos.x
        };
        let rampart_height = ceil_height + if edge_pos % 2 == 0 { 3 } else { 4 };
        let min_dist = bound_offset.reduce_max();

        if profile.y <= 0 - (min_dist - width - 1).max(0) && min_dist < width + 3 {
            // Foundations
            foundation
        } else if profile.y == ceil_height && min_dist < rampart_width {
            if min_dist < width {
                floor
            } else {
                wall
            }
        } else if bound_offset.x.abs() == 4 && min_dist == width + 1 && profile.y < ceil_height {
            wall
        } else if bound_offset.x.abs() < 3 && profile.y < door_height - bound_offset.x.abs() && profile.y > 0 {
            internal
        } else if min_dist == width && profile.y <= ceil_height {
            wall
        } else if profile.y >= ceil_height {
            if profile.y > ceil_height && min_dist < rampart_width {
                internal
            } else if min_dist == rampart_width {
                if profile.y < rampart_height {
                    wall
                } else {
                    internal
                }
            } else {
                empty
            }
        } else if profile.y < ceil_height && min_dist < width {
            internal
        } else {
            empty
        }
    }
}
