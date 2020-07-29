use super::{super::skeleton::*, Archetype};
use crate::{
    site::BlockMask,
    util::{RandomField, Sampler},
};
use common::{
    terrain::{Block, BlockKind},
    vol::Vox,
};
use rand::prelude::*;
use vek::*;

pub struct Keep {
    pub flag_color: Rgb<u8>,
}

pub struct Attr {
    pub storeys: i32,
    pub is_tower: bool,
    pub ridged: bool,
    pub rounded: bool,
}

impl Archetype for Keep {
    type Attr = Attr;

    fn generate<R: Rng>(rng: &mut R) -> (Self, Skeleton<Self::Attr>) {
        let len = rng.gen_range(-8, 24).max(0);
        let skel = Skeleton {
            offset: -rng.gen_range(0, len + 7).clamped(0, len),
            ori: if rng.gen() { Ori::East } else { Ori::North },
            root: Branch {
                len,
                attr: Attr {
                    storeys: 1,
                    is_tower: false,
                    ridged: false,
                    rounded: true,
                },
                locus: 10 + rng.gen_range(0, 5),
                border: 3,
                children: (0..1)
                    .map(|_| {
                        (
                            rng.gen_range(-5, len + 5).clamped(0, len.max(1) - 1),
                            Branch {
                                len: 0,
                                attr: Attr {
                                    storeys: 2,
                                    is_tower: true,
                                    ridged: false,
                                    rounded: true,
                                },
                                locus: 6 + rng.gen_range(0, 3),
                                border: 3,
                                children: Vec::new(),
                            },
                        )
                    })
                    .collect(),
            },
        };

        (
            Self {
                flag_color: Rgb::new(200, 80, 40),
            },
            skel,
        )
    }

    #[allow(clippy::if_same_then_else)] // TODO: Pending review in #587
    fn draw(
        &self,
        pos: Vec3<i32>,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        ori: Ori,
        locus: i32,
        len: i32,
        attr: &Self::Attr,
    ) -> BlockMask {
        let profile = Vec2::new(bound_offset.x, z);

        let weak_layer = 1;
        let normal_layer = weak_layer + 1;
        let important_layer = normal_layer + 1;
        let internal_layer = important_layer + 1;

        let make_block = |r, g, b| {
            BlockMask::new(
                Block::new(BlockKind::Normal, Rgb::new(r, g, b)),
                normal_layer,
            )
        };

        let brick_tex_pos = (pos + Vec3::new(pos.z, pos.z, 0)) / Vec3::new(2, 2, 1);
        let brick_tex = RandomField::new(0).get(brick_tex_pos) as u8 % 24;
        let foundation = make_block(80 + brick_tex, 80 + brick_tex, 80 + brick_tex);
        let wall = make_block(100 + brick_tex, 100 + brick_tex, 110 + brick_tex);
        let floor = make_block(
            80 + (pos.y.abs() % 2) as u8 * 15,
            60 + (pos.y.abs() % 2) as u8 * 15,
            10 + (pos.y.abs() % 2) as u8 * 15,
        )
        .with_priority(important_layer);
        let pole = make_block(90, 70, 50).with_priority(important_layer);
        let flag = make_block(self.flag_color.r, self.flag_color.g, self.flag_color.b)
            .with_priority(important_layer);
        let internal = BlockMask::new(Block::empty(), internal_layer);
        let empty = BlockMask::nothing();

        let make_staircase = move |pos: Vec3<i32>, radius: f32, inner_radius: f32, stretch: f32| {
            let stone = BlockMask::new(Block::new(BlockKind::Normal, Rgb::new(150, 150, 175)), 5);

            if (pos.xy().magnitude_squared() as f32) < inner_radius.powf(2.0) {
                stone
            } else if (pos.xy().magnitude_squared() as f32) < radius.powf(2.0) {
                if ((pos.x as f32).atan2(pos.y as f32) / (std::f32::consts::PI * 2.0) * stretch
                    + pos.z as f32)
                    .rem_euclid(stretch)
                    < 1.5
                {
                    stone
                } else {
                    internal
                }
            } else {
                BlockMask::nothing()
            }
        };

        let edge_pos = if (bound_offset.x.abs() > bound_offset.y.abs()) ^ (ori == Ori::East) {
            pos.y
        } else {
            pos.x
        };

        let width = locus
            + if (edge_pos + 1000) % 8 < 4 && attr.ridged && !attr.rounded {
                1
            } else {
                0
            };
        let rampart_width = 2 + width;
        let storey_height = 16;
        let roof_height = attr.storeys * storey_height;
        let door_height = 6;
        let rampart_height = roof_height + if edge_pos % 2 == 0 { 3 } else { 4 };
        let min_dist = if attr.rounded {
            bound_offset.map(|e| e.pow(2) as f32).sum().powf(0.5) as i32
        } else {
            bound_offset.map(|e| e.abs()).reduce_max()
        };

        if profile.y <= 0 - (min_dist - width - 1).max(0) && min_dist < width + 3 {
            // Foundations
            foundation
        } else if (0..=roof_height).contains(&profile.y) && profile.y % storey_height == 0 && min_dist < rampart_width {
            if min_dist < width { floor } else { wall }
        } else if !attr.is_tower
            && bound_offset.x.abs() == 4
            && min_dist == width + 1
            && profile.y < roof_height
        {
            wall
        } else if bound_offset.x.abs() < 3
            && profile.y < door_height - bound_offset.x.abs()
            && profile.y > 0
            && min_dist >= width - 2
        {
            internal
        } else if min_dist == width && profile.y <= roof_height {
            wall
        } else if profile.y >= roof_height {
            if profile.y > roof_height && min_dist < rampart_width {
                if attr.is_tower && center_offset == Vec2::zero() && profile.y < roof_height + 16 {
                    pole
                } else if attr.is_tower
                    && center_offset.x == 0
                    && center_offset.y > 0
                    && center_offset.y < 8
                    && profile.y > roof_height + 8
                    && profile.y < roof_height + 14
                {
                    flag
                } else {
                    empty
                }
            } else if min_dist == rampart_width {
                if profile.y < rampart_height {
                    wall
                } else {
                    empty
                }
            } else {
                empty
            }
        } else if profile.y < roof_height && min_dist < width {
            internal
        } else {
            empty
        }
        .resolve_with(if attr.is_tower && profile.y > 0 && profile.y <= roof_height {
            make_staircase(
                Vec3::new(center_offset.x, center_offset.y, pos.z),
                7.0f32.min(width as f32 - 1.0),
                0.5,
                9.0,
            )
        } else {
            BlockMask::nothing()
        })
    }
}
