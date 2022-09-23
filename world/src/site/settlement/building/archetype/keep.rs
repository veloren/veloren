use super::{super::skeleton::*, Archetype};
use crate::{
    site::BlockMask,
    util::{RandomField, Sampler},
    IndexRef,
};
use common::{
    calendar::Calendar,
    make_case_elim,
    terrain::{Block, BlockKind, SpriteKind},
};
use rand::prelude::*;
use serde::Deserialize;
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    pub brick_base: (u8, u8, u8),
    pub floor_base: (u8, u8, u8),
    pub pole: (u8, u8, u8),
    pub flag: flag_color::PureCases<(u8, u8, u8)>,
    pub stone: stone_color::PureCases<(u8, u8, u8)>,
}

pub struct Keep {
    pub flag_color: FlagColor,
    pub stone_color: StoneColor,
}

pub struct Attr {
    pub storeys: i32,
    pub is_tower: bool,
    pub flag: bool,
    pub ridged: bool,
    pub rounded: bool,
    pub has_doors: bool,
}

make_case_elim!(
    flag_color,
    #[repr(u32)]
    pub enum FlagColor {
        Good = 0,
        Evil = 1,
    }
);

make_case_elim!(
    stone_color,
    #[repr(u32)]
    pub enum StoneColor {
        Good = 0,
        Evil = 1,
    }
);

impl Archetype for Keep {
    type Attr = Attr;

