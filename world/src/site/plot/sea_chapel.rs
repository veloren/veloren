use super::*;
use crate::{
    CONFIG, Land,
    site::generation::{PrimitiveTransform, spiral_staircase},
    util::{DIAGONALS, NEIGHBORS, RandomField, sampler::Sampler, within_distance},
};
use common::{
    generation::EntityInfo,
    terrain::{Block, BlockKind, SpriteKind},
};

use rand::prelude::*;
use std::{
    f32::consts::{PI, TAU},
    sync::Arc,
};

use vek::*;

pub struct SeaChapel {
    pub(crate) center: Vec2<i32>,
    pub(crate) alt: i32,
}
impl SeaChapel {
    pub fn generate(_land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        let center = bounds.center();
        Self {
            center,
            alt: CONFIG.sea_level as i32,
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            waypoints: false,
            trees: !within_distance(wpos, self.center, 100),
            ..SpawnRules::default()
        }
    }
}

impl Structure for SeaChapel {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_seachapel\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "render_seachapel"))]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let base = self.alt + 1;
        let center = self.center;
        let diameter = 54;
        let variant = center.with_z(base);
        let mut rng = rand::rng();
        // Fills
        let (top, washed) = match (RandomField::new(0).get(variant)) % 2 {
            0 => {
                //color_scheme_1 blue
                (
                    Fill::Brick(BlockKind::Rock, Rgb::new(0, 51, 209), 24),
                    Fill::Brick(BlockKind::Rock, Rgb::new(24, 115, 242), 12),
                )
            },
            _ => {
                //color_scheme_2 turquoise
                (
                    Fill::Brick(BlockKind::Rock, Rgb::new(0, 55, 71), 24),
                    Fill::Brick(BlockKind::Rock, Rgb::new(2, 106, 129), 24),
                )
            },
        };
        let water = Fill::Block(Block::new(BlockKind::Water, Rgb::zero()));
        let rope = Fill::Block(Block::air(SpriteKind::Rope));
        let ropefix1 = Fill::Brick(BlockKind::Rock, Rgb::new(80, 75, 35), 24);
        let ropefix2 = Fill::Brick(BlockKind::Rock, Rgb::new(172, 172, 172), 4);
        let white = Fill::Brick(BlockKind::Rock, Rgb::new(202, 202, 202), 24);
        let floor_blue = Fill::Sampling(Arc::new(|variant| {
            Some(
                match (RandomField::new(0).get(Vec3::new(variant.z, variant.x, variant.y))) % 9 {
                    0 => Block::new(BlockKind::Rock, Rgb::new(38, 118, 179)),
                    1 => Block::new(BlockKind::Rock, Rgb::new(34, 52, 126)),
                    2 => Block::new(BlockKind::Rock, Rgb::new(69, 179, 228)),
                    3 => Block::new(BlockKind::Rock, Rgb::new(32, 45, 113)),
                    4 => Block::new(BlockKind::Rock, Rgb::new(252, 253, 248)),
                    5 => Block::new(BlockKind::Rock, Rgb::new(40, 106, 167)),
                    6 => Block::new(BlockKind::Rock, Rgb::new(69, 182, 240)),
                    7 => Block::new(BlockKind::Rock, Rgb::new(69, 182, 240)),
                    _ => Block::new(BlockKind::Rock, Rgb::new(240, 240, 238)),
                },
            )
        }));
        let floor_white = white.clone();
        let floor_color = match (RandomField::new(0).get(center.with_z(base - 1))) % 2 {
            0 => floor_white,
            _ => floor_blue,
        };
        let gold = Fill::Brick(BlockKind::GlowingRock, Rgb::new(245, 232, 0), 10);
        let gold_chain = Fill::Block(Block::air(SpriteKind::SeaDecorChain));
        let gold_decor = Fill::Block(Block::air(SpriteKind::SeaDecorBlock));
        let window_ver = Fill::Block(Block::air(SpriteKind::SeaDecorWindowVer));
        let window_ver2 = Fill::Block(
            Block::air(SpriteKind::SeaDecorWindowVer)
                .with_ori(2)
                .unwrap(),
        );
        let window_hor = Fill::Block(Block::air(SpriteKind::SeaDecorWindowHor));
        let glass_barrier = Fill::Block(Block::air(SpriteKind::GlassBarrier));
        let glass_keyhole = Fill::Block(Block::air(SpriteKind::GlassKeyhole));
        // random exit from water basin to side building
        let mut connect_gate_types = vec![
            SpriteKind::GlassBarrier,
            SpriteKind::SeaDecorWindowHor,
            SpriteKind::SeaDecorWindowHor,
            SpriteKind::SeaDecorWindowHor,
        ];
        let sprite_fill = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 200 {
                0 => Block::air(SpriteKind::CoralChest),
                1..=5 => Block::air(SpriteKind::SeaDecorPillar),
                6..=25 => Block::air(SpriteKind::SeashellLantern),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        let pos_var = RandomField::new(0).get(center.with_z(base)) % 5;
        let radius = diameter / 2; //8 + pos_var;
        let tubes = 7.0 + pos_var as f32;
        let phi = TAU / tubes;
        let up = diameter / 16;
        // chapel main room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8)),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + diameter),
            })
            .fill(white.clone());
        let main_upper_half = painter.aabb(Aabb {
            min: (center - (diameter / 2)).with_z(base - (diameter / 8) + (diameter / 2)),
            max: (center + (diameter / 2)).with_z(base - (diameter / 8) + diameter),
        });
        // chapel 1st washed out top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8)),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + diameter),
            })
            .intersect(main_upper_half)
            .fill(washed.clone());
        // chapel 1st top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2) + 1).with_z(base - (diameter / 8) + 1),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + diameter),
            })
            .intersect(main_upper_half)
            .fill(top.clone());
        // chapel main room gold ring
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8) + (diameter / 2) + 1),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + (diameter / 2) + 2),
            })
            .fill(gold.clone());

        // chapel top room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3)),
                max: (center + (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3)),
            })
            .fill(white.clone());
        let small_upper_half = painter.aabb(Aabb {
            min: (center - (diameter / 3)).with_z(base - (diameter / 8) + diameter),
            max: (center + (diameter / 3))
                .with_z(base - (diameter / 8) + diameter + (diameter / 3)),
        });
        // chapel small washed out top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3)),
                max: (center + (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3)),
            })
            .intersect(small_upper_half)
            .fill(washed.clone());
        // chapel small top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3) + 1),
                max: (center + (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3)),
            })
            .intersect(small_upper_half)
            .fill(top.clone());
        // chapel small top  gold ring
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3)).with_z(base - (diameter / 8) + diameter + 1),
                max: (center + (diameter / 3)).with_z(base - (diameter / 8) + diameter + 2),
            })
            .fill(gold.clone());
        // clear chapel top room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3) + 1),
                max: (center + (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3) - 1),
            })
            .clear();

        // chapel gold top emblem
        let emblem_4 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 7,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 9,
            ),
            max: Vec3::new(
                center.x + 7,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 11,
            ),
        });
        emblem_4.fill(gold.clone());
        let emblem_4_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 3,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 9,
            ),
            max: Vec3::new(
                center.x + 3,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 11,
            ),
        });
        emblem_4_clear.clear();
        emblem_4
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_4_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();

        let emblem_5 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 9,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 11,
            ),
            max: Vec3::new(
                center.x + 9,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 13,
            ),
        });
        emblem_5.fill(gold.clone());
        let emblem_5_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 5,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 11,
            ),
            max: Vec3::new(
                center.x + 5,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 13,
            ),
        });
        emblem_5_clear.clear();
        emblem_5
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_5_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();
        let emblem_6 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 11,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 13,
            ),
            max: Vec3::new(
                center.x + 11,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 17,
            ),
        });
        emblem_6.fill(gold.clone());
        let emblem_6_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 7,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 13,
            ),
            max: Vec3::new(
                center.x + 7,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 17,
            ),
        });
        emblem_6_clear.clear();
        emblem_6
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_6_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();
        let emblem_7 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 11,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 13,
            ),
            max: Vec3::new(
                center.x + 11,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 17,
            ),
        });
        emblem_7.fill(gold.clone());
        let emblem_7_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 5,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 17,
            ),
            max: Vec3::new(
                center.x + 5,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 19,
            ),
        });
        emblem_7_clear.clear();
        emblem_7
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_7_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();

        let emblem_8 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 9,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 19,
            ),
            max: Vec3::new(
                center.x + 9,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 21,
            ),
        });
        emblem_8.fill(gold.clone());
        let emblem_8_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 3,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 19,
            ),
            max: Vec3::new(
                center.x + 3,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 21,
            ),
        });
        emblem_8_clear.clear();
        emblem_8
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_8_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();
        let emblem_9 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 11,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 21,
            ),
            max: Vec3::new(
                center.x + 11,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 23,
            ),
        });
        emblem_9.fill(gold.clone());
        let emblem_9_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 7,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 21,
            ),
            max: Vec3::new(
                center.x + 7,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 23,
            ),
        });
        emblem_9_clear.clear();
        emblem_9
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_9_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();
        let emblem_10 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 11,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 23,
            ),
            max: Vec3::new(
                center.x + 11,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 25,
            ),
        });
        emblem_10.fill(gold.clone());
        let emblem_10_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 5,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 23,
            ),
            max: Vec3::new(
                center.x + 5,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 25,
            ),
        });
        emblem_10_clear.clear();
        emblem_10
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_10_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();
        let emblem_11 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 9,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 25,
            ),
            max: Vec3::new(
                center.x + 9,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 27,
            ),
        });
        emblem_11.fill(gold.clone());
        let emblem_11_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 3,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 25,
            ),
            max: Vec3::new(
                center.x + 3,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 27,
            ),
        });
        emblem_11_clear.clear();
        emblem_11
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_11_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();

        let emblem_12 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 5,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 27,
            ),
            max: Vec3::new(
                center.x + 5,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 29,
            ),
        });
        emblem_12.fill(gold.clone());
        let emblem_12_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 3,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 27,
            ),
            max: Vec3::new(
                center.x + 3,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 29,
            ),
        });
        emblem_12_clear.clear();
        emblem_12
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_12_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();
        let emblem_13 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 7,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 29,
            ),
            max: Vec3::new(
                center.x + 7,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 31,
            ),
        });
        emblem_13.fill(gold.clone());
        let emblem_13_clear = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 5,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 29,
            ),
            max: Vec3::new(
                center.x + 5,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 31,
            ),
        });
        emblem_13_clear.clear();
        emblem_13
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        emblem_13_clear
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .clear();
        // chapel gold top sphere
        painter
            .sphere(Aabb {
                min: (center - 4).with_z(base - (diameter / 8) + diameter + (diameter / 3) - 3),
                max: (center + 4).with_z(base - (diameter / 8) + diameter + (diameter / 3) + 5),
            })
            .fill(gold.clone());
        // chapel gold top pole
        painter
            .aabb(Aabb {
                min: (center - 1).with_z(base - (diameter / 8) + diameter + (diameter / 3) + 5),
                max: (center + 1).with_z(base - (diameter / 8) + diameter + (diameter / 3) + 21),
            })
            .fill(gold.clone());

        let emblem_1 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 3,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 11,
            ),
            max: Vec3::new(
                center.x + 3,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 19,
            ),
        });
        emblem_1.fill(gold.clone());
        emblem_1
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        let emblem_2 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 5,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 21,
            ),
            max: Vec3::new(
                center.x + 5,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 23,
            ),
        });
        emblem_2.fill(gold.clone());
        emblem_2
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());
        let emblem_3 = painter.aabb(Aabb {
            min: Vec3::new(
                center.x - 5,
                center.y - 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 7,
            ),
            max: Vec3::new(
                center.x + 5,
                center.y + 1,
                base - (diameter / 8) + diameter + (diameter / 3) + 9,
            ),
        });
        emblem_3.fill(gold.clone());
        emblem_3
            .rotate_about(Mat3::rotation_z(PI / 2.0).as_(), center.with_z(base))
            .fill(gold.clone());

        // chapel bottom
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2)).with_z(base - (2 * (diameter / 3))),
                max: (center + (diameter / 2)).with_z(base - (2 * (diameter / 3)) + diameter),
            })
            .fill(white.clone());
        // chapel clear bottom
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2) + 1).with_z(base - (2 * (diameter / 3)) + 1),
                max: (center + (diameter / 2) - 1)
                    .with_z(base - (2 * (diameter / 3)) + diameter - 1),
            })
            .clear();
        // cellar sea crocodiles
        let cellar_sea_croc_pos = (center - (diameter / 4)).with_z(base - (diameter / 2));
        for _ in 0..(3 + ((RandomField::new(0).get((cellar_sea_croc_pos).with_z(base))) % 5)) {
            painter.spawn(EntityInfo::at(cellar_sea_croc_pos.as_()).with_asset_expect(
                "common.entity.wild.aggressive.sea_crocodile",
                &mut rng,
                None,
            ))
        }
        // clear chapel main room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2) + 1).with_z(base - (diameter / 8) + 1),
                max: (center + (diameter / 2) - 1).with_z(base - (diameter / 8) + diameter - 1),
            })
            .clear();

        // chapel small top room gold decor ring and floor
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3) + 2).with_z(base - (diameter / 8) + diameter - 7),
                max: (center + (diameter / 3) - 2).with_z(base - (diameter / 8) + diameter - 6),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3) + 3).with_z(base - (diameter / 8) + diameter - 7),
                max: (center + (diameter / 3) - 3).with_z(base - (diameter / 8) + diameter - 6),
            })
            .fill(floor_color.clone());
        // chapel small top sprites
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3) + 3).with_z(base - (diameter / 8) + diameter - 6),
                max: (center + (diameter / 3) - 3).with_z(base - (diameter / 8) + diameter - 5),
            })
            .fill(sprite_fill.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3) + 4).with_z(base - (diameter / 8) + diameter - 6),
                max: (center + (diameter / 3) - 4).with_z(base - (diameter / 8) + diameter - 5),
            })
            .clear();
        // window to main room
        let center_w = center + 4;
        painter
            .cylinder(Aabb {
                min: (center_w - 6).with_z(base - (diameter / 8) + diameter - 7),
                max: (center_w + 6).with_z(base - (diameter / 8) + diameter - 6),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_w - 5).with_z(base - (diameter / 8) + diameter - 7),
                max: (center_w + 5).with_z(base - (diameter / 8) + diameter - 6),
            })
            .fill(window_hor);
        // chapel top floor organ podium
        let center_o2 = center - 4;
        painter
            .cylinder(Aabb {
                min: (center_o2 - 4).with_z(base - (diameter / 8) + diameter - 6),
                max: (center_o2 + 4).with_z(base - (diameter / 8) + diameter - 5),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_o2 - 3).with_z(base - (diameter / 8) + diameter - 6),
                max: (center_o2 + 3).with_z(base - (diameter / 8) + diameter - 5),
            })
            .fill(floor_color.clone());
        // organ on chapel top floor organ podium
        let first_floor_organ_pos = center_o2.with_z(base - (diameter / 8) + diameter - 4);
        painter.spawn(
            EntityInfo::at(first_floor_organ_pos.as_()).with_asset_expect(
                "common.entity.dungeon.sea_chapel.organ",
                &mut rng,
                None,
            ),
        );
        // sea clerics, bishop on top floor
        let first_floor_spawn_pos = (center_o2 - 2).with_z(base - (diameter / 8) + diameter - 4);
        for _ in 0..(2 + ((RandomField::new(0).get((first_floor_spawn_pos).with_z(base))) % 2)) {
            painter.spawn(
                EntityInfo::at(first_floor_spawn_pos.as_()).with_asset_expect(
                    "common.entity.dungeon.sea_chapel.sea_cleric",
                    &mut rng,
                    None,
                ),
            )
        }
        painter.spawn(
            EntityInfo::at(first_floor_spawn_pos.as_()).with_asset_expect(
                "common.entity.dungeon.sea_chapel.sea_bishop",
                &mut rng,
                None,
            ),
        );
        // chapel main room gold decor ring and floor
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 3).with_z(base - 1),
                max: (center + (diameter / 2) - 3).with_z(base),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 5).with_z(base - 1),
                max: (center + (diameter / 2) - 5).with_z(base),
            })
            .fill(floor_color.clone());
        // chapel main room sprites
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 5).with_z(base),
                max: (center + (diameter / 2) - 5).with_z(base + 1),
            })
            .fill(sprite_fill.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 6).with_z(base),
                max: (center + (diameter / 2) - 6).with_z(base + 1),
            })
            .clear();
        // chapel main room organ podium
        let center_o1 = center + (diameter / 8);
        painter
            .cylinder(Aabb {
                min: (center_o1 - 4).with_z(base),
                max: (center_o1 + 4).with_z(base + 1),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_o1 - 3).with_z(base),
                max: (center_o1 + 3).with_z(base + 1),
            })
            .fill(floor_color.clone());
        // organ on chapel main room organ podium
        let first_floor_organ_pos = center_o1.with_z(base + 2);
        painter.spawn(
            EntityInfo::at(first_floor_organ_pos.as_()).with_asset_expect(
                "common.entity.dungeon.sea_chapel.organ",
                &mut rng,
                None,
            ),
        );
        // sea clerics on main floor
        let main_room_sea_clerics_pos = (center_o1 - 2).with_z(base + 2);
        for _ in 0..(3 + ((RandomField::new(0).get((main_room_sea_clerics_pos).with_z(base))) % 3))
        {
            painter.spawn(
                EntityInfo::at(main_room_sea_clerics_pos.as_()).with_asset_expect(
                    "common.entity.dungeon.sea_chapel.sea_cleric",
                    &mut rng,
                    None,
                ),
            )
        }
        // coral golem on main floor
        painter.spawn(
            EntityInfo::at((first_floor_organ_pos + 2).as_()).with_asset_expect(
                "common.entity.dungeon.sea_chapel.coralgolem",
                &mut rng,
                None,
            ),
        );
        painter.spawn(
            EntityInfo::at((first_floor_organ_pos + 4).as_()).with_asset_expect(
                "common.entity.dungeon.sea_chapel.sea_bishop",
                &mut rng,
                None,
            ),
        );
        // chapel main room glassbarrier to cellar
        let center_g = center - diameter / 7;
        painter
            .cylinder(Aabb {
                min: (center_g - 4).with_z(base - 1),
                max: (center_g + 4).with_z(base),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_g - 3).with_z(base - 1),
                max: (center_g + 3).with_z(base),
            })
            .fill(glass_barrier.clone());
        painter
            .cylinder(Aabb {
                min: (center_g - 1).with_z(base - 1),
                max: (center_g).with_z(base),
            })
            .fill(glass_keyhole.clone());

        // cellar gold decor ring and floor
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 1).with_z(base - (diameter / 4) - 5),
                max: (center + (diameter / 2) - 1).with_z(base - (diameter / 4) - 4),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 3).with_z(base - (diameter / 4) - 5),
                max: (center + (diameter / 2) - 3).with_z(base - (diameter / 4) - 4),
            })
            .fill(floor_color.clone());
        // chapel cellar sprites
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 3).with_z(base - (diameter / 4) - 4),
                max: (center + (diameter / 2) - 3).with_z(base - (diameter / 4) - 3),
            })
            .fill(sprite_fill.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 4).with_z(base - (diameter / 4) - 4),
                max: (center + (diameter / 2) - 4).with_z(base - (diameter / 4) - 3),
            })
            .clear();
        // stairway to cellar cardinals room
        let stairs_pos = center_g + 1;
        let stair_radius1 = 5.0;
        let stairs_clear1 = painter.cylinder(Aabb {
            min: (stairs_pos - stair_radius1 as i32).with_z(base - (diameter / 4) - 5),
            max: (stairs_pos + stair_radius1 as i32).with_z(base - 1),
        });
        stairs_clear1
            .sample(spiral_staircase(
                stairs_pos.with_z(base - (diameter / 8) + diameter - (diameter / 8)),
                stair_radius1,
                0.5,
                7.0,
            ))
            .fill(gold.clone());
        stairs_clear1
            .sample(spiral_staircase(
                stairs_pos.with_z(base - (diameter / 8) + diameter - (diameter / 8)),
                stair_radius1 - 1.0,
                0.5,
                7.0,
            ))
            .fill(white.clone());
        // cardinals room sea clerics
        let cr_sea_clerics_pos = (center - (diameter / 5)).with_z(base - (diameter / 4) - 3);
        for _ in 0..(2 + ((RandomField::new(0).get((cr_sea_clerics_pos).with_z(base))) % 3)) {
            painter.spawn(EntityInfo::at(cr_sea_clerics_pos.as_()).with_asset_expect(
                "common.entity.dungeon.sea_chapel.sea_cleric",
                &mut rng,
                None,
            ))
        }
        // Cardinal
        let cr_cardinal_pos = (center - (diameter / 6)).with_z(base - (diameter / 4) - 3);
        painter.spawn(EntityInfo::at(cr_cardinal_pos.as_()).with_asset_expect(
            "common.entity.dungeon.sea_chapel.cardinal",
            &mut rng,
            None,
        ));
        // glassbarrier to water basin
        painter
            .cylinder(Aabb {
                min: (center - 3).with_z(base - (diameter / 4) - 5),
                max: (center + 3).with_z(base - (diameter / 4) - 4),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center - 2).with_z(base - (diameter / 4) - 5),
                max: (center + 2).with_z(base - (diameter / 4) - 4),
            })
            .fill(glass_barrier.clone());
        painter
            .cylinder(Aabb {
                min: (center - 1).with_z(base - (diameter / 4) - 5),
                max: (center).with_z(base - (diameter / 4) - 4),
            })
            .fill(glass_keyhole.clone());

        // chapel floor1 window1
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 3),
                    center.y - 2,
                    base - (diameter / 8) + diameter - 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 3) + 1,
                    center.y + 2,
                    base - (diameter / 8) + diameter - 1,
                ),
            })
            .fill(window_ver2.clone());

        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 3),
                    center.y - 1,
                    base - (diameter / 8) + diameter - 3,
                ),
                max: Vec3::new(
                    center.x - (diameter / 3) + 1,
                    center.y + 1,
                    base - (diameter / 8) + diameter - 2,
                ),
            })
            .fill(window_ver2.clone());

        // chapel floor1 window2
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 3) - 1,
                    center.y - 2,
                    base - (diameter / 8) + diameter - 2,
                ),
                max: Vec3::new(
                    center.x + (diameter / 3),
                    center.y + 2,
                    base - (diameter / 8) + diameter - 1,
                ),
            })
            .clear();

        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 3) - 1,
                    center.y - 1,
                    base - (diameter / 8) + diameter - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 3),
                    center.y + 1,
                    base - (diameter / 8) + diameter - 2,
                ),
            })
            .clear();

        // chapel floor1 window3
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 2,
                    center.y + (diameter / 3) - 1,
                    base - (diameter / 8) + diameter - 2,
                ),
                max: Vec3::new(
                    center.x + 2,
                    center.y + (diameter / 3),
                    base - (diameter / 8) + diameter - 1,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 1,
                    center.y + (diameter / 3) - 1,
                    base - (diameter / 8) + diameter - 3,
                ),
                max: Vec3::new(
                    center.x + 1,
                    center.y + (diameter / 3),
                    base - (diameter / 8) + diameter - 2,
                ),
            })
            .clear();
        // chapel floor1 window4
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 2,
                    center.y - (diameter / 3),
                    base - (diameter / 8) + diameter - 2,
                ),
                max: Vec3::new(
                    center.x + 2,
                    center.y - (diameter / 3) + 1,
                    base - (diameter / 8) + diameter - 1,
                ),
            })
            .fill(window_ver.clone());

        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 1,
                    center.y - (diameter / 3),
                    base - (diameter / 8) + diameter - 3,
                ),
                max: Vec3::new(
                    center.x + 1,
                    center.y - (diameter / 3) + 1,
                    base - (diameter / 8) + diameter - 2,
                ),
            })
            .fill(window_ver.clone());

        // chapel main room window1
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2),
                    center.y - 4,
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + 1,
                    center.y + 4,
                    base - (diameter / 8) + (diameter / 2) - 1,
                ),
            })
            .fill(window_ver2.clone());

        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2),
                    center.y - 3,
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + 1,
                    center.y + 3,
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
            })
            .fill(window_ver2.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2),
                    center.y - 2,
                    base - (diameter / 8) + (diameter / 2) - 4,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + 1,
                    center.y + 2,
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
            })
            .fill(window_ver2.clone());

        // chapel main room window2
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - 1,
                    center.y - 4,
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2),
                    center.y + 4,
                    base - (diameter / 8) + (diameter / 2) - 1,
                ),
            })
            .fill(window_ver2.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - 1,
                    center.y - 3,
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2),
                    center.y + 3,
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
            })
            .fill(window_ver2.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - 1,
                    center.y - 2,
                    base - (diameter / 8) + (diameter / 2) - 4,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2),
                    center.y + 2,
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
            })
            .fill(window_ver2.clone());
        // chapel main room window3
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 4,
                    center.y + (diameter / 2) - 1,
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
                max: Vec3::new(
                    center.x + 4,
                    center.y + (diameter / 2),
                    base - (diameter / 8) + (diameter / 2) - 1,
                ),
            })
            .fill(window_ver.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 3,
                    center.y + (diameter / 2) - 1,
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
                max: Vec3::new(
                    center.x + 3,
                    center.y + (diameter / 2),
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
            })
            .fill(window_ver.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 2,
                    center.y + (diameter / 2) - 1,
                    base - (diameter / 8) + (diameter / 2) - 4,
                ),
                max: Vec3::new(
                    center.x + 2,
                    center.y + (diameter / 2),
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
            })
            .fill(window_ver.clone());

        // chapel main room window4
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 4,
                    center.y - (diameter / 2) + 1,
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
                max: Vec3::new(
                    center.x + 4,
                    center.y - (diameter / 2),
                    base - (diameter / 8) + (diameter / 2) - 1,
                ),
            })
            .fill(window_ver.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 3,
                    center.y - (diameter / 2) + 1,
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
                max: Vec3::new(
                    center.x + 3,
                    center.y - (diameter / 2),
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
            })
            .fill(window_ver.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 2,
                    center.y - (diameter / 2) + 1,
                    base - (diameter / 8) + (diameter / 2) - 4,
                ),
                max: Vec3::new(
                    center.x + 2,
                    center.y - (diameter / 2),
                    base - (diameter / 8) + (diameter / 2) - 3,
                ),
            })
            .fill(window_ver.clone());

        // main room hanging emblem
        painter.rotated_sprite(
            center.with_z(base + (diameter / 4)),
            SpriteKind::SeaDecorEmblem,
            2_u8,
        );
        painter
            .aabb(Aabb {
                min: Vec3::new(center.x, center.y, base + (diameter / 4) + 1),
                max: Vec3::new(
                    center.x + 1,
                    center.y + 1,
                    base - (diameter / 8) + diameter - 7,
                ),
            })
            .fill(gold_chain.clone());

        for n in 1..=tubes as i32 {
            let pos = Vec2::new(
                center.x + (radius as f32 * ((n as f32 * phi).cos())) as i32,
                center.y + (radius as f32 * ((n as f32 * phi).sin())) as i32,
            );

            let storeys = 2 + (RandomField::new(0).get(pos.with_z(base)) as i32 % 2);
            let steps = 7;
            for n in 0..2 {
                for t in 0..storeys {
                    let mut points = vec![];
                    for s in 0..steps {
                        let step_pos = Vec2::new(
                            pos.x + (8_f32 * ((s as f32 * phi).cos())) as i32,
                            pos.y + (8_f32 * ((s as f32 * phi).sin())) as i32,
                        );
                        points.push(step_pos);
                    }
                    if t == (storeys - 1) {
                        // room
                        painter
                            .sphere(Aabb {
                                min: (points[0] - (diameter / 6)).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6),
                                ),

                                max: (points[0] + (diameter / 6)).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) + (diameter / 6),
                                ),
                            })
                            .fill(white.clone());

                        let room_upper_half = painter.aabb(Aabb {
                            min: (points[0] - (diameter / 6))
                                .with_z(base + (6 * up) + (t * (steps - 1) * up)),
                            max: (points[0] + (diameter / 6))
                                .with_z(base + (6 * up) + (t * (steps - 1) * up) + (diameter / 6)),
                        });
                        // room top washed out
                        painter
                            .sphere(Aabb {
                                min: (points[0] - (diameter / 6)).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6),
                                ),
                                max: (points[0] + (diameter / 6)).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) + (diameter / 6),
                                ),
                            })
                            .intersect(room_upper_half)
                            .fill(washed.clone());
                        // room top
                        painter
                            .sphere(Aabb {
                                min: (points[0] - (diameter / 6) + 1).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 1,
                                ),
                                max: (points[0] + (diameter / 6)).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) + (diameter / 6),
                                ),
                            })
                            .intersect(room_upper_half)
                            .fill(top.clone());
                        // room gold ring
                        painter
                            .cylinder(Aabb {
                                min: (points[0] - (diameter / 6))
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) + 1),
                                max: (points[0] + (diameter / 6))
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) + 2),
                            })
                            .fill(gold.clone());
                        // clear room
                        painter
                            .sphere(Aabb {
                                min: (points[0] - (diameter / 6) + 1).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 1,
                                ),

                                max: (points[0] + (diameter / 6) - 1).with_z(
                                    base + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) - 1,
                                ),
                            })
                            .clear();
                        // room windows 1
                        painter
                            .aabb(Aabb {
                                min: Vec3::new(
                                    points[0].x + (diameter / 6) - 1,
                                    points[0].y - 2,
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 7,
                                ),
                                max: Vec3::new(
                                    points[0].x + (diameter / 6),
                                    points[0].y + 2,
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 8,
                                ),
                            })
                            .fill(window_ver2.clone());
                        // room windows 2
                        painter
                            .aabb(Aabb {
                                min: Vec3::new(
                                    points[0].x - 2,
                                    points[0].y + (diameter / 6) - 1,
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 7,
                                ),
                                max: Vec3::new(
                                    points[0].x + 2,
                                    points[0].y + (diameter / 6),
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 8,
                                ),
                            })
                            .fill(window_ver.clone());
                        // room windows 3
                        painter
                            .aabb(Aabb {
                                min: Vec3::new(
                                    points[0].x - 2,
                                    points[0].y - (diameter / 6) + 1,
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 7,
                                ),
                                max: Vec3::new(
                                    points[0].x + 2,
                                    points[0].y - (diameter / 6),
                                    base + (6 * up) + (t * (steps - 1) * up) - (diameter / 6) + 8,
                                ),
                            })
                            .fill(window_ver.clone());
                    }
                    let stairs_decor = painter
                        .cubic_bezier(
                            points[0].with_z(base - 4 + (n * 2) + (t * (steps - 1) * up)),
                            points[1].with_z(base - 4 + (n * 2) + up + (t * (steps - 1) * up)),
                            points[2]
                                .with_z(base - 4 + (n * 2) + (2 * up) + (t * (steps - 1) * up)),
                            points[3]
                                .with_z(base - 4 + (n * 2) + (3 * up) + (t * (steps - 1) * up)),
                            1.0,
                        )
                        .union(
                            painter.cubic_bezier(
                                points[3]
                                    .with_z(base - 4 + (n * 2) + (3 * up) + (t * (steps - 1) * up)),
                                points[4]
                                    .with_z(base - 4 + (n * 2) + (4 * up) + (t * (steps - 1) * up)),
                                points[5]
                                    .with_z(base - 4 + (n * 2) + (5 * up) + (t * (steps - 1) * up)),
                                points[0]
                                    .with_z(base - 4 + (n * 2) + (6 * up) + (t * (steps - 1) * up)),
                                1.0,
                            ),
                        );
                    let stairs = painter
                        .cubic_bezier(
                            points[0].with_z(base + (n * 2) + (t * (steps - 1) * up)),
                            points[1].with_z(base + (n * 2) + up + (t * (steps - 1) * up)),
                            points[2].with_z(base + (n * 2) + (2 * up) + (t * (steps - 1) * up)),
                            points[3].with_z(base + (n * 2) + (3 * up) + (t * (steps - 1) * up)),
                            4.0,
                        )
                        .union(painter.cubic_bezier(
                            points[3].with_z(base + (n * 2) + (3 * up) + (t * (steps - 1) * up)),
                            points[4].with_z(base + (n * 2) + (4 * up) + (t * (steps - 1) * up)),
                            points[5].with_z(base + (n * 2) + (5 * up) + (t * (steps - 1) * up)),
                            points[0].with_z(base + (n * 2) + (6 * up) + (t * (steps - 1) * up)),
                            4.0,
                        ));
                    match n {
                        0 => {
                            stairs_decor.fill(gold.clone());
                            stairs.fill(white.clone())
                        },
                        _ => stairs.clear(),
                    };
                    if t == (storeys - 1) {
                        // room gold decor ring and floor
                        painter
                            .cylinder(Aabb {
                                min: (points[0] - (diameter / 6) + 2)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 5),
                                max: (points[0] + (diameter / 6) - 2)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 4),
                            })
                            .fill(gold_decor.clone());
                        painter
                            .cylinder(Aabb {
                                min: (points[0] - (diameter / 6) + 3)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 7),
                                max: (points[0] + (diameter / 6) - 3)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 4),
                            })
                            .fill(floor_color.clone());
                        // room sprites
                        painter
                            .cylinder(Aabb {
                                min: (points[0] - (diameter / 6) + 3)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 3),
                                max: (points[0] + (diameter / 6) - 3)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 4),
                            })
                            .fill(sprite_fill.clone());
                        painter
                            .cylinder(Aabb {
                                min: (points[0] - (diameter / 6) + 4)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 3),
                                max: (points[0] + (diameter / 6) - 4)
                                    .with_z(base + (6 * up) + (t * (steps - 1) * up) - 4),
                            })
                            .clear();
                        // room sea clerics
                        let room_clerics_pos =
                            (points[0] + 2).with_z(base + (6 * up) + (t * (steps - 1) * up) - 4);
                        painter.spawn(EntityInfo::at(room_clerics_pos.as_()).with_asset_expect(
                            "common.entity.dungeon.sea_chapel.sea_cleric",
                            &mut rng,
                            None,
                        ));
                    };
                    // decor for top rooms
                    if t == 2 {
                        let bldg_gold_top1 = Aabb {
                            min: (points[0] - 2).with_z(
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) - 3,
                            ),
                            max: (points[0] + 2).with_z(
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 1,
                            ),
                        };
                        let bldg_gold_top_pole = Aabb {
                            min: (points[0] - 1).with_z(
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 1,
                            ),
                            max: (points[0] + 1).with_z(
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 7,
                            ),
                        };
                        let bldg_gold_top_antlers1 = Aabb {
                            min: Vec3::new(
                                points[0].x - 2,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 1,
                            ),
                            max: Vec3::new(
                                points[0].x + 2,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 2,
                            ),
                        };
                        let bldg_gold_top_antlers2 = painter.aabb(Aabb {
                            min: Vec3::new(
                                points[0].x - 3,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 2,
                            ),

                            max: Vec3::new(
                                points[0].x + 3,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 3,
                            ),
                        });
                        let bldg_gold_top_antlers2_clear = painter.aabb(Aabb {
                            min: Vec3::new(
                                points[0].x - 2,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 2,
                            ),

                            max: Vec3::new(
                                points[0].x + 2,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 3,
                            ),
                        });
                        let bldg_gold_top_antlers3 = Aabb {
                            min: Vec3::new(
                                points[0].x - 3,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 4,
                            ),
                            max: Vec3::new(
                                points[0].x + 3,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 5,
                            ),
                        };
                        let bldg_gold_top_antlers4 = painter.aabb(Aabb {
                            min: Vec3::new(
                                points[0].x - 5,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 5,
                            ),

                            max: Vec3::new(
                                points[0].x + 5,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 6,
                            ),
                        });
                        let bldg_gold_top_antlers4_clear = painter.aabb(Aabb {
                            min: Vec3::new(
                                points[0].x - 2,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 5,
                            ),

                            max: Vec3::new(
                                points[0].x + 2,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 6,
                            ),
                        });
                        let bldg_gold_top_antlers5 = painter.aabb(Aabb {
                            min: Vec3::new(
                                points[0].x - 2,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 7,
                            ),

                            max: Vec3::new(
                                points[0].x + 2,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 8,
                            ),
                        });
                        let bldg_gold_top_antlers5_clear = painter.aabb(Aabb {
                            min: Vec3::new(
                                points[0].x - 1,
                                points[0].y - 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 7,
                            ),
                            max: Vec3::new(
                                points[0].x + 1,
                                points[0].y + 1,
                                base + 1 + (6 * up) + (t * (steps - 1) * up) + (diameter / 6) + 8,
                            ),
                        });
                        bldg_gold_top_antlers2.fill(gold.clone());
                        bldg_gold_top_antlers2_clear.clear();
                        bldg_gold_top_antlers4.fill(gold.clone());
                        bldg_gold_top_antlers4_clear.clear();
                        bldg_gold_top_antlers5.fill(gold.clone());
                        bldg_gold_top_antlers5_clear.clear();
                        painter.sphere(bldg_gold_top1).fill(gold.clone());
                        painter.aabb(bldg_gold_top_pole).fill(gold.clone());
                        painter.aabb(bldg_gold_top_antlers1).fill(gold.clone());
                        painter.aabb(bldg_gold_top_antlers3).fill(gold.clone());
                    };
                }
            }
        }

        // water basin
        painter
            .sphere_with_radius(
                center.with_z(base - (2 * diameter) + (diameter / 4)),
                (diameter + (diameter / 5) + 1) as f32,
            )
            .fill(white.clone());
        let water_basin = painter.sphere_with_radius(
            center.with_z(base - (2 * diameter) + (diameter / 4) + 1),
            (diameter + (diameter / 5)) as f32,
        );
        water_basin.fill(water.clone());
        // clear some water
        water_basin
            .intersect(
                painter.aabb(Aabb {
                    min: (center - (diameter + (diameter / 5)))
                        .with_z(base - (4 * (diameter / 4)) + 1),
                    max: (center + (diameter + (diameter / 5)))
                        .with_z(base - diameter + (diameter / 5) + (diameter / 4) + 1),
                }),
            )
            .clear();
        // water basin gold ring
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                (diameter - 4) as f32,
                1.0,
            )
            .fill(gold.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                (diameter - 5) as f32,
                1.0,
            )
            .fill(water.clone());

        // underwater chamber
        painter
            .sphere_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                ((diameter / 2) + 2) as f32,
            )
            .fill(white.clone());

        painter
            .sphere_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                (diameter / 2) as f32,
            )
            .fill(water.clone());

        // underwater chamber entries
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) - 2,
                    center.y - 2,
                    base - (3 * diameter) + (diameter / 3) - 8,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) + 2,
                    center.y + 2,
                    base - (3 * diameter) + (diameter / 3) + 4,
                ),
            })
            .fill(water.clone());
        // underwater chamber entries
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 2,
                    center.y - (diameter / 2) - 2,
                    base - (3 * diameter) + (diameter / 3) - 8,
                ),
                max: Vec3::new(
                    center.x + 2,
                    center.y + (diameter / 2) + 2,
                    base - (3 * diameter) + (diameter / 3) + 4,
                ),
            })
            .fill(water);
        // underwater chamber top
        painter
            .sphere_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                ((diameter / 2) + 2) as f32,
            )
            .intersect(
                painter.aabb(Aabb {
                    min: (center - (diameter / 2) - 3)
                        .with_z(base - (3 * diameter) + (diameter / 2) - 1),
                    max: (center + (diameter / 2) + 3).with_z(base - (3 * diameter) + diameter + 2),
                }),
            )
            .fill(top.clone());
        // clear underwater chamber
        painter
            .sphere_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                (diameter / 2) as f32,
            )
            .intersect(
                painter.aabb(Aabb {
                    min: (center - (diameter / 2) - 3)
                        .with_z(base - (3 * diameter) + (diameter / 2) - 1),
                    max: (center + (diameter / 2) + 3).with_z(base - (3 * diameter) + diameter + 2),
                }),
            )
            .clear();
        // underwater chamber gold ring and floors
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                ((diameter / 2) + 2) as f32,
                1.0,
            )
            .fill(gold.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                ((diameter / 2) + 1) as f32,
                1.0,
            )
            .fill(floor_color.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) - 1),
                ((diameter / 2) + 1) as f32,
                1.0,
            )
            .fill(floor_color.clone());
        // organ in underwater chamber
        let underwater_organ_pos =
            (center - (diameter / 4)).with_z(base - (3 * diameter) + (diameter / 2) + 1);
        painter.spawn(
            EntityInfo::at(underwater_organ_pos.as_()).with_asset_expect(
                "common.entity.dungeon.sea_chapel.organ",
                &mut rng,
                None,
            ),
        );
        // underwater chamber decor ring
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) + 1),
                (diameter / 2) as f32,
                1.0,
            )
            .fill(gold_decor.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) + 1),
                ((diameter / 2) - 1) as f32,
                1.0,
            )
            .clear();

        // underwater chamber sprites
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) + 1),
                ((diameter / 2) - 1) as f32,
                1.0,
            )
            .fill(sprite_fill);
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) + 1),
                ((diameter / 2) - 2) as f32,
                1.0,
            )
            .clear();
        // underwater chamber hanging emblem
        painter.rotated_sprite(
            center.with_z(base - (3 * diameter) + (diameter / 2) + (diameter / 4)),
            SpriteKind::SeaDecorEmblem,
            2_u8,
        );
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x,
                    center.y,
                    base - (3 * diameter) + (diameter / 2) + (diameter / 4) + 1,
                ),
                max: Vec3::new(
                    center.x + 1,
                    center.y + 1,
                    base - (3 * diameter) + diameter + 1,
                ),
            })
            .fill(gold_chain);
        // underwater chamber dagon
        let cellar_miniboss_pos = (center + 6).with_z(base - (3 * diameter) + (diameter / 2) + 1);
        painter.spawn(EntityInfo::at(cellar_miniboss_pos.as_()).with_asset_expect(
            "common.entity.dungeon.sea_chapel.dagon",
            &mut rng,
            None,
        ));
        // underwater chamber floor entry
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                3.0,
                1.0,
            )
            .fill(white.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) + 3),
                3.0,
                1.0,
            )
            .fill(gold_decor.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2)),
                3.0,
                3.0,
            )
            .fill(white.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) - 1),
                3.0,
                1.0,
            )
            .fill(gold.clone());
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 2) - 1),
                2.0,
                7.0,
            )
            .clear();
        // water basin ground floor
        painter
            .cylinder_with_radius(
                center.with_z(base - (3 * diameter) + (diameter / 3) - 9),
                (diameter / 2) as f32,
                1.0,
            )
            .fill(floor_color.clone());

        // side buildings hut, pavillon, tower
        for dir in DIAGONALS {
            let bldg_center = center + dir * (2 * (diameter / 3));
            let bldg_variant = (RandomField::new(0).get((bldg_center).with_z(base))) % 10;
            let tower_height = (diameter / 4) + (3 * (bldg_variant as i32));
            let bldg_diameter = diameter;
            let bldg_cellar = Aabb {
                min: (bldg_center - (bldg_diameter / 4)).with_z(base - (bldg_diameter / 3)),
                max: (bldg_center + (bldg_diameter / 4))
                    .with_z(base - (bldg_diameter / 3) + (bldg_diameter / 2)),
            };
            let bldg_cellar_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 4) + 1).with_z(base - (bldg_diameter / 3) + 1),
                max: (bldg_center + (bldg_diameter / 4) - 1)
                    .with_z(base - (bldg_diameter / 3) + (bldg_diameter / 2) - 1),
            };
            let bldg_room = Aabb {
                min: (bldg_center - (bldg_diameter / 4)).with_z(base - (bldg_diameter / 15)),
                max: (bldg_center + (bldg_diameter / 4))
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
            };
            let bldg_hut_entry_clear1 = Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 4) - 2,
                    bldg_center.y - 2,
                    base + (bldg_diameter / 15),
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 4) + 2,
                    bldg_center.y + 2,
                    base + (bldg_diameter / 15) + 4,
                ),
            };
            let bldg_hut_entry_clear2 = Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 4) - 2,
                    bldg_center.y - 1,
                    base + (bldg_diameter / 15) + 4,
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 4) + 2,
                    bldg_center.y + 1,
                    base + (bldg_diameter / 15) + 5,
                ),
            };
            let bldg_pavillon_entry_clear1 = Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 4),
                    bldg_center.y - 6,
                    base + (bldg_diameter / 15),
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 4),
                    bldg_center.y + 6,
                    base + (bldg_diameter / 15) + 4,
                ),
            };
            let bldg_pavillon_entry_clear2 = Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 4),
                    bldg_center.y - 5,
                    base + (bldg_diameter / 15) + 4,
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 4),
                    bldg_center.y + 5,
                    base + (bldg_diameter / 15) + 5,
                ),
            };
            let bldg_pavillon_entry_clear3 = Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 4),
                    bldg_center.y - 4,
                    base + (bldg_diameter / 15) + 5,
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 4),
                    bldg_center.y + 4,
                    base + (bldg_diameter / 15) + 6,
                ),
            };
            let bldg_pavillon_entry_clear4 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 6,
                    bldg_center.y - (bldg_diameter / 4),
                    base + (bldg_diameter / 15),
                ),
                max: Vec3::new(
                    bldg_center.x + 6,
                    bldg_center.y + (bldg_diameter / 4),
                    base + (bldg_diameter / 15) + 4,
                ),
            };
            let bldg_pavillon_entry_clear5 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 5,
                    bldg_center.y - (bldg_diameter / 4),
                    base + (bldg_diameter / 15) + 4,
                ),
                max: Vec3::new(
                    bldg_center.x + 5,
                    bldg_center.y + (bldg_diameter / 4),
                    base + (bldg_diameter / 15) + 5,
                ),
            };
            let bldg_pavillon_entry_clear6 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 4,
                    bldg_center.y - (bldg_diameter / 4),
                    base + (bldg_diameter / 15) + 5,
                ),
                max: Vec3::new(
                    bldg_center.x + 4,
                    bldg_center.y + (bldg_diameter / 4),
                    base + (bldg_diameter / 15) + 6,
                ),
            };
            let bldg_room_windows_1 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 1,
                    bldg_center.y - (bldg_diameter / 4),
                    base - (bldg_diameter / 15) + (bldg_diameter / 4) - 2,
                ),
                max: Vec3::new(
                    bldg_center.x + 1,
                    bldg_center.y - (bldg_diameter / 4) + 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 4) - 1,
                ),
            });
            let bldg_room_windows_2 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 1,
                    bldg_center.y + (bldg_diameter / 4) - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 4) - 2,
                ),
                max: Vec3::new(
                    bldg_center.x + 1,
                    bldg_center.y + (bldg_diameter / 4),
                    base - (bldg_diameter / 15) + (bldg_diameter / 4) - 1,
                ),
            });
            let bldg_top_half = painter.aabb(Aabb {
                min: (bldg_center - (bldg_diameter / 4))
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4)),
                max: (bldg_center + (bldg_diameter / 4))
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
            });
            let bldg_washed_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 4)).with_z(base - (bldg_diameter / 15)),
                    max: (bldg_center + (bldg_diameter / 4))
                        .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
                })
                .intersect(bldg_top_half);
            let bldg_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 4) + 1)
                        .with_z(base - (bldg_diameter / 15) + 1),
                    max: (bldg_center + (bldg_diameter / 4))
                        .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
                })
                .intersect(bldg_top_half);
            let bldg_room_goldring = painter.cylinder(Aabb {
                min: (bldg_center - (bldg_diameter / 4))
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 1),
                max: (bldg_center + (bldg_diameter / 4))
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 2),
            });
            let bldg_room_goldring_clear = painter.cylinder(Aabb {
                min: (bldg_center - (bldg_diameter / 4) + 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 1),
                max: (bldg_center + (bldg_diameter / 4) - 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 2),
            });
            let bldg_room_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 4) + 1)
                    .with_z(base - (bldg_diameter / 15) + 1),
                max: (bldg_center + (bldg_diameter / 4) - 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1),
            };
            let bldg_room_floor = Aabb {
                min: (bldg_center - (bldg_diameter / 4) + 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 1),
                max: (bldg_center + (bldg_diameter / 4) - 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 2),
            };
            let bldg_hut_floors_clear = Aabb {
                min: (bldg_center - 3).with_z(base - (bldg_diameter / 3) + 2),
                max: (bldg_center + 3)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2),
            };
            let bldg_room2 = Aabb {
                min: (bldg_center - (bldg_diameter / 6)).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6),
                ),
                max: (bldg_center + (bldg_diameter / 6)).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                ),
            };
            let bldg_room2_windows1 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 1,
                    bldg_center.y - (bldg_diameter / 6),
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                ),
                max: Vec3::new(
                    bldg_center.x + 1,
                    bldg_center.y - (bldg_diameter / 6) + 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                ),
            });
            let bldg_room2_windows2 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 1,
                    bldg_center.y + (bldg_diameter / 6) - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                ),
                max: Vec3::new(
                    bldg_center.x + 1,
                    bldg_center.y + (bldg_diameter / 6),
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                ),
            });
            let bldg_room2_windows3 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 6),
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                ),
                max: Vec3::new(
                    bldg_center.x - (bldg_diameter / 6) + 1,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                ),
            });
            let bldg_room2_windows4 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x + (bldg_diameter / 6) - 1,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 6),
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                ),
            });
            let bldg_room2_top_half = painter.aabb(Aabb {
                min: (bldg_center - (bldg_diameter / 6) + 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
                max: (bldg_center + (bldg_diameter / 6)).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                ),
            });
            let bldg_room2_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 6) + 1).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6) + 1,
                    ),
                    max: (bldg_center + (bldg_diameter / 6)).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                    ),
                })
                .intersect(bldg_room2_top_half);
            let bldg_room2_washed_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 6)).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6),
                    ),
                    max: (bldg_center + (bldg_diameter / 6)).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                    ),
                })
                .intersect(bldg_room2_top_half);
            let bldg_room2_goldring = painter.cylinder(Aabb {
                min: (bldg_center - (bldg_diameter / 6))
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) + 1),
                max: (bldg_center + (bldg_diameter / 6))
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) + 2),
            });
            let bldg_room2_goldring_clear = painter.cylinder(Aabb {
                min: (bldg_center - (bldg_diameter / 6) + 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) + 1),
                max: (bldg_center + (bldg_diameter / 6) - 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) + 2),
            });
            let bldg_room2_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 6) + 1).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6),
                ),
                max: (bldg_center + (bldg_diameter / 6) - 1).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                ),
            };
            let bldg_room2_floor = Aabb {
                min: (bldg_center - (bldg_diameter / 6) + 1).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10),
                ),
                max: (bldg_center + (bldg_diameter / 6) - 1).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 1,
                ),
            };
            let bldg_tube = painter.cylinder(Aabb {
                min: (bldg_center - 4).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6) - 1,
                ),
                max: (bldg_center + 4)
                    .with_z(base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4)),
            });
            let bldg_tube_clear = painter.cylinder(Aabb {
                min: (bldg_center - 3).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6) - 1,
                ),
                max: (bldg_center + 3)
                    .with_z(base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4)),
            });
            let bldg_tube_windows1 = Aabb {
                min: Vec3::new(
                    bldg_center.x + 3,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                ),
                max: Vec3::new(
                    bldg_center.x + 4,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2,
                ),
            };
            let bldg_tube_windows2 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 4,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                ),
                max: Vec3::new(
                    bldg_center.x - 3,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2,
                ),
            };
            let bldg_tube_windows3 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 1,
                    bldg_center.y - 4,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                ),
                max: Vec3::new(
                    bldg_center.x + 1,
                    bldg_center.y - 3,
                    base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2,
                ),
            };
            let bldg_tube_windows4 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 1,
                    bldg_center.y + 3,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                ),
                max: Vec3::new(
                    bldg_center.x + 1,
                    bldg_center.y + 4,
                    base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2,
                ),
            };
            let bldg_room3 = Aabb {
                min: (bldg_center - (bldg_diameter / 7))
                    .with_z(base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2),
                max: (bldg_center + (bldg_diameter / 7)).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 2,
                ),
            };
            let bldg_room3_washed_top_half = painter.aabb(Aabb {
                min: (bldg_center - (bldg_diameter / 7)).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 2,
                ),
                max: (bldg_center + (bldg_diameter / 7)).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 2,
                ),
            });
            let bldg_room3_washed_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 7)).with_z(
                        base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2,
                    ),
                    max: (bldg_center + (bldg_diameter / 7)).with_z(
                        base - (bldg_diameter / 15)
                            + tower_height
                            + (bldg_diameter / 4)
                            + (2 * (bldg_diameter / 7))
                            - 2,
                    ),
                })
                .intersect(bldg_room3_washed_top_half);
            let bldg_room3_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 7) + 1).with_z(
                        base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 1,
                    ),
                    max: (bldg_center + (bldg_diameter / 7)).with_z(
                        base - (bldg_diameter / 15)
                            + tower_height
                            + (bldg_diameter / 4)
                            + (2 * (bldg_diameter / 7))
                            - 2,
                    ),
                })
                .intersect(bldg_room3_washed_top_half);
            let bldg_room3_goldring = Aabb {
                min: (bldg_center - (bldg_diameter / 7)).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 1,
                ),
                max: (bldg_center + (bldg_diameter / 7)).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7),
                ),
            };
            let bldg_room3_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 7) + 1)
                    .with_z(base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 1),
                max: (bldg_center + (bldg_diameter / 7) - 1).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 3,
                ),
            };
            let bldg_room3_floor = painter.cylinder(Aabb {
                min: (bldg_center - 3).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 8,
                ),
                max: (bldg_center + 3).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 7,
                ),
            });
            let bldg_room3_floor_clear = painter.cylinder(Aabb {
                min: (bldg_center - 2).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 8,
                ),
                max: (bldg_center + 2).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 7,
                ),
            });
            let bldg_tower_floors_clear = Aabb {
                min: (bldg_center - 3).with_z(base - (bldg_diameter / 3) + 2),
                max: (bldg_center + 3).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 4)
                        - 3,
                ),
            };
            let bldg_room3_entry_clear1 = Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 7),
                    bldg_center.y - 3,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 6,
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 7),
                    bldg_center.y + 3,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 4,
                ),
            };
            let bldg_room3_entry_clear2 = Aabb {
                min: Vec3::new(
                    bldg_center.x - (bldg_diameter / 7),
                    bldg_center.y - 2,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 4,
                ),
                max: Vec3::new(
                    bldg_center.x + (bldg_diameter / 7),
                    bldg_center.y + 2,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 3,
                ),
            };
            let bldg_room3_entry_clear3 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 3,
                    bldg_center.y - (bldg_diameter / 7),
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 6,
                ),
                max: Vec3::new(
                    bldg_center.x + 3,
                    bldg_center.y + (bldg_diameter / 7),
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 4,
                ),
            };
            let bldg_room3_entry_clear4 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 2,
                    bldg_center.y - (bldg_diameter / 7),
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 4,
                ),
                max: Vec3::new(
                    bldg_center.x + 2,
                    bldg_center.y + (bldg_diameter / 7),
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (bldg_diameter / 7)
                        - 3,
                ),
            };
            let bldg_gold_top1 = Aabb {
                min: (bldg_center - 2).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 3,
                ),
                max: (bldg_center + 2).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 1,
                ),
            };
            let bldg_gold_top_pole = Aabb {
                min: (bldg_center - 1).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 1,
                ),
                max: (bldg_center + 1).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 7,
                ),
            };
            let bldg_gold_top_antlers1 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 2,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 1,
                ),
                max: Vec3::new(
                    bldg_center.x + 2,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 2,
                ),
            };
            let bldg_gold_top_antlers2 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 3,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 2,
                ),

                max: Vec3::new(
                    bldg_center.x + 3,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 3,
                ),
            });
            let bldg_gold_top_antlers2_clear = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 2,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 2,
                ),

                max: Vec3::new(
                    bldg_center.x + 2,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 3,
                ),
            });
            let bldg_gold_top_antlers3 = Aabb {
                min: Vec3::new(
                    bldg_center.x - 3,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 4,
                ),
                max: Vec3::new(
                    bldg_center.x + 3,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 5,
                ),
            };
            let bldg_gold_top_antlers4 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 5,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 5,
                ),

                max: Vec3::new(
                    bldg_center.x + 5,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 6,
                ),
            });
            let bldg_gold_top_antlers4_clear = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 2,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 5,
                ),

                max: Vec3::new(
                    bldg_center.x + 2,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 6,
                ),
            });
            let bldg_gold_top_antlers5 = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 2,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 7,
                ),

                max: Vec3::new(
                    bldg_center.x + 2,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 8,
                ),
            });
            let bldg_gold_top_antlers5_clear = painter.aabb(Aabb {
                min: Vec3::new(
                    bldg_center.x - 1,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 7,
                ),
                max: Vec3::new(
                    bldg_center.x + 1,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        + 8,
                ),
            });
            let bldg_tower_ropefix1 = Aabb {
                min: (bldg_center - 2).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 4,
                ),
                max: (bldg_center + 2).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 3,
                ),
            };
            let bldg_tower_ropefix2 = Aabb {
                min: (bldg_center).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 5,
                ),
                max: (bldg_center + 1).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 4,
                ),
            };
            let bldg_tower_rope = Aabb {
                min: bldg_center.with_z(base - (5 * (bldg_diameter / 4)) - 5),
                max: (bldg_center + 1).with_z(
                    base - (bldg_diameter / 15)
                        + tower_height
                        + (bldg_diameter / 4)
                        + (2 * (bldg_diameter / 7))
                        - 5,
                ),
            };
            let bldg_hut_ropefix1 = Aabb {
                min: (bldg_center - 2)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 3),
                max: (bldg_center + 2)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2),
            };
            let bldg_hut_ropefix2 = Aabb {
                min: bldg_center.with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 4),
                max: (bldg_center + 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 3),
            };
            let bldg_hut_rope = Aabb {
                min: bldg_center.with_z(base - (5 * (bldg_diameter / 4)) - 5),
                max: (bldg_center + 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 4),
            };
            let bldg_room_lantern_pos = (bldg_center + 2).with_z(base - (bldg_diameter / 15) + 2);
            let bldg_floor_lantern_pos =
                (bldg_center + 2).with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 2);
            let bldg_floor2_lantern_pos = Vec3::new(
                bldg_center.x + 3,
                bldg_center.y + 2,
                base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 1,
            );
            let bldg_floor3_lantern_pos = (bldg_center + 2).with_z(
                base - (bldg_diameter / 15)
                    + tower_height
                    + (bldg_diameter / 4)
                    + (bldg_diameter / 7)
                    - 7,
            );
            let bldg_floor3_drawer_pos = (bldg_center - 3).with_z(
                base - (bldg_diameter / 15)
                    + tower_height
                    + (bldg_diameter / 4)
                    + (bldg_diameter / 7)
                    - 7,
            );
            let bldg_floor3_potion_pos = (bldg_center - 3).with_z(
                base - (bldg_diameter / 15)
                    + tower_height
                    + (bldg_diameter / 4)
                    + (bldg_diameter / 7)
                    - 6,
            );
            let bldg_cellar_chest_pos = Vec3::new(
                bldg_center.x - (bldg_diameter / 8),
                bldg_center.y,
                base - (bldg_diameter / 3) + 3,
            );
            let bldg_floor_bed_pos = Vec3::new(
                bldg_center.x - (bldg_diameter / 6),
                bldg_center.y,
                base - (bldg_diameter / 15) + (bldg_diameter / 4) + 2,
            );
            let bldg_floor_drawer_pos = (bldg_center - (bldg_diameter / 8))
                .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 2);
            let bldg_floor_potion_pos = (bldg_center - (bldg_diameter / 8))
                .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 3);
            let bldg_floor2_wall = Aabb {
                min: (bldg_center - 4).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 1,
                ),
                max: (bldg_center + 4).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6) - 2,
                ),
            };

            let bldg_underwater_exit_pos_1 =
                center + dir * (2 * ((diameter / 3) - (diameter / 10)));
            let bldg_underwater_exit_pos_2 = center + dir * (2 * ((diameter / 3) - (diameter / 7)));
            let bldg_underwater_tube = painter.cubic_bezier(
                bldg_center.with_z(base - (5 * (bldg_diameter / 4)) + 2),
                bldg_center.with_z(base - (6 * (bldg_diameter / 4))),
                bldg_underwater_exit_pos_1.with_z(base - (6 * (bldg_diameter / 4))),
                bldg_underwater_exit_pos_2.with_z(base - (5 * (bldg_diameter / 4))),
                ((bldg_diameter / 10) + 1) as f32,
            );
            let bldg_underwater_tube_clear = painter.cubic_bezier(
                bldg_center.with_z(base - (5 * (bldg_diameter / 4)) + 2),
                bldg_center.with_z(base - (6 * (bldg_diameter / 4))),
                bldg_underwater_exit_pos_1.with_z(base - (6 * (bldg_diameter / 4))),
                bldg_underwater_exit_pos_2.with_z(base - (5 * (bldg_diameter / 4))),
                (bldg_diameter / 10) as f32,
            );
            let bldg_underwater_exit = painter.cylinder(Aabb {
                min: (bldg_underwater_exit_pos_2 - (bldg_diameter / 10) - 1)
                    .with_z(base - (5 * (bldg_diameter / 4)) + 1),
                max: (bldg_underwater_exit_pos_2 + (bldg_diameter / 10) + 1)
                    .with_z(base - (4 * (bldg_diameter / 4)) + 2),
            });
            let bldg_underwater_exit_clear = painter.cylinder(Aabb {
                min: (bldg_underwater_exit_pos_2 - (bldg_diameter / 10))
                    .with_z(base - (5 * (bldg_diameter / 4)) + 1),
                max: (bldg_underwater_exit_pos_2 + (bldg_diameter / 10))
                    .with_z(base - (4 * (bldg_diameter / 4)) + 1),
            });
            let bldg_connect_tube = Aabb {
                min: (bldg_center - (bldg_diameter / 10))
                    .with_z(base - (5 * (bldg_diameter / 4)) + 2),
                max: (bldg_center + (bldg_diameter / 10)).with_z(base - (bldg_diameter / 3) + 1),
            };
            let bldg_connect_step_1 = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 1)
                    .with_z(base - (2 * (bldg_diameter / 4))),
                max: (bldg_center + (bldg_diameter / 10) - 1)
                    .with_z(base - (2 * (bldg_diameter / 4)) + 1),
            };
            let bldg_connect_step_1_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 2)
                    .with_z(base - (2 * (bldg_diameter / 4))),
                max: (bldg_center + (bldg_diameter / 10) - 2)
                    .with_z(base - (2 * (bldg_diameter / 4)) + 1),
            };
            let bldg_connect_step_2 = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 1)
                    .with_z(base - (3 * (bldg_diameter / 4))),
                max: (bldg_center + (bldg_diameter / 10) - 1)
                    .with_z(base - (3 * (bldg_diameter / 4)) + 1),
            };
            let bldg_connect_step_2_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 2)
                    .with_z(base - (3 * (bldg_diameter / 4))),
                max: (bldg_center + (bldg_diameter / 10) - 2)
                    .with_z(base - (3 * (bldg_diameter / 4)) + 1),
            };
            let bldg_connect_step_3 = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 1)
                    .with_z(base - (4 * (bldg_diameter / 4)) - 3),
                max: (bldg_center + (bldg_diameter / 10) - 1)
                    .with_z(base - (4 * (bldg_diameter / 4)) - 2),
            };
            let bldg_connect_step_3_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 2)
                    .with_z(base - (4 * (bldg_diameter / 4)) - 3),
                max: (bldg_center + (bldg_diameter / 10) - 2)
                    .with_z(base - (4 * (bldg_diameter / 4)) - 2),
            };
            let bldg_connect_tube_gold_ring = Aabb {
                min: (bldg_underwater_exit_pos_2 - (bldg_diameter / 10) + 1)
                    .with_z(base - (4 * (bldg_diameter / 4)) + 1),
                max: (bldg_underwater_exit_pos_2 + (bldg_diameter / 10) - 1)
                    .with_z(base - (4 * (bldg_diameter / 4)) + 2),
            };
            let bldg_connect_decor_ring = Aabb {
                min: (bldg_underwater_exit_pos_2 - (bldg_diameter / 10))
                    .with_z(base - (4 * (bldg_diameter / 4))),
                max: (bldg_underwater_exit_pos_2 + (bldg_diameter / 10))
                    .with_z(base - (4 * (bldg_diameter / 4)) + 1),
            };
            let bldg_connect_decor_ring_clear = Aabb {
                min: (bldg_underwater_exit_pos_2 - (bldg_diameter / 10) + 1)
                    .with_z(base - (4 * (bldg_diameter / 4))),
                max: (bldg_underwater_exit_pos_2 + (bldg_diameter / 10) - 1)
                    .with_z(base - (4 * (bldg_diameter / 4)) + 1),
            };
            let bldg_connect_clear = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 1)
                    .with_z(base - (5 * (bldg_diameter / 4)) + 1),
                max: (bldg_center + (bldg_diameter / 10) - 1)
                    .with_z(base - (bldg_diameter / 3) + 2),
            };
            let bldg_connect_gate = Aabb {
                min: (bldg_underwater_exit_pos_2 - 2).with_z(base - (4 * (bldg_diameter / 4)) + 1),
                max: (bldg_underwater_exit_pos_2 + 2).with_z(base - (4 * (bldg_diameter / 4)) + 2),
            };
            let bldg_connect_keyhole = Aabb {
                min: (bldg_underwater_exit_pos_2 - 1).with_z(base - (4 * (bldg_diameter / 4)) + 1),
                max: (bldg_underwater_exit_pos_2).with_z(base - (4 * (bldg_diameter / 4)) + 2),
            };
            let bldg_floor_sea_cleric_pos = (bldg_center + (bldg_diameter / 8))
                .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4) + 2);
            let bldg_floor3_sea_cleric_pos = (bldg_center + 2).with_z(
                base - (bldg_diameter / 15)
                    + tower_height
                    + (bldg_diameter / 4)
                    + (bldg_diameter / 7)
                    - 6,
            );
            // bldg cellar Sea Crocodiles
            let bldg_cellar_sea_croc_pos = bldg_center.with_z(base - (5 * (bldg_diameter / 4)) - 5);
            for _ in
                0..(2 + ((RandomField::new(0).get((bldg_cellar_sea_croc_pos).with_z(base))) % 2))
            {
                painter.spawn(
                    EntityInfo::at(bldg_cellar_sea_croc_pos.as_()).with_asset_expect(
                        "common.entity.wild.aggressive.sea_crocodile",
                        &mut rng,
                        None,
                    ),
                )
            }
            match bldg_variant {
                0..=2 => {
                    // paint SeaHut
                    painter.sphere(bldg_cellar).fill(white.clone());
                    painter.sphere(bldg_cellar_clear).clear();
                    painter.sphere(bldg_room).fill(white.clone());
                    painter.aabb(bldg_hut_entry_clear1).clear();
                    painter.aabb(bldg_hut_entry_clear2).clear();
                    bldg_room_windows_1.fill(window_ver.clone());
                    bldg_room_windows_2.fill(window_ver.clone());
                    bldg_washed_top.fill(washed.clone());
                    bldg_top.fill(top.clone());
                    painter.sphere(bldg_room_clear).clear();
                    bldg_room_goldring.fill(gold.clone());
                    bldg_room_goldring_clear.clear();
                    painter.cylinder(bldg_room_floor).fill(floor_color.clone());
                    painter.cylinder(bldg_hut_floors_clear).clear();
                    painter.cylinder(bldg_hut_ropefix1).fill(ropefix1.clone());
                    painter.aabb(bldg_hut_ropefix2).fill(ropefix2.clone());
                    bldg_underwater_tube.fill(white.clone());
                    bldg_underwater_tube_clear.clear();
                    bldg_underwater_exit.fill(white.clone());
                    bldg_underwater_exit_clear.clear();
                    painter.cylinder(bldg_connect_tube).fill(white.clone());
                    painter
                        .cylinder(bldg_connect_tube_gold_ring)
                        .fill(gold.clone());
                    painter.cylinder(bldg_connect_clear).clear();
                    painter
                        .cylinder(bldg_connect_decor_ring)
                        .fill(gold_decor.clone());
                    painter.cylinder(bldg_connect_decor_ring_clear).clear();
                    painter.cylinder(bldg_connect_step_1).fill(white.clone());
                    painter.cylinder(bldg_connect_step_1_clear).clear();
                    painter.cylinder(bldg_connect_step_2).fill(white.clone());
                    painter.cylinder(bldg_connect_step_2_clear).clear();
                    painter.cylinder(bldg_connect_step_3).fill(white.clone());
                    painter.cylinder(bldg_connect_step_3_clear).clear();
                    painter.aabb(bldg_hut_rope).fill(rope.clone());
                    painter.sprite(bldg_room_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_cellar_chest_pos, SpriteKind::DungeonChest1);
                    painter.sprite(bldg_floor_bed_pos, SpriteKind::BedWoodWoodlandHead);
                    painter.sprite(bldg_floor_drawer_pos, SpriteKind::DrawerWoodWoodlandS);
                    painter.sprite(bldg_floor_potion_pos, SpriteKind::PotionMinor);
                    // bldg floor Sea Clerics
                    for _ in 0..(1
                        + ((RandomField::new(0).get((bldg_floor_sea_cleric_pos).with_z(base))) % 2))
                    {
                        painter.spawn(
                            EntityInfo::at(bldg_floor_sea_cleric_pos.as_()).with_asset_expect(
                                "common.entity.dungeon.sea_chapel.sea_cleric",
                                &mut rng,
                                None,
                            ),
                        )
                    }
                },
                3..=5 => {
                    // paint SeaPavillon
                    painter.sphere(bldg_cellar).fill(white.clone());
                    painter.sphere(bldg_cellar_clear).clear();
                    painter.sphere(bldg_room).fill(white.clone());
                    painter.aabb(bldg_pavillon_entry_clear1).clear();
                    painter.aabb(bldg_pavillon_entry_clear2).clear();
                    painter.aabb(bldg_pavillon_entry_clear3).clear();
                    painter.aabb(bldg_pavillon_entry_clear4).clear();
                    painter.aabb(bldg_pavillon_entry_clear5).clear();
                    painter.aabb(bldg_pavillon_entry_clear6).clear();
                    bldg_washed_top.fill(washed.clone());
                    bldg_top.fill(top.clone());
                    painter.sphere(bldg_room_clear).clear();
                    bldg_room_goldring.fill(gold.clone());
                    bldg_room_goldring_clear.clear();
                    painter.cylinder(bldg_hut_floors_clear).clear();
                    painter.cylinder(bldg_hut_ropefix1).fill(ropefix1.clone());
                    painter.aabb(bldg_hut_ropefix2).fill(ropefix2.clone());
                    bldg_underwater_tube.fill(white.clone());
                    bldg_underwater_tube_clear.clear();
                    bldg_underwater_exit.fill(white.clone());
                    bldg_underwater_exit_clear.clear();
                    painter.cylinder(bldg_connect_tube).fill(white.clone());
                    painter
                        .cylinder(bldg_connect_tube_gold_ring)
                        .fill(gold.clone());
                    painter.cylinder(bldg_connect_clear).clear();
                    painter
                        .cylinder(bldg_connect_decor_ring)
                        .fill(gold_decor.clone());
                    painter.cylinder(bldg_connect_decor_ring_clear).clear();
                    painter.cylinder(bldg_connect_step_1).fill(white.clone());
                    painter.cylinder(bldg_connect_step_1_clear).clear();
                    painter.cylinder(bldg_connect_step_2).fill(white.clone());
                    painter.cylinder(bldg_connect_step_2_clear).clear();
                    painter.cylinder(bldg_connect_step_3).fill(white.clone());
                    painter.cylinder(bldg_connect_step_3_clear).clear();
                    painter.aabb(bldg_hut_rope).fill(rope.clone());
                    painter.sprite(bldg_room_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_cellar_chest_pos, SpriteKind::DungeonChest1);
                },
                6..=9 => {
                    // paint SeaTower
                    painter.sphere(bldg_cellar).fill(white.clone());
                    painter.sphere(bldg_cellar_clear).clear();
                    painter.sphere(bldg_room).fill(white.clone());
                    painter.aabb(bldg_hut_entry_clear1).clear();
                    painter.aabb(bldg_hut_entry_clear2).clear();
                    bldg_room_windows_1.fill(window_ver.clone());
                    bldg_room_windows_2.fill(window_ver.clone());
                    bldg_washed_top.fill(washed.clone());
                    bldg_top.fill(top.clone());
                    painter.sphere(bldg_room2).fill(white.clone());
                    bldg_room2_windows1.fill(window_ver.clone());
                    bldg_room2_windows2.fill(window_ver.clone());
                    bldg_room2_windows3.fill(window_ver2.clone());
                    bldg_room2_windows4.fill(window_ver2.clone());
                    bldg_room2_washed_top.fill(washed.clone());
                    bldg_room2_top.fill(top.clone());
                    painter.sphere(bldg_room2_clear).clear();
                    bldg_room2_goldring.fill(gold.clone());
                    bldg_room2_goldring_clear.clear();
                    painter.sphere(bldg_room_clear).clear();
                    bldg_room_goldring.fill(gold.clone());
                    bldg_room_goldring_clear.clear();
                    painter.cylinder(bldg_room_floor).fill(floor_color.clone());
                    painter.cylinder(bldg_room2_floor).fill(floor_color.clone());
                    bldg_tube.fill(white.clone());
                    bldg_tube_clear.clear();
                    painter.aabb(bldg_tube_windows1).fill(window_ver2.clone());
                    painter.aabb(bldg_tube_windows2).fill(window_ver2.clone());
                    painter.aabb(bldg_tube_windows3).fill(window_ver.clone());
                    painter.aabb(bldg_tube_windows4).fill(window_ver.clone());
                    painter.sphere(bldg_room3).fill(white.clone());
                    bldg_room3_washed_top.fill(washed.clone());
                    bldg_room3_top.fill(top.clone());
                    painter.cylinder(bldg_room3_goldring).fill(gold.clone());
                    painter.sphere(bldg_room3_clear).clear();
                    painter.aabb(bldg_room3_entry_clear1).clear();
                    painter.aabb(bldg_room3_entry_clear2).clear();
                    painter.aabb(bldg_room3_entry_clear3).clear();
                    painter.aabb(bldg_room3_entry_clear4).clear();
                    painter.cylinder(bldg_floor2_wall).fill(white.clone());
                    painter.cylinder(bldg_tower_floors_clear).clear();
                    bldg_room3_floor.fill(floor_color.clone());
                    bldg_room3_floor_clear.clear();
                    painter.cylinder(bldg_tower_ropefix1).fill(ropefix1.clone());
                    painter.aabb(bldg_tower_ropefix2).fill(ropefix2.clone());
                    bldg_gold_top_antlers2.fill(gold.clone());
                    bldg_gold_top_antlers2_clear.clear();
                    bldg_gold_top_antlers4.fill(gold.clone());
                    bldg_gold_top_antlers4_clear.clear();
                    bldg_gold_top_antlers5.fill(gold.clone());
                    bldg_gold_top_antlers5_clear.clear();
                    painter.sphere(bldg_gold_top1).fill(gold.clone());
                    painter.aabb(bldg_gold_top_pole).fill(gold.clone());
                    painter.aabb(bldg_gold_top_antlers1).fill(gold.clone());
                    painter.aabb(bldg_gold_top_antlers3).fill(gold.clone());
                    bldg_underwater_tube.fill(white.clone());
                    bldg_underwater_tube_clear.clear();
                    bldg_underwater_exit.fill(white.clone());
                    bldg_underwater_exit_clear.clear();
                    painter.cylinder(bldg_connect_tube).fill(white.clone());
                    painter
                        .cylinder(bldg_connect_tube_gold_ring)
                        .fill(gold.clone());
                    painter.cylinder(bldg_connect_clear).clear();
                    painter
                        .cylinder(bldg_connect_decor_ring)
                        .fill(gold_decor.clone());
                    painter.cylinder(bldg_connect_decor_ring_clear).clear();
                    painter.cylinder(bldg_connect_step_1).fill(white.clone());
                    painter.cylinder(bldg_connect_step_1_clear).clear();
                    painter.cylinder(bldg_connect_step_2).fill(white.clone());
                    painter.cylinder(bldg_connect_step_2_clear).clear();
                    painter.cylinder(bldg_connect_step_3).fill(white.clone());
                    painter.cylinder(bldg_connect_step_3_clear).clear();
                    painter.aabb(bldg_tower_rope).fill(rope.clone());
                    painter.sprite(bldg_room_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor2_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor3_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor3_drawer_pos, SpriteKind::DrawerWoodWoodlandS);
                    painter.sprite(bldg_floor3_potion_pos, SpriteKind::PotionMinor);
                    painter.sprite(bldg_cellar_chest_pos, SpriteKind::DungeonChest1);
                    painter.sprite(bldg_floor_bed_pos, SpriteKind::BedWoodWoodlandHead);
                    painter.sprite(bldg_floor_drawer_pos, SpriteKind::DrawerWoodWoodlandS);
                    painter.sprite(bldg_floor_potion_pos, SpriteKind::PotionMinor);
                    // bldg floor Sea Clerics
                    for _ in 0..(1
                        + ((RandomField::new(0).get((bldg_floor_sea_cleric_pos).with_z(base))) % 2))
                    {
                        painter.spawn(
                            EntityInfo::at(bldg_floor_sea_cleric_pos.as_()).with_asset_expect(
                                "common.entity.dungeon.sea_chapel.sea_cleric",
                                &mut rng,
                                None,
                            ),
                        )
                    }
                    // bldg floor3 Sea Clerics
                    for _ in 0..(1
                        + ((RandomField::new(0).get((bldg_floor3_sea_cleric_pos).with_z(base)))
                            % 2))
                    {
                        painter.spawn(
                            EntityInfo::at(bldg_floor3_sea_cleric_pos.as_()).with_asset_expect(
                                "common.entity.dungeon.sea_chapel.sea_cleric",
                                &mut rng,
                                None,
                            ),
                        )
                    }
                },
                _ => {},
            };
            let connect_gate_type = connect_gate_types.swap_remove(
                RandomField::new(0).get((center + dir).with_z(base)) as usize
                    % connect_gate_types.len(),
            );
            painter
                .cylinder(bldg_connect_gate)
                .fill(Fill::Block(Block::air(connect_gate_type)));
            if connect_gate_type == SpriteKind::GlassBarrier {
                painter
                    .cylinder(bldg_connect_keyhole)
                    .fill(glass_keyhole.clone());
            };
        }
        // surrounding buildings foundling, small hut, small pavillon
        for dir in NEIGHBORS {
            let su_bldg_variant =
                ((RandomField::new(0).get((center - dir).with_z(base))) % 10) as i32;
            let su_bldg_center = center + dir * (diameter + (3 * su_bldg_variant));
            let su_bldg_base = base - 2 + (su_bldg_variant / 2);
            let su_bldg_diameter = diameter;

            let foundling_bottom1 = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 8) - 3)
                    .with_z(su_bldg_base - (su_bldg_diameter / 5) - (su_bldg_diameter / 2)),
                max: (su_bldg_center + (su_bldg_diameter / 8) + 3)
                    .with_z(su_bldg_base - (su_bldg_diameter / 5) - (su_bldg_diameter / 4) + 6),
            };
            let foundling_bottom2 = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 8) - 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 5) - (su_bldg_diameter / 4) + 2),
                max: (su_bldg_center + (su_bldg_diameter / 8) + 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 5) + 2),
            };
            let foundling_top = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 8))
                    .with_z(su_bldg_base - (su_bldg_diameter / 5) - 1),
                max: (su_bldg_center + (su_bldg_diameter / 8))
                    .with_z(su_bldg_base - (su_bldg_diameter / 5) + (su_bldg_diameter / 4) - 1),
            };
            let su_bldg_bottom1 = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 6) - 1)
                    .with_z(su_bldg_base - (2 * (su_bldg_diameter / 3)) + 1),
                max: (su_bldg_center + (su_bldg_diameter / 6) + 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 3) + 1),
            };
            let su_bldg_bottom2 = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 3)),
                max: (su_bldg_center + (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 3) + (su_bldg_diameter / 3)),
            };
            let su_bldg_room = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 15)),
                max: (su_bldg_center + (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3)),
            };
            let su_bldg_hut_entries1 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - (su_bldg_diameter / 6) - 2,
                    su_bldg_center.y - 2,
                    su_bldg_base + (su_bldg_diameter / 15) - 2,
                ),
                max: Vec3::new(
                    su_bldg_center.x + (su_bldg_diameter / 6) + 2,
                    su_bldg_center.y + 2,
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
            };
            let su_bldg_hut_entries2 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - (su_bldg_diameter / 6) - 2,
                    su_bldg_center.y - 1,
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
                max: Vec3::new(
                    su_bldg_center.x + (su_bldg_diameter / 6) + 2,
                    su_bldg_center.y + 1,
                    su_bldg_base + (su_bldg_diameter / 15) + 2,
                ),
            };
            let su_bldg_top_half = painter.aabb(Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6)),
                max: (su_bldg_center + (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3)),
            });
            let su_bldg_washed_top = painter
                .sphere(Aabb {
                    min: (su_bldg_center - (su_bldg_diameter / 6))
                        .with_z(su_bldg_base - (su_bldg_diameter / 15)),
                    max: (su_bldg_center + (su_bldg_diameter / 6))
                        .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3)),
                })
                .intersect(su_bldg_top_half);
            let su_bldg_top = painter
                .sphere(Aabb {
                    min: (su_bldg_center - (su_bldg_diameter / 6) + 1)
                        .with_z(su_bldg_base - (su_bldg_diameter / 15) + 1),
                    max: (su_bldg_center + (su_bldg_diameter / 6))
                        .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3)),
                })
                .intersect(su_bldg_top_half);
            let su_bldg_goldring = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 1),
                max: (su_bldg_center + (su_bldg_diameter / 6))
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 2),
            };
            let su_bldg_room_clear = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 6) + 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + 1),
                max: (su_bldg_center + (su_bldg_diameter / 6) - 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3) - 1),
            };
            let su_bldg_floor = Aabb {
                min: (su_bldg_center - (su_bldg_diameter / 6) + 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 1),
                max: (su_bldg_center + (su_bldg_diameter / 6) - 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 2),
            };
            let su_bldg_room_lantern_pos =
                (su_bldg_center + 2).with_z(su_bldg_base - (su_bldg_diameter / 15) + 2);
            let su_bldg_floor_lantern_pos = (su_bldg_center + 2)
                .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 2);
            let su_bldg_floor_drawer_pos = (su_bldg_center - (su_bldg_diameter / 10))
                .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 2);
            let su_bldg_floor_potion_pos = (su_bldg_center - (su_bldg_diameter / 10))
                .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 3);
            let su_bldg_floor_bed_pos = Vec3::new(
                su_bldg_center.x - (su_bldg_diameter / 8),
                su_bldg_center.y,
                su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 2,
            );
            let su_bldg_floor_entry = Aabb {
                min: (su_bldg_center - 3)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 1),
                max: (su_bldg_center + 3)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6) + 2),
            };
            let su_bldg_ropefix1 = Aabb {
                min: (su_bldg_center - 2)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3) - 3),
                max: (su_bldg_center + 2)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3) - 2),
            };
            let su_bldg_ropefix2 = Aabb {
                min: (su_bldg_center)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3) - 4),
                max: (su_bldg_center + 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3) - 3),
            };
            let su_bldg_rope = Aabb {
                min: (su_bldg_center).with_z(su_bldg_base - (su_bldg_diameter / 15) + 3),
                max: (su_bldg_center + 1)
                    .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3) - 4),
            };
            let su_bldg_pavillon_entries1 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - (su_bldg_diameter / 6),
                    su_bldg_center.y - 4,
                    su_bldg_base + (su_bldg_diameter / 15) - 2,
                ),
                max: Vec3::new(
                    su_bldg_center.x + (su_bldg_diameter / 6),
                    su_bldg_center.y + 4,
                    su_bldg_base + (su_bldg_diameter / 15),
                ),
            };
            let su_bldg_pavillon_entries2 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - (su_bldg_diameter / 6),
                    su_bldg_center.y - 3,
                    su_bldg_base + (su_bldg_diameter / 15),
                ),
                max: Vec3::new(
                    su_bldg_center.x + (su_bldg_diameter / 6),
                    su_bldg_center.y + 3,
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
            };
            let su_bldg_pavillon_entries3 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - (su_bldg_diameter / 6),
                    su_bldg_center.y - 2,
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
                max: Vec3::new(
                    su_bldg_center.x + (su_bldg_diameter / 6),
                    su_bldg_center.y + 2,
                    su_bldg_base + (su_bldg_diameter / 15) + 2,
                ),
            };
            let su_bldg_pavillon_entries4 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 4,
                    su_bldg_center.y - (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15) - 2,
                ),
                max: Vec3::new(
                    su_bldg_center.x + 4,
                    su_bldg_center.y + (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15),
                ),
            };
            let su_bldg_pavillon_entries5 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 3,
                    su_bldg_center.y - (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15),
                ),
                max: Vec3::new(
                    su_bldg_center.x + 3,
                    su_bldg_center.y + (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
            };
            let su_bldg_pavillon_entries6 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 2,
                    su_bldg_center.y - (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
                max: Vec3::new(
                    su_bldg_center.x + 2,
                    su_bldg_center.y + (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15) + 2,
                ),
            };
            let su_bldg_pavillon_entries7 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 4,
                    su_bldg_center.y,
                    su_bldg_base + (su_bldg_diameter / 15) - 2,
                ),
                max: Vec3::new(
                    su_bldg_center.x + 4,
                    su_bldg_center.y + (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15),
                ),
            };
            let su_bldg_pavillon_entries8 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 2,
                    su_bldg_center.y,
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
                max: Vec3::new(
                    su_bldg_center.x + 2,
                    su_bldg_center.y + (su_bldg_diameter / 6),
                    su_bldg_base + (su_bldg_diameter / 15) + 2,
                ),
            };
            let su_bldg_pavillon_barriers_1 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 3,
                    su_bldg_center.y + (su_bldg_diameter / 6) - 3,
                    su_bldg_base + (su_bldg_diameter / 15),
                ),
                max: Vec3::new(
                    su_bldg_center.x + 3,
                    su_bldg_center.y + (su_bldg_diameter / 6) - 2,
                    su_bldg_base + (su_bldg_diameter / 15) + 1,
                ),
            };
            let su_bldg_pavillon_barriers_2 = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 4,
                    su_bldg_center.y + (su_bldg_diameter / 6) - 3,
                    su_bldg_base + (su_bldg_diameter / 15) - 1,
                ),
                max: Vec3::new(
                    su_bldg_center.x + 4,
                    su_bldg_center.y + (su_bldg_diameter / 6) - 2,
                    su_bldg_base + (su_bldg_diameter / 15),
                ),
            };
            let su_bldg_pavillon_barriers_keyhole = Aabb {
                min: Vec3::new(
                    su_bldg_center.x - 1,
                    su_bldg_center.y + (su_bldg_diameter / 6) - 3,
                    su_bldg_base + (su_bldg_diameter / 15) - 2,
                ),
                max: Vec3::new(
                    su_bldg_center.x,
                    su_bldg_center.y + (su_bldg_diameter / 6) - 2,
                    su_bldg_base + (su_bldg_diameter / 15) - 1,
                ),
            };
            match su_bldg_variant {
                0..=5 => {
                    // common parts for small hut / small pavillon,
                    painter.sphere(su_bldg_bottom1).fill(white.clone());
                    painter.sphere(su_bldg_bottom2).fill(white.clone());
                    painter.sphere(su_bldg_room).fill(white.clone());
                    su_bldg_washed_top.fill(washed.clone());
                    su_bldg_top.fill(top.clone());
                    painter.cylinder(su_bldg_goldring).fill(gold.clone());
                    painter.sphere(su_bldg_room_clear).clear();
                    painter.sprite(su_bldg_room_lantern_pos, SpriteKind::SeashellLantern);
                    match su_bldg_variant {
                        0..=3 => {
                            // small hut
                            painter.aabb(su_bldg_hut_entries1).clear();
                            painter.aabb(su_bldg_hut_entries2).clear();
                            painter.cylinder(su_bldg_floor).fill(white.clone());
                            painter.cylinder(su_bldg_floor_entry).clear();
                            painter.aabb(su_bldg_ropefix1).fill(ropefix1.clone());
                            painter.aabb(su_bldg_ropefix2).fill(ropefix2.clone());
                            painter.aabb(su_bldg_rope).fill(rope.clone());
                            painter.sprite(su_bldg_floor_lantern_pos, SpriteKind::SeashellLantern);
                            painter
                                .sprite(su_bldg_floor_drawer_pos, SpriteKind::DrawerWoodWoodlandS);
                            painter.sprite(su_bldg_floor_potion_pos, SpriteKind::PotionMinor);
                            painter.sprite(su_bldg_floor_bed_pos, SpriteKind::BedWoodWoodlandHead);
                        },
                        _ => {
                            // small pavillon, some with prisoners
                            if su_bldg_variant > 4 {
                                painter.aabb(su_bldg_pavillon_entries2).clear();
                                painter.aabb(su_bldg_pavillon_entries7).clear();
                                painter.aabb(su_bldg_pavillon_entries5).clear();
                                painter.aabb(su_bldg_pavillon_entries8).clear();
                                painter
                                    .aabb(su_bldg_pavillon_barriers_1)
                                    .fill(glass_barrier.clone());
                                painter
                                    .aabb(su_bldg_pavillon_barriers_2)
                                    .fill(glass_barrier.clone());
                                painter
                                    .aabb(su_bldg_pavillon_barriers_keyhole)
                                    .fill(glass_keyhole.clone());
                                let prisoner_pos = su_bldg_center.with_z(base + 2);
                                for _ in 0..(6
                                    + ((RandomField::new(0).get((prisoner_pos).with_z(base))) % 6))
                                {
                                    painter.spawn(
                                        EntityInfo::at(prisoner_pos.as_()).with_asset_expect(
                                            "common.entity.dungeon.sea_chapel.prisoner",
                                            &mut rng,
                                            None,
                                        ),
                                    )
                                }
                            } else {
                                painter.aabb(su_bldg_pavillon_entries1).clear();
                                painter.aabb(su_bldg_pavillon_entries2).clear();
                                painter.aabb(su_bldg_pavillon_entries3).clear();
                                painter.aabb(su_bldg_pavillon_entries4).clear();
                                painter.aabb(su_bldg_pavillon_entries5).clear();
                                painter.aabb(su_bldg_pavillon_entries6).clear();
                            };
                        },
                    }
                },
                6..=7 => {
                    // foundling
                    painter.sphere(foundling_bottom1).fill(white.clone());
                    painter.sphere(foundling_bottom2).fill(white.clone());
                    painter.sphere(foundling_top).fill(white.clone());
                },
                _ => {},
            };
        }
    }
}