    fn generate<R: Rng>(rng: &mut R, _calendar: Option<&Calendar>) -> (Self, Skeleton<Self::Attr>) {
        let len = rng.gen_range(-8..24).max(0);
        let storeys = rng.gen_range(1..3);
        let skel = Skeleton {
            offset: -rng.gen_range(0..len + 7).clamped(0, len),
            ori: if rng.gen() { Ori::East } else { Ori::North },
            root: Branch {
                len,
                attr: Attr {
                    storeys,
                    is_tower: false,
                    flag: false,
                    ridged: false,
                    rounded: true,
                    has_doors: true,
                },
                locus: 10 + rng.gen_range(0..5),
                border: 3,
                children: (0..1)
                    .map(|_| {
                        (
                            rng.gen_range(-5..len + 5).clamped(0, len.max(1) - 1),
                            Branch {
                                len: 0,
                                attr: Attr {
                                    storeys: storeys + rng.gen_range(1..3),
                                    is_tower: true,
                                    flag: true,
                                    ridged: false,
                                    rounded: true,
                                    has_doors: false,
                                },
                                locus: 6 + rng.gen_range(0..3),
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
                flag_color: FlagColor::Good,
                stone_color: StoneColor::Good,
            },
            skel,
        )
    }

    fn draw(
        &self,
        index: IndexRef,
        pos: Vec3<i32>,
        _dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        ori: Ori,
        locus: i32,
        _len: i32,
        attr: &Self::Attr,
    ) -> BlockMask {
        let dungeon_stone = index.colors.site.dungeon.stone;
        let colors = &index.colors.site.settlement.building.archetype.keep;
        let flag_color = self.flag_color.elim_case_pure(&colors.flag);
        let stone_color = self.stone_color.elim_case_pure(&colors.stone);

        let profile = Vec2::new(bound_offset.x, z);

        let weak_layer = 1;
        let normal_layer = weak_layer + 1;
        let important_layer = normal_layer + 1;
        let internal_layer = important_layer + 1;

        let make_block =
            |r, g, b| BlockMask::new(Block::new(BlockKind::Rock, Rgb::new(r, g, b)), normal_layer);

        let brick_tex_pos = (pos + Vec3::new(pos.z, pos.z, 0)) / Vec3::new(2, 2, 1);
        let brick_tex = RandomField::new(0).get(brick_tex_pos) as u8 % 24;
        let foundation = make_block(
            colors.brick_base.0 + brick_tex,
            colors.brick_base.1 + brick_tex,
            colors.brick_base.2 + brick_tex,
        );
        let wall = make_block(
            stone_color.0 + brick_tex,
            stone_color.1 + brick_tex,
            stone_color.2 + brick_tex,
        );
        let window = BlockMask::new(
            Block::air(SpriteKind::Window1)
                .with_ori(match ori {
                    Ori::East => 2,
                    Ori::North => 0,
                })
                .unwrap(),
            normal_layer,
        );
        let floor = make_block(
            colors.floor_base.0 + (pos.y.abs() % 2) as u8 * 15,
            colors.floor_base.1 + (pos.y.abs() % 2) as u8 * 15,
            colors.floor_base.2 + (pos.y.abs() % 2) as u8 * 15,
        )
        .with_priority(important_layer);
        let pole =
            make_block(colors.pole.0, colors.pole.1, colors.pole.2).with_priority(important_layer);
        let flag =
            make_block(flag_color.0, flag_color.1, flag_color.2).with_priority(important_layer);
        const AIR: Block = Block::air(SpriteKind::Empty);
        const EMPTY: BlockMask = BlockMask::nothing();
        let internal = BlockMask::new(AIR, internal_layer);

        let make_staircase = move |pos: Vec3<i32>, radius: f32, inner_radius: f32, stretch: f32| {
            let stone = BlockMask::new(Block::new(BlockKind::Rock, dungeon_stone.into()), 5);

            if (pos.xy().magnitude_squared() as f32) < inner_radius.powi(2) {
                stone
            } else if (pos.xy().magnitude_squared() as f32) < radius.powi(2) {
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
                EMPTY
            }
        };

        let ridge_x = (center_offset.map(|e| e.abs()).reduce_min() + 2) % 8;
        let width = locus + i32::from(ridge_x < 4 && attr.ridged && !attr.rounded);
        let rampart_width = 2 + width;
        let storey_height = 9;
        let roof_height = attr.storeys * storey_height;
        let storey_y = profile.y % storey_height;
        let door_height = 6;
        let rampart_height = roof_height + if ridge_x % 2 == 0 { 3 } else { 4 };
        let min_dist = if attr.rounded {
            bound_offset.map(|e| e.pow(2) as f32).sum().sqrt() as i32
        } else {
            bound_offset.map(|e| e.abs()).reduce_max()
        };

        if profile.y <= 0 - (min_dist - width - 1).max(0) && min_dist < width + 3 {
            // Foundations
            foundation
        } else if (0..=roof_height).contains(&profile.y) && storey_y == 0 && min_dist <= width + 1 {
            if min_dist < width { floor } else { wall }
        } else if bound_offset.x.abs() < 3
            && profile.y < door_height - bound_offset.x.abs()
            && profile.y > 0
            && min_dist >= width - 2
            && min_dist <= width + 1
            && attr.has_doors
        {
            internal
        } else if (min_dist == width || (!attr.is_tower && min_dist == width + 1))
            && profile.y <= roof_height
        {
            if attr.is_tower
                && (3..7).contains(&storey_y)
                && bound_offset.x.abs() < width - 2
                && (5..7).contains(&ridge_x)
            {
                window
            } else {
                wall
            }
        } else if profile.y >= roof_height {
            if profile.y > roof_height
                && (min_dist < rampart_width - 1 || (attr.is_tower && min_dist < rampart_width))
            {
                if attr.is_tower
                    && attr.flag
                    && center_offset == Vec2::zero()
                    && profile.y < roof_height + 16
                {
                    pole
                } else if attr.is_tower
                    && attr.flag
                    && center_offset.x == 0
                    && center_offset.y > 0
                    && center_offset.y < 8
                    && profile.y > roof_height + 8
                    && profile.y < roof_height + 14
                {
                    flag
                } else {
                    EMPTY
                }
            } else if min_dist <= rampart_width {
                if profile.y < rampart_height {
                    wall
                } else {
                    EMPTY
                }
            } else {
                EMPTY
            }
        } else if profile.y < roof_height && min_dist < width {
            internal
        } else {
            EMPTY
        }
        .resolve_with(
            if attr.is_tower && profile.y > 0 && profile.y <= roof_height {
                make_staircase(
                    Vec3::new(center_offset.x, center_offset.y, pos.z),
                    7.0f32.min(width as f32 - 1.0),
                    0.5,
                    9.0,
                )
            } else {
                EMPTY
            },
        )
    }
}
