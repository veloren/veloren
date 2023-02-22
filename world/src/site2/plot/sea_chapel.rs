use super::*;
use crate::{
    site2::{plot::dungeon::spiral_staircase, util::Dir},
    util::{sampler::Sampler, RandomField, NEIGHBORS},
    Land, CONFIG,
};
use common::{
    generation::EntityInfo,
    terrain::{Block, BlockKind, SpriteKind},
};

use rand::prelude::*;
use std::sync::Arc;
use vek::*;

pub struct SeaChapel {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
}
impl SeaChapel {
    pub fn generate(_land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        Self {
            bounds,
            alt: CONFIG.sea_level as i32,
        }
    }
}

impl Structure for SeaChapel {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_seachapel\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_seachapel")]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let base = self.alt + 1;
        let center = self.bounds.center();
        let diameter = 54;
        let mut rng = thread_rng();
        // Fills
        let (top, washed) = match (RandomField::new(0).get(center.with_z(base))) % 2 {
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
        let white_polished = Fill::Brick(BlockKind::Rock, Rgb::new(202, 202, 202), 24);
        let white_coral = Fill::Sampling(Arc::new(|center| {
            let c = (RandomField::new(0).get(center) % 13) as u8 * 10 + 120;
            Some(Block::new(BlockKind::Rock, Rgb::new(c, c, c)))
        }));
        let white = match (RandomField::new(0).get(center.with_z(base - 1))) % 2 {
            0 => white_polished,
            _ => white_coral.clone(),
        };
        let gold = Fill::Brick(BlockKind::GlowingRock, Rgb::new(245, 232, 0), 10);
        let gold_chain = Fill::Block(Block::air(SpriteKind::SeaDecorChain));
        let gold_decor = Fill::Block(Block::air(SpriteKind::SeaDecorBlock));
        let window_hor = Fill::Block(Block::air(SpriteKind::SeaDecorWindowHor));
        let window_ver = Fill::Block(Block::air(SpriteKind::SeaDecorWindowVer));
        let window_ver2 = Fill::Block(
            Block::air(SpriteKind::SeaDecorWindowVer)
                .with_ori(2)
                .unwrap(),
        );
        let glass_barrier = Fill::Block(Block::air(SpriteKind::GlassBarrier));
        let sea_urchins = Fill::Block(Block::air(SpriteKind::SeaUrchin));
        // random exit from water basin to side building
        let mut connect_gate_types = vec![
            SpriteKind::GlassBarrier,
            SpriteKind::SeaDecorWindowHor,
            SpriteKind::SeaDecorWindowHor,
            SpriteKind::SeaDecorWindowHor,
        ];
        //Paint SeaChapel
        // balcony1
        let center_b1 = Vec2::new(center.x + (diameter / 2) - (diameter / 4), center.y);
        painter
            .cylinder(Aabb {
                min: (center_b1 - (diameter / 3) + 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 8),
                max: (center_b1 + (diameter / 3) - 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 7),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b1 - (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 7),
                max: (center_b1 + (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 6),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b1 - (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 6),
                max: (center_b1 + (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 5),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b1 - (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 5),
                max: (center_b1 + (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b1 - (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 5),
                max: (center_b1 + (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
            })
            .clear();
        painter
            .cylinder(Aabb {
                min: (center_b1 - (diameter / 3) - 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
                max: (center_b1 + (diameter / 3) + 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 3),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_b1 - (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
                max: (center_b1 + (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 3),
            })
            .clear();
        // balcony2
        let center_b2 = Vec2::new(center.x, center.y + (diameter / 2) - (diameter / 4));
        painter
            .cylinder(Aabb {
                min: (center_b2 - (diameter / 3) + 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 8),
                max: (center_b2 + (diameter / 3) - 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 7),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b2 - (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 7),
                max: (center_b2 + (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 6),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b2 - (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 6),
                max: (center_b2 + (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 5),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b2 - (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 5),
                max: (center_b2 + (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_b2 - (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 5),
                max: (center_b2 + (diameter / 3))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
            })
            .clear();
        painter
            .cylinder(Aabb {
                min: (center_b2 - (diameter / 3) - 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
                max: (center_b2 + (diameter / 3) + 2)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 3),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_b2 - (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 4),
                max: (center_b2 + (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 3),
            })
            .clear();
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
        // chapel main room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8)),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + diameter),
            })
            .fill(white.clone());
        // chapel main room entry1 stairs1
        // entry1 white floor
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) + 3,
                    center.y - 5,
                    base + (diameter / 8) - 4,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + (diameter / 5),
                    center.y + 5,
                    base + (diameter / 8) - 1,
                ),
            })
            .fill(white.clone());
        painter
            .ramp_inset(
                Aabb {
                    min: Vec3::new(
                        center.x - (diameter / 2) - 12,
                        center.y - 3,
                        base - (diameter / 8) - 7,
                    ),
                    max: Vec3::new(
                        center.x - (diameter / 2) + 2,
                        center.y + 3,
                        base - (diameter / 8) + 7,
                    ),
                },
                14,
                Dir::X,
            )
            .fill(white.clone());
        // chapel main room entry2 stairs
        // entry2 white floor
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - (diameter / 5),
                    center.y - 5,
                    base + (diameter / 8) - 4,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) - 3,
                    center.y + 5,
                    base + (diameter / 8) - 1,
                ),
            })
            .fill(white.clone());
        painter
            .ramp_inset(
                Aabb {
                    min: Vec3::new(
                        center.x + (diameter / 2) - 2,
                        center.y - 3,
                        base - (diameter / 8) - 7,
                    ),
                    max: Vec3::new(
                        center.x + (diameter / 2) + 12,
                        center.y + 3,
                        base - (diameter / 8) + 7,
                    ),
                },
                14,
                Dir::NegX,
            )
            .fill(white.clone());
        // chapel 1st washed out top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8)),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + diameter),
            })
            .without(painter.cylinder(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8)),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + (diameter / 2)),
            }))
            .fill(washed.clone());
        // chapel 1st top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2) + 1).with_z(base - (diameter / 8)),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + diameter),
            })
            .without(painter.cylinder(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8)),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + (diameter / 2)),
            }))
            .fill(top.clone());
        // chapel small top room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3)),
                max: (center + (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3)),
            })
            .fill(white.clone());
        // chapel small washed out top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3)),
                max: (center + (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3)),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center - (diameter / 3))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 3)),
                    max: (center + (diameter / 3)).with_z(base - (diameter / 8) + diameter),
                }),
            )
            .fill(washed.clone());
        // chapel small top
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3)),
                max: (center + (diameter / 3))
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3)),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center - (diameter / 3))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 3)),
                    max: (center + (diameter / 3)).with_z(base - (diameter / 8) + diameter),
                }),
            )
            .fill(top.clone());
        // ground to top room stairway3
        let center_s3 = Vec2::new(center.x, center.y - (diameter / 2));
        // stairway3 top room
        painter
            .sphere(Aabb {
                min: (center_s3 - (diameter / 6))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 3),
                max: (center_s3 + (diameter / 6))
                    .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 3) - 3),
            })
            .fill(white.clone());
        // stairway3 top washed out
        painter
            .sphere(Aabb {
                min: (center_s3 - (diameter / 6))
                    .with_z(base - (diameter / 8) + (diameter / 2) - 3),
                max: (center_s3 + (diameter / 6))
                    .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 3) - 3),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center_s3 - (diameter / 6))
                        .with_z(base - (diameter / 8) + (diameter / 2) - 3),
                    max: (center_s3 + (diameter / 6))
                        .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 3),
                }),
            )
            .fill(washed.clone());
        // stairway3 top
        painter
            .sphere(Aabb {
                min: (center_s3 - (diameter / 6) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 3),
                max: (center_s3 + (diameter / 6))
                    .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 3) - 3),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center_s3 - (diameter / 6))
                        .with_z(base - (diameter / 8) + (diameter / 2) - 3),
                    max: (center_s3 + (diameter / 6))
                        .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 3),
                }),
            )
            .fill(top.clone());
        // stairway3 top gold ring
        painter
            .cylinder(Aabb {
                min: (center_s3 - (diameter / 6))
                    .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 2),
                max: (center_s3 + (diameter / 6))
                    .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 1),
            })
            .fill(gold.clone());
        // stairway3 clear top halfway
        painter
            .sphere(Aabb {
                min: (center_s3 - (diameter / 6) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) - 2),
                max: (center_s3 + (diameter / 6) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 3) - 4),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center_s3 - (diameter / 6))
                        .with_z(base - (diameter / 8) + (diameter / 2))
                        - 2,
                    max: (center_s3 + (diameter / 6))
                        .with_z(base - (diameter / 8) + (diameter / 2) + (diameter / 10) - 3),
                }),
            )
            .clear();
        // stairway3 top window1
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s3.x + (diameter / 6) - 1,
                    center_s3.y - 1,
                    base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 5,
                ),
                max: Vec3::new(
                    center_s3.x + (diameter / 6),
                    center_s3.y + 1,
                    base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 4,
                ),
            })
            .fill(window_ver2.clone());
        // stairway3 top window2
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s3.x - (diameter / 6),
                    center_s3.y - 1,
                    base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 5,
                ),
                max: Vec3::new(
                    center_s3.x - (diameter / 6) + 1,
                    center_s3.y + 1,
                    base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 4,
                ),
            })
            .fill(window_ver2.clone());

        // stairway3 top window3
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s3.x - 1,
                    center_s3.y - (diameter / 6),
                    base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 5,
                ),
                max: Vec3::new(
                    center_s3.x + 1,
                    center_s3.y - (diameter / 6) + 1,
                    base - (diameter / 8) + (diameter / 2) + (diameter / 6) - 4,
                ),
            })
            .fill(window_ver.clone());

        // chapel clear room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 2) + 1).with_z(base - (diameter / 8) + 1),
                max: (center + (diameter / 2) - 1).with_z(base - (diameter / 8) + diameter - 1),
            })
            .clear();
        // chapel main room entry1 gold door frame
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) + 2,
                    center.y - 2,
                    base + (diameter / 8) + 5,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + 4,
                    center.y + 2,
                    base + (diameter / 8) + 6,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) + 2,
                    center.y - 3,
                    base + (diameter / 8) + 3,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + 5,
                    center.y + 3,
                    base + (diameter / 8) + 5,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) + 3,
                    center.y - 3,
                    base + (diameter / 8) - 3,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + 5,
                    center.y + 3,
                    base + (diameter / 8) + 3,
                ),
            })
            .fill(gold_decor.clone());
        // chapel main room entry2 gold door frame
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - 4,
                    center.y - 2,
                    base + (diameter / 8) + 5,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) - 2,
                    center.y + 2,
                    base + (diameter / 8) + 6,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - 5,
                    center.y - 3,
                    base + (diameter / 8) + 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) - 2,
                    center.y + 3,
                    base + (diameter / 8) + 5,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - 5,
                    center.y - 3,
                    base + (diameter / 8) - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) - 3,
                    center.y + 3,
                    base + (diameter / 8) + 3,
                ),
            })
            .fill(gold_decor.clone());
        // chapel main room clear entries
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) - 8,
                    center.y - 2,
                    base + (diameter / 8) - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) + 8,
                    center.y + 2,
                    base + (diameter / 8) + 4,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) - 8,
                    center.y - 1,
                    base + (diameter / 8) + 4,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) + 8,
                    center.y + 1,
                    base + (diameter / 8) + 5,
                ),
            })
            .clear();
        // chapel main room mobilees
        let mbl_corner = center - 5;
        for dir in SQUARE_4 {
            let mbl_center = mbl_corner + dir * 10;
            let mbl_offset =
                ((RandomField::new(0).get((mbl_corner - dir).with_z(base))) % 4) as i32 - 2;
            painter
                .cone(Aabb {
                    min: (mbl_center - 2)
                        .with_z(base - (diameter / 8) + (diameter / 3) + 2 + mbl_offset),
                    max: (mbl_center + 3)
                        .with_z(base - (diameter / 8) + (diameter / 3) + 2 + mbl_offset + 2),
                })
                .fill(top.clone());
            painter
                .cylinder(Aabb {
                    min: (mbl_center - 2)
                        .with_z(base - (diameter / 8) + (diameter / 3) + 1 + mbl_offset),
                    max: (mbl_center + 3)
                        .with_z(base - (diameter / 8) + (diameter / 3) + 2 + mbl_offset),
                })
                .fill(gold_decor.clone());
            painter
                .aabb(Aabb {
                    min: (mbl_center - 1)
                        .with_z(base - (diameter / 8) + (diameter / 3) + 1 + mbl_offset),
                    max: (mbl_center + 2)
                        .with_z(base - (diameter / 8) + (diameter / 3) + 2 + mbl_offset),
                })
                .clear();
            // chapel main room mobilee chains
            painter
                .aabb(Aabb {
                    min: Vec3::new(
                        mbl_center.x,
                        mbl_center.y,
                        base - (diameter / 8) + (diameter / 3) + 4 + mbl_offset,
                    ),
                    max: Vec3::new(
                        mbl_center.x + 1,
                        mbl_center.y + 1,
                        base - (diameter / 8) + (diameter / 2) + 1,
                    ),
                })
                .fill(gold_chain.clone());
        }
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

        // chapel main room window2 to balcony1
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
            .clear();
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
            .clear();
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
            .clear();
        // chapel main room window3 to balcony2
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
            .clear();
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
            .clear();
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
            .clear();
        // chapel gold ring and white floor
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2)).with_z(base - (diameter / 8) + (diameter / 2) + 1),
                max: (center + (diameter / 2)).with_z(base - (diameter / 8) + (diameter / 2) + 2),
            })
            .fill(gold.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 2) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) + 1),
                max: (center + (diameter / 2) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) + 2),
            })
            .fill(white.clone());
        // chapel main room organ podium
        let center_o = Vec2::new(center.x - (diameter / 4), center.y + (diameter / 4));
        painter
            .cylinder(Aabb {
                min: (center_o - 2).with_z(base),
                max: (center_o + 2).with_z(base + (diameter / 8) - 2),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_o - 3).with_z(base + (diameter / 8) - 2),
                max: (center_o + 3).with_z(base + (diameter / 8) - 1),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center_o - 4).with_z(base + (diameter / 8) - 1),
                max: (center_o + 4).with_z(base + (diameter / 8)),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_o - 3).with_z(base + (diameter / 8) - 1),
                max: (center_o + 3).with_z(base + (diameter / 8)),
            })
            .fill(white.clone());
        // organ on chapel main room organ podium
        let main_room_organ_pos = center_o.with_z(base + (diameter / 8));
        painter.spawn(
            EntityInfo::at(main_room_organ_pos.as_())
                .with_asset_expect("common.entity.dungeon.sea_chapel.organ", &mut rng),
        );
        // sea clerics in chapel main room
        let main_room_sea_clerics_pos = (center_o - 2).with_z(base + (diameter / 8));
        for _ in 0..(4 + ((RandomField::new(0).get((main_room_sea_clerics_pos).with_z(base))) % 4))
        {
            painter.spawn(
                EntityInfo::at(main_room_sea_clerics_pos.as_())
                    .with_asset_expect("common.entity.dungeon.sea_chapel.sea_cleric", &mut rng),
            )
        }
        // chapel first floor organ podium
        let center_o2 = Vec2::new(center.x - (diameter / 4), center.y - (diameter / 4));
        painter
            .cylinder(Aabb {
                min: (center_o2 - 4).with_z(base - (diameter / 8) + (diameter / 2) + 2),
                max: (center_o2 + 4).with_z(base - (diameter / 8) + (diameter / 2) + 3),
            })
            .fill(gold_decor.clone());
        painter
            .cylinder(Aabb {
                min: (center_o2 - 3).with_z(base - (diameter / 8) + (diameter / 2) + 2),
                max: (center_o2 + 3).with_z(base - (diameter / 8) + (diameter / 2) + 3),
            })
            .fill(white.clone());
        // organ on chapel first floor organ podium
        let first_floor_organ_pos = center_o2.with_z(base - (diameter / 8) + (diameter / 2) + 2);
        painter.spawn(
            EntityInfo::at(first_floor_organ_pos.as_())
                .with_asset_expect("common.entity.dungeon.sea_chapel.organ", &mut rng),
        );
        // sea clerics on first floor
        let first_floor_sea_clerics_pos =
            (center_o2 - 2).with_z(base - (diameter / 8) + (diameter / 2) + 2);
        for _ in
            0..(3 + ((RandomField::new(0).get((first_floor_sea_clerics_pos).with_z(base))) % 3))
        {
            painter.spawn(
                EntityInfo::at(first_floor_sea_clerics_pos.as_())
                    .with_asset_expect("common.entity.dungeon.sea_chapel.sea_cleric", &mut rng),
            )
        }
        // ground to top room stairway1
        let center_s1 = Vec2::new(
            center.x - (diameter / 2) + (diameter / 8),
            center.y - (diameter / 8) - (diameter / 16),
        );
        // stairway1 top room
        painter
            .sphere(Aabb {
                min: (center_s1 - (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4)),
                max: (center_s1 + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 3)),
            })
            .fill(white.clone());
        // stairway1 top washed out
        painter
            .sphere(Aabb {
                min: (center_s1 - (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4)),
                max: (center_s1 + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 3)),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center_s1 - (diameter / 6))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 4)),
                    max: (center_s1 + (diameter / 6))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6)),
                }),
            )
            .fill(washed.clone());
        // stairway1 top
        painter
            .sphere(Aabb {
                min: (center_s1 - (diameter / 6) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4)),
                max: (center_s1 + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 3)),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center_s1 - (diameter / 6))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 4)),
                    max: (center_s1 + (diameter / 6))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6)),
                }),
            )
            .fill(top.clone());
        // stairway1 top gold ring
        painter
            .cylinder(Aabb {
                min: (center_s1 - (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) + 1),
                max: (center_s1 + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) + 2),
            })
            .fill(gold.clone());
        // stairway1 top window1
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s1.x - (diameter / 6),
                    center_s1.y - 1,
                    base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) - 2,
                ),
                max: Vec3::new(
                    center_s1.x - (diameter / 6) + 1,
                    center_s1.y + 1,
                    base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) - 1,
                ),
            })
            .fill(window_ver2.clone());

        // stairway1 top window2
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s1.x - 1,
                    center_s1.y - (diameter / 6),
                    base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) - 2,
                ),
                max: Vec3::new(
                    center_s1.x + 1,
                    center_s1.y - (diameter / 6) + 1,
                    base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) - 1,
                ),
            })
            .fill(window_ver.clone());

        // stairway1 top window3
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s1.x - 1,
                    center_s1.y + (diameter / 6) - 1,
                    base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) - 2,
                ),
                max: Vec3::new(
                    center_s1.x + 1,
                    center_s1.y + (diameter / 6),
                    base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 6) - 1,
                ),
            })
            .fill(window_ver.clone());

        // ground to top room stairway2
        let center_s2 = Vec2::new(
            center.x + (diameter / 2) - (diameter / 6),
            center.y + (diameter / 8) + (diameter / 16),
        );
        // stairway2 top room
        painter
            .sphere(Aabb {
                min: (center_s2 - (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 10)),
                max: (center_s2 + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 3)),
            })
            .fill(white.clone());
        // stairway2 top washed out
        painter
            .sphere(Aabb {
                min: (center_s2 - (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 10)),
                max: (center_s2 + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 3)),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center_s2 - (diameter / 6))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 10)),
                    max: (center_s2 + (diameter / 6)).with_z(
                        base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6),
                    ),
                }),
            )
            .fill(washed.clone());
        // stairway2 top
        painter
            .sphere(Aabb {
                min: (center_s2 - (diameter / 6) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 10)),
                max: (center_s2 + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 3)),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center_s2 - (diameter / 6))
                        .with_z(base - (diameter / 8) + diameter - (diameter / 10)),
                    max: (center_s2 + (diameter / 6)).with_z(
                        base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6),
                    ),
                }),
            )
            .fill(top.clone());
        // stairway2 top gold ring
        painter
            .cylinder(Aabb {
                min: (center_s2 - (diameter / 6)).with_z(
                    base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6) + 1,
                ),
                max: (center_s2 + (diameter / 6)).with_z(
                    base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6) + 2,
                ),
            })
            .fill(gold.clone());
        // chapel clear top room
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 3) + 1),
                max: (center + (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3) - 1),
            })
            .clear();
        // stairway1 clear top halfway
        painter
            .sphere(Aabb {
                min: (center_s1 - (diameter / 6) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4) + 1),
                max: (center_s1 + (diameter / 6) - 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 4) + (diameter / 3) - 1),
            })
            .without(
                painter.sphere(Aabb {
                    min: (center_s1 - (diameter / 6) + 1)
                        .with_z(base - (diameter / 8) + diameter - (diameter / 4) + 1),
                    max: (center_s1 + (diameter / 6) - 1)
                        .with_z(base - (diameter / 8) + diameter - (diameter / 8)),
                }),
            )
            .clear();
        // stairway2 clear top halfway
        painter
            .sphere(Aabb {
                min: (center_s2 - (diameter / 6) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 10) + 1),
                max: (center_s2 + (diameter / 6) - 1).with_z(
                    base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 3) - 1,
                ),
            })
            .without(
                painter.sphere(Aabb {
                    min: (center_s2 - (diameter / 6) + 1)
                        .with_z(base - (diameter / 8) + diameter - (diameter / 10) + 1),
                    max: (center_s2 + (diameter / 6) - 1)
                        .with_z(base - (diameter / 8) + diameter + 2),
                }),
            )
            .clear();
        // stairway2 top window1
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s2.x + (diameter / 6) - 1,
                    center_s2.y - 1,
                    base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6) - 2,
                ),
                max: Vec3::new(
                    center_s2.x + (diameter / 6),
                    center_s2.y + 1,
                    base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6) - 1,
                ),
            })
            .fill(window_ver2.clone());

        // stairway2 top window2
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s2.x - 1,
                    center_s2.y + (diameter / 6) - 1,
                    base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6) - 2,
                ),
                max: Vec3::new(
                    center_s2.x + 1,
                    center_s2.y + (diameter / 6),
                    base - (diameter / 8) + diameter - (diameter / 10) + (diameter / 6) - 1,
                ),
            })
            .fill(window_ver.clone());
        // chapel tube1 / NPC-fence
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 6)).with_z(base - (diameter / 8) + (diameter / 2) + 2),
                max: (center + (diameter / 6)).with_z(base - (diameter / 8) + diameter + 1),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 6) + 1)
                    .with_z(base - (diameter / 8) + (diameter / 2) + 2),
                max: (center + (diameter / 6) - 1).with_z(base - (diameter / 8) + diameter + 1),
            })
            .clear();
        // chapel tube1 gold window
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 1,
                    base + (diameter / 2) + 5,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 1,
                    base + (diameter / 2) + 6,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 2,
                    base + (diameter / 2) + 4,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 2,
                    base + (diameter / 2) + 5,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 1,
                    base + (diameter / 2) + 4,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6) + 1,
                    center.y + 1,
                    base + (diameter / 2) + 5,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 3,
                    base + (diameter / 2) + 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 3,
                    base + (diameter / 2) + 4,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 2,
                    base + (diameter / 2) + 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6) + 1,
                    center.y + 2,
                    base + (diameter / 2) + 4,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 2,
                    base + (diameter / 2) + 1,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 2,
                    base + (diameter / 2) + 2,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 1,
                    base + (diameter / 2) + 1,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6) + 1,
                    center.y + 1,
                    base + (diameter / 2) + 2,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 1,
                    base + (diameter / 2),
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 1,
                    base + (diameter / 2) + 1,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 2,
                    base + (diameter / 2) + 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 2,
                    base + (diameter / 2) + 4,
                ),
            })
            .fill(window_ver2.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 1,
                    base + (diameter / 2) + 1,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 1,
                    base + (diameter / 2) + 5,
                ),
            })
            .fill(window_ver2.clone());
        // chapel floor1 mobilees and cages
        let floor1_corner = center - (diameter / 5);
        for dir in SQUARE_4 {
            let floor1_pos = floor1_corner + dir * (2 * diameter / 5);
            let floor1_variant = (RandomField::new(0).get((floor1_corner + dir).with_z(base))) % 10;
            match floor1_variant {
                // chapel first floor mobilee
                0..=4 => {
                    let floor1_mbl_top = Aabb {
                        min: (floor1_pos - 3)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 8),
                        max: (floor1_pos + 2)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 6),
                    };
                    let floor1_mbl_gold = painter.cylinder(Aabb {
                        min: (floor1_pos - 3)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 9),
                        max: (floor1_pos + 2)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 8),
                    });
                    let floor1_mbl_gold_clear = painter.aabb(Aabb {
                        min: (floor1_pos - 2)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 9),
                        max: (floor1_pos + 1)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 8),
                    });
                    let floor1_mbl_chain = Aabb {
                        min: (floor1_pos - 1)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 6),
                        max: (floor1_pos)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 1),
                    };
                    painter.cone(floor1_mbl_top).fill(top.clone());
                    floor1_mbl_gold.fill(gold_decor.clone());
                    floor1_mbl_gold_clear.clear();
                    painter.aabb(floor1_mbl_chain).fill(gold_chain.clone());
                },
                _ => {
                    // chapel floor1 hanging cages
                    let cage_glass_barriers = Aabb {
                        min: (floor1_pos - 3).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 5,
                        ),
                        max: (floor1_pos + 3)
                            .with_z(base - (diameter / 4) + diameter - (diameter / 8)),
                    };
                    let cage_clear1 = Aabb {
                        min: (floor1_pos - 3 + 1).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 5,
                        ),
                        max: (floor1_pos + 3 - 1)
                            .with_z(base - (diameter / 4) + diameter - (diameter / 8)),
                    };
                    let cage_clear2 = Aabb {
                        min: (floor1_pos - 3)
                            .with_z(base - (diameter / 4) + diameter - (diameter / 8) - 2),
                        max: (floor1_pos + 3)
                            .with_z(base - (diameter / 4) + diameter - (diameter / 8) - 1),
                    };
                    let cage_windows = Aabb {
                        min: (floor1_pos - 3 + 1).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 4,
                        ),
                        max: (floor1_pos + 3 - 1).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 5,
                        ),
                    };
                    let cage_platform = Aabb {
                        min: (floor1_pos - 1).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 4,
                        ),
                        max: (floor1_pos + 1).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 5,
                        ),
                    };
                    let cage_chain = Aabb {
                        min: (floor1_pos - 1).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 5,
                        ),
                        max: (floor1_pos + 1)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 2),
                    };
                    let cage_chain_fix = Aabb {
                        min: (floor1_pos - 1)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 2),
                        max: (floor1_pos + 1)
                            .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 1),
                    };
                    let cage_coral_chest_podium = Aabb {
                        min: (floor1_pos + 1).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 4,
                        ),
                        max: (floor1_pos + 2).with_z(
                            base - (diameter / 4) + diameter
                                - (diameter / 8)
                                - (2 * (diameter / 11))
                                + 6,
                        ),
                    };
                    let cage_coral_chest_pos = (floor1_pos + 1).with_z(
                        base - (diameter / 4) + diameter - (diameter / 8) - (2 * (diameter / 11))
                            + 6,
                    );
                    // first floor cage Sea Cleric
                    let cage_sea_cleric_pos = (floor1_pos - 1).with_z(
                        base - (diameter / 4) + diameter - (diameter / 8) - (2 * (diameter / 11))
                            + 5,
                    );
                    painter
                        .cylinder(cage_glass_barriers)
                        .fill(glass_barrier.clone());
                    painter.aabb(cage_clear1).clear();
                    painter.cylinder(cage_clear2).clear();
                    painter.aabb(cage_windows).fill(window_hor.clone());
                    painter.aabb(cage_platform).fill(gold_decor.clone());
                    painter.aabb(cage_chain).fill(gold_chain.clone());
                    painter.aabb(cage_chain_fix).fill(gold_decor.clone());
                    painter
                        .aabb(cage_coral_chest_podium)
                        .fill(gold_decor.clone());
                    painter.rotated_sprite(cage_coral_chest_pos, SpriteKind::CoralChest, 2);
                    painter.spawn(EntityInfo::at(cage_sea_cleric_pos.as_()).with_asset_expect(
                        "common.entity.dungeon.sea_chapel.sea_cleric_sceptre",
                        &mut rng,
                    ));
                },
            }
        }

        // chapel floor1 window1
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 3),
                    center.y,
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
                    center.y,
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
                    center.y,
                    base - (diameter / 8) + diameter - 1,
                ),
            })
            .fill(window_ver2.clone());

        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 3) - 1,
                    center.y - 1,
                    base - (diameter / 8) + diameter - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 3),
                    center.y,
                    base - (diameter / 8) + diameter - 2,
                ),
            })
            .fill(window_ver2.clone());

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
            .fill(window_ver.clone());

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
            .fill(window_ver.clone());

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

        // chapel floor1
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3)).with_z(base - (diameter / 8) + diameter + 1),
                max: (center + (diameter / 3)).with_z(base - (diameter / 8) + diameter + 2),
            })
            .fill(gold.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3) + 1).with_z(base - (diameter / 8) + diameter + 1),
                max: (center + (diameter / 3) - 1).with_z(base - (diameter / 8) + diameter + 2),
            })
            .fill(white.clone());
        // chapel tube2 / NPC-fence
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 6)).with_z(base - (diameter / 8) + diameter + 2),
                max: (center + (diameter / 6))
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3) - 3),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 6) + 1).with_z(base - (diameter / 8) + diameter + 2),
                max: (center + (diameter / 6) - 1)
                    .with_z(base - (diameter / 8) + diameter + (diameter / 3) - 3),
            })
            .clear();
        // chapel tube2 gold door
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 3,
                    base - (diameter / 8) + diameter + 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 3,
                    base - (diameter / 8) + diameter + 6,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 2,
                    base - (diameter / 8) + diameter + 6,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6),
                    center.y + 2,
                    base - (diameter / 8) + diameter + 7,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 2,
                    base - (diameter / 8) + diameter + 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6) + 1,
                    center.y + 2,
                    base - (diameter / 8) + diameter + 5,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 6) - 1,
                    center.y - 1,
                    base - (diameter / 8) + diameter + 5,
                ),
                max: Vec3::new(
                    center.x - (diameter / 6) + 1,
                    center.y + 1,
                    base - (diameter / 8) + diameter + 6,
                ),
            })
            .clear();
        //chapel floor2
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3) + 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 1),
                max: (center + (diameter / 3) - 1)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 8)),
            })
            .fill(white.clone());
        //chapel floor2 drawer and potion
        painter.sprite(
            (center - (diameter / 8)).with_z(base - (diameter / 8) + diameter - (diameter / 8)),
            SpriteKind::DrawerSmall,
        );
        painter.sprite(
            (center - (diameter / 8)).with_z(base - (diameter / 8) + diameter - (diameter / 8) + 1),
            SpriteKind::PotionMinor,
        );
        //chapel main room pillars1
        for dir in SQUARE_4 {
            let sq_corner = Vec2::new(center.x - 3, center.y - (diameter / 4) - 2);
            let pos = Vec3::new(
                sq_corner.x + (dir.x * 5),
                sq_corner.y + (dir.y * ((diameter / 2) + 2)),
                base - (diameter / 8) + (diameter / 6),
            );
            painter.sprite(pos, SpriteKind::SeaDecorPillar);
        }
        //chapel main room pillars2
        for dir in SQUARE_4 {
            let sq_corner = Vec2::new(center.x - (diameter / 2) + 6, center.y - 4);
            let pos = Vec3::new(
                sq_corner.x + (dir.x * (diameter - 13)),
                sq_corner.y + (dir.y * 7),
                base - (diameter / 8) + (diameter / 6) + 2,
            );
            painter.sprite(pos, SpriteKind::SeaDecorPillar);
        }
        //chapel floor1 pillars inside tube
        for dir in SQUARE_4 {
            let sq_corner = Vec2::new(center.x - (diameter / 8) - 2, center.y - 3);
            let pos = Vec3::new(
                sq_corner.x + (dir.x * ((diameter / 4) + 2)),
                sq_corner.y + (dir.y * 5),
                base - (diameter / 8) + (diameter / 2) + 2,
            );
            painter.sprite(pos, SpriteKind::SeaDecorPillar);
        }
        //chapel floor1 pillars outside tube
        for dir in SQUARE_4 {
            let sq_corner = Vec2::new(center.x - (diameter / 8) - 3, center.y - 4);
            let pos = Vec3::new(
                sq_corner.x + (dir.x * ((diameter / 4) + 4)),
                sq_corner.y + (dir.y * 7),
                base - (diameter / 8) + (diameter / 2) + 2,
            );
            painter.sprite(pos, SpriteKind::SeaDecorPillar);
        }
        //chapel floor2/3 pillars inside tube
        for dir in SQUARE_4 {
            for f in 0..2 {
                let sq_corner = Vec2::new(center.x - (diameter / 8) - 2, center.y - 3);
                let pos = Vec3::new(
                    sq_corner.x + (dir.x * ((diameter / 4) + 2)),
                    sq_corner.y + (dir.y * 5),
                    base - (diameter / 8) + diameter - (diameter / 8) + (f * ((diameter / 8) + 2)),
                );
                painter.sprite(pos, SpriteKind::SeaDecorPillar);
            }
        }
        //chapel floor2/3 pillars outside tube
        for dir in SQUARE_4 {
            for f in 0..2 {
                let sq_corner = Vec2::new(center.x - (diameter / 8) - 3, center.y - 4);
                let pos = Vec3::new(
                    sq_corner.x + (dir.x * ((diameter / 4) + 4)),
                    sq_corner.y + (dir.y * 7),
                    base - (diameter / 8) + diameter - (diameter / 8) + (f * ((diameter / 8) + 2)),
                );
                painter.sprite(pos, SpriteKind::SeaDecorPillar);
            }
        }
        // floor2 tube Sea Clerics
        let fl2_tb_sea_clerics_pos = (center + (diameter / 8) - 2)
            .with_z(base - (diameter / 8) + diameter - (diameter / 8) + 1);
        for _ in 0..(1 + (RandomField::new(0).get((fl2_tb_sea_clerics_pos).with_z(base))) % 3) {
            painter.spawn(
                EntityInfo::at(fl2_tb_sea_clerics_pos.as_()).with_asset_expect(
                    "common.entity.dungeon.sea_chapel.sea_cleric_sceptre",
                    &mut rng,
                ),
            )
        }
        // chapel upper floor exits
        let (exit_pos1, exit_pos2) = (
            Vec2::new(center.x, center.y + (diameter / 4)),
            Vec2::new(center.x, center.y - (diameter / 4)),
        );
        let floors_exit_distr = RandomField::new(0).get(center.with_z(base)) as usize % 2;
        let (floor2_exit_center, floor3_exit_center) = match floors_exit_distr {
            0 => (exit_pos1, exit_pos2),
            _ => (exit_pos2, exit_pos1),
        };
        // floor3 exit
        painter
            .cylinder(Aabb {
                min: (floor3_exit_center - 2).with_z(base - (diameter / 8) + diameter + 1),
                max: (floor3_exit_center + 2).with_z(base - (diameter / 8) + diameter + 2),
            })
            .fill(glass_barrier.clone());
        // floor2 exit
        painter
            .cylinder(Aabb {
                min: (floor2_exit_center - 2)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 8) - 1),
                max: (floor2_exit_center + 2)
                    .with_z(base - (diameter / 8) + diameter - (diameter / 8)),
            })
            .fill(glass_barrier.clone());
        // floor 2 Sea Clerics
        let fl2_sea_clerics_pos = Vec3::new(
            center.x - 3,
            center.y - (diameter / 4),
            base - (diameter / 8) + diameter - (diameter / 8) + 1,
        );
        for _ in 0..(2 + ((RandomField::new(0).get((fl2_sea_clerics_pos).with_z(base))) % 4)) {
            painter.spawn(
                EntityInfo::at(fl2_sea_clerics_pos.as_())
                    .with_asset_expect("common.entity.dungeon.sea_chapel.sea_cleric", &mut rng),
            )
        }
        // floor 3 Sea Clerics
        let fl3_sea_clerics_pos =
            (center - (diameter / 6)).with_z(base - (diameter / 8) + diameter + 2);
        for _ in 0..(2 + ((RandomField::new(0).get((fl3_sea_clerics_pos).with_z(base))) % 2)) {
            painter.spawn(
                EntityInfo::at(fl3_sea_clerics_pos.as_())
                    .with_asset_expect("common.entity.dungeon.sea_chapel.sea_cleric", &mut rng),
            )
        }
        // chapel gold top emblem4
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem5
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem6
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem7
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - 9,
                    center.y - 1,
                    base - (diameter / 8) + diameter + (diameter / 3) + 17,
                ),
                max: Vec3::new(
                    center.x + 9,
                    center.y + 1,
                    base - (diameter / 8) + diameter + (diameter / 3) + 19,
                ),
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem8
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem9
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem10
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem11
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem12
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
            .clear();
        // chapel gold top emblem13
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        painter
            .aabb(Aabb {
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
            })
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
        // chapel gold top emblem1
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        // chapel gold top emblem2
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        // chapel gold top emblem3
        painter
            .aabb(Aabb {
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
            })
            .fill(gold.clone());
        // chapel main room pulpit
        painter
            .sphere(Aabb {
                min: (center - (diameter / 6)).with_z(base - (diameter / 8)),
                max: (center + (diameter / 6)).with_z(base - (diameter / 8) + (diameter / 3)),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 6)).with_z(base - (diameter / 8) + (diameter / 6)),
                max: (center + (diameter / 6)).with_z(base - (diameter / 8) + (diameter / 3)),
            })
            .clear();
        // chapel main room pulpit gold ring
        painter
            .sphere(Aabb {
                min: (center - (diameter / 6)).with_z(base - (diameter / 8) + (diameter / 6)),
                max: (center + (diameter / 6)).with_z(base - (diameter / 8) + (diameter / 6) + 1),
            })
            .fill(gold_decor.clone());

        painter
            .cylinder(Aabb {
                min: (center - (diameter / 6) + 1).with_z(base - (diameter / 8) + (diameter / 6)),
                max: (center + (diameter / 6) - 1)
                    .with_z(base - (diameter / 8) + (diameter / 6) + 1),
            })
            .clear();
        // chapel clear rope & platform entries
        painter
            .cylinder(Aabb {
                min: (center - 3).with_z(base - (2 * (diameter / 3)) + 3),
                max: (center + 3).with_z(base - (diameter / 8) + diameter + (diameter / 3) - 2),
            })
            .clear();
        //chapel main room gold window downwards
        painter
            .cylinder(Aabb {
                min: (center - 3).with_z(base - (diameter / 8)),
                max: (center + 3).with_z(base - (diameter / 8) + 1),
            })
            .fill(window_hor.clone());
        // chapel Ropefix1
        painter
            .cylinder(Aabb {
                min: (center - 2).with_z(base - (diameter / 8) + diameter + (diameter / 3) - 3),
                max: (center + 2).with_z(base - (diameter / 8) + diameter + (diameter / 3) - 2),
            })
            .fill(ropefix1.clone());
        // chapel Ropefix2
        painter
            .cylinder(Aabb {
                min: (center).with_z(base - (diameter / 8) + diameter + (diameter / 3) - 4),
                max: (center + 1).with_z(base - (diameter / 8) + diameter + (diameter / 3) - 3),
            })
            .fill(ropefix2.clone());
        // chapel rope
        painter
            .cylinder(Aabb {
                min: (center).with_z(base - (diameter / 8) + (diameter / 2) - 5),
                max: (center + 1).with_z(base - (diameter / 8) + diameter + (diameter / 3) - 4),
            })
            .fill(rope.clone());
        // chapel floor1 to cellar1 tube (access to dagons room)
        let t_center = Vec2::new(center.x + (diameter / 4) + 2, center.y + (diameter / 4) + 4);
        painter
            .cylinder(Aabb {
                min: (t_center - 2).with_z(base - (diameter / 8)),
                max: (t_center + 2).with_z(base - (diameter / 8) + (diameter / 2) + 2),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: (t_center - 1).with_z(base - (diameter / 8)),
                max: (t_center + 1).with_z(base - (diameter / 8) + (diameter / 2) + 2),
            })
            .clear();
        painter
            .cylinder(Aabb {
                min: (t_center - 1).with_z(base - (diameter / 8) + (diameter / 2) + 1),
                max: (t_center + 1).with_z(base - (diameter / 8) + (diameter / 2) + 2),
            })
            .fill(glass_barrier.clone());
        // chapel cellar2 water
        painter
            .cylinder(Aabb {
                min: (center - 5).with_z(base - (2 * (diameter / 3)) + 1),
                max: (center + 5).with_z(base - (2 * (diameter / 3)) + 2),
            })
            .fill(water.clone());
        painter
            .cylinder(Aabb {
                min: (center - 11).with_z(base - (2 * (diameter / 3)) + 2),
                max: (center + 11).with_z(base - (2 * (diameter / 3)) + 3),
            })
            .fill(water.clone());
        // chapel cellar to basin glass barrier barricade downwards & miniboss
        // cellar floor
        painter
            .cylinder(Aabb {
                min: (center - (2 * (diameter / 3))).with_z(base - (2 * (diameter / 3)) + 10),
                max: (center + (2 * (diameter / 3))).with_z(base - (2 * (diameter / 3)) + 16),
            })
            .fill(white_coral.clone());
        // exit to water basin
        let exit = Fill::Sampling(Arc::new(|center| {
            let c = RandomField::new(0).get(center - 6) % 11;
            Some(if c < 8 {
                let c = c as u8 * 10 + 120;
                Block::new(BlockKind::Rock, Rgb::new(c, c, c))
            } else {
                Block::air(SpriteKind::GlassBarrier)
            })
        }));
        // glass barriers with center clearance for dagon
        painter
            .cylinder(Aabb {
                min: (center - 9).with_z(base - (2 * (diameter / 3)) + 10),
                max: (center + 9).with_z(base - (2 * (diameter / 3)) + 16),
            })
            .fill(glass_barrier.clone());
        painter
            .cylinder(Aabb {
                min: (center - 9).with_z(base - (2 * (diameter / 3)) + 13),
                max: (center + 9).with_z(base - (2 * (diameter / 3)) + 14),
            })
            .without(painter.cylinder(Aabb {
                min: (center - 8).with_z(base - (2 * (diameter / 3)) + 13),
                max: (center + 8).with_z(base - (2 * (diameter / 3)) + 14),
            }))
            .fill(exit);
        painter
            .cylinder(Aabb {
                min: (center - 8).with_z(base - (2 * (diameter / 3)) + 14),
                max: (center + 8).with_z(base - (2 * (diameter / 3)) + 16),
            })
            .fill(white_coral.clone());
        painter
            .cylinder(Aabb {
                min: (center - 9).with_z(base - (2 * (diameter / 3)) + 16),
                max: (center + 9).with_z(base - (2 * (diameter / 3)) + 18),
            })
            .fill(sea_urchins);
        painter
            .cylinder(Aabb {
                min: (center - 8).with_z(base - (2 * (diameter / 3)) + 16),
                max: (center + 8).with_z(base - (2 * (diameter / 3)) + 18),
            })
            .clear();
        painter
            .cylinder(Aabb {
                min: (center - 8).with_z(base - (2 * (diameter / 3)) + 10),
                max: (center + 8).with_z(base - (2 * (diameter / 3)) + 14),
            })
            .fill(white_coral);
        let cellar_miniboss_pos = center.with_z(base - (2 * (diameter / 3)) + 17);
        painter.spawn(
            EntityInfo::at(cellar_miniboss_pos.as_())
                .with_asset_expect("common.entity.dungeon.sea_chapel.dagon", &mut rng),
        );
        // cellar 2 sea crocodiles
        let cellar_sea_croc_pos = (center - 12).with_z(base - (2 * (diameter / 3)) + 8);
        for _ in 0..(3 + ((RandomField::new(0).get((cellar_sea_croc_pos).with_z(base))) % 5)) {
            painter.spawn(
                EntityInfo::at(cellar_sea_croc_pos.as_())
                    .with_asset_expect("common.entity.wild.aggressive.sea_crocodile", &mut rng),
            )
        }
        // water basin
        painter
            .sphere(Aabb {
                min: (center - diameter + (diameter / 5) + 1).with_z(base - (4 * diameter / 3)),
                max: (center + diameter - (diameter / 5) - 1).with_z(base + 1),
            })
            .without(painter.cylinder(Aabb {
                min: (center - diameter + (diameter / 5) + 1).with_z(base - (2 * diameter / 3) + 1),
                max: (center + diameter - (diameter / 5) - 1).with_z(base + 1),
            }))
            .fill(white.clone());
        painter
            .sphere(Aabb {
                min: (center - diameter + (diameter / 5) + 2).with_z(base - (4 * diameter / 3) + 1),
                max: (center + diameter - (diameter / 5) - 2).with_z(base + 1),
            })
            .without(painter.cylinder(Aabb {
                min: (center - diameter + (diameter / 5) + 2).with_z(base - (2 * diameter / 3) + 1),
                max: (center + diameter - (diameter / 5) - 2).with_z(base + 1),
            }))
            .fill(water.clone());
        // stairway1 bottom
        painter
            .sphere(Aabb {
                min: (center_s1 - (diameter / 4))
                    .with_z(base - (2 * (diameter / 3)) + 3 - (diameter / 2)),
                max: (center_s1 + (diameter / 4)).with_z(base - (2 * (diameter / 3)) + 3),
            })
            .fill(white.clone());
        // stairway1 bottom gold ring
        painter
            .cylinder(Aabb {
                min: (center_s1 - (diameter / 4))
                    .with_z(base - (2 * (diameter / 3)) + 3 - (diameter / 4)),
                max: (center_s1 + (diameter / 4))
                    .with_z(base - (2 * (diameter / 3)) + 3 - (diameter / 4) + 1),
            })
            .fill(gold.clone());
        painter
            .sphere(Aabb {
                min: (center_s1 - (diameter / 4) + 1)
                    .with_z(base - (2 * (diameter / 3)) + 4 - (diameter / 2)),
                max: (center_s1 + (diameter / 4) - 1).with_z(base - (2 * (diameter / 3)) + 2),
            })
            .clear();
        // stairway1 bottom clear entry
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s1.x - 5,
                    center_s1.y - 5,
                    base - (2 * (diameter / 3)) + 1,
                ),
                max: Vec3::new(
                    center_s1.x + 5,
                    center_s1.y + 5,
                    base - (2 * (diameter / 3)) + 2,
                ),
            })
            .clear();
        // stairway1 bottom window to basin
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s1.x - 4,
                    center_s1.y - 4,
                    base - (2 * (diameter / 3)) + 4 - (diameter / 2),
                ),
                max: Vec3::new(
                    center_s1.x + 4,
                    center_s1.y + 4,
                    base - (2 * (diameter / 3)) + 5 - (diameter / 2),
                ),
            })
            .fill(window_hor);
        // stairway1 tube
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s1.x - 5,
                    center_s1.y - 5,
                    base - (2 * (diameter / 3)) + 1,
                ),
                max: Vec3::new(
                    center_s1.x + 5,
                    center_s1.y + 5,
                    base - (diameter / 8) + diameter - (diameter / 8),
                ),
            })
            .fill(white.clone());

        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s1.x - 4,
                    center_s1.y - 4,
                    base - (2 * (diameter / 3)) + 1,
                ),
                max: Vec3::new(
                    center_s1.x + 4,
                    center_s1.y + 4,
                    base - (diameter / 8) + diameter - (diameter / 8),
                ),
            })
            .clear();
        // stairway1 tube window1
        painter
            .aabb(Aabb {
                min: Vec3::new(center_s1.x - 5, center_s1.y - 1, base + (diameter / 6)),
                max: Vec3::new(
                    center_s1.x - 4,
                    center_s1.y + 1,
                    base - (diameter / 8) + diameter - (diameter / 4) + 1,
                ),
            })
            .fill(window_ver2.clone());

        // stairway1 stairs
        let stair_radius1 = 4.5;
        let stairs_clear1 = painter.cylinder(Aabb {
            min: (center_s1 - stair_radius1 as i32)
                .with_z(base - (2 * (diameter / 3)) + 3 - (diameter / 2)),
            max: (center_s1 + stair_radius1 as i32)
                .with_z(base - (diameter / 8) + diameter - (diameter / 8)),
        });
        stairs_clear1
            .sample(spiral_staircase(
                center_s1.with_z(base - (diameter / 8) + diameter - (diameter / 8)),
                stair_radius1,
                0.5,
                7.0,
            ))
            .fill(white.clone());
        // coral chest podium
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s1.x,
                    center_s1.y - 2,
                    base - (2 * (diameter / 3)) - (diameter / 2) + 12,
                ),
                max: Vec3::new(
                    center_s1.x + 1,
                    center_s1.y - 1,
                    base - (2 * (diameter / 3)) - (diameter / 2) + 13,
                ),
            })
            .fill(gold_decor.clone());
        // coral chest
        painter.rotated_sprite(
            Vec3::new(
                center_s1.x,
                center_s1.y - 2,
                base - (2 * (diameter / 3)) - (diameter / 2) + 13,
            ),
            SpriteKind::CoralChest,
            2,
        );
        //  cardinals room next to stairway1 bottom
        let cr_center = Vec2::new(center.x - diameter - 3, center.y - (diameter / 5) + 1);
        painter
            .cylinder(Aabb {
                min: (cr_center - (diameter / 3))
                    .with_z(base - (2 * (diameter / 3)) - (diameter / 4) - 5),
                max: (cr_center + (diameter / 3)).with_z(base - (2 * (diameter / 3)) + 3),
            })
            .fill(white.clone());
        // cardinals room gold ring
        painter
            .cylinder(Aabb {
                min: (cr_center - (diameter / 3))
                    .with_z(base - (2 * (diameter / 3)) - (diameter / 4) + 3),
                max: (cr_center + (diameter / 3))
                    .with_z(base - (2 * (diameter / 3)) - (diameter / 4) + 4),
            })
            .fill(gold.clone());
        // clear cardinals room
        painter
            .cylinder(Aabb {
                min: (cr_center - (diameter / 3) + 1)
                    .with_z(base - (2 * (diameter / 3)) - (diameter / 4) - 4),
                max: (cr_center + (diameter / 3) - 1).with_z(base - (2 * (diameter / 3)) + 2),
            })
            .clear();
        // Cardinals room chamber mobile1
        painter
            .cone(Aabb {
                min: (cr_center - 2).with_z(base - (2 * (diameter / 3)) - 7),
                max: (cr_center + 3).with_z(base - (2 * (diameter / 3)) - 5),
            })
            .fill(top.clone());
        painter
            .cylinder(Aabb {
                min: (cr_center - 2).with_z(base - (2 * (diameter / 3)) - 8),
                max: (cr_center + 3).with_z(base - (2 * (diameter / 3)) - 7),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: (cr_center - 1).with_z(base - (2 * (diameter / 3)) - 8),
                max: (cr_center + 2).with_z(base - (2 * (diameter / 3)) - 7),
            })
            .clear();
        // Cardinals room mobile1 chain
        painter
            .aabb(Aabb {
                min: cr_center.with_z(base - (2 * (diameter / 3)) - 5),
                max: (cr_center + 1).with_z(base - (2 * (diameter / 3)) + 2),
            })
            .fill(gold_chain.clone());
        // passage from stairway1 bottom to cardinals room
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    cr_center.x + (diameter / 3),
                    cr_center.y - 3,
                    base - (2 * (diameter / 3)) - (diameter / 4),
                ),
                max: Vec3::new(
                    cr_center.x + (diameter / 3) + 5,
                    cr_center.y + 3,
                    base - (2 * (diameter / 3)) - (diameter / 4) + 5,
                ),
            })
            .fill(white.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    cr_center.x + (diameter / 3),
                    cr_center.y - 2,
                    base - (2 * (diameter / 3)) - (diameter / 4) - 1,
                ),
                max: Vec3::new(
                    cr_center.x + (diameter / 3) + 5,
                    cr_center.y + 2,
                    base - (2 * (diameter / 3)) - (diameter / 4) + 6,
                ),
            })
            .fill(white.clone());
        // passage from stairway1 bottom to cardinals room gold stripes
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    cr_center.x + (diameter / 3),
                    cr_center.y - 3,
                    base - (2 * (diameter / 3)) - (diameter / 4) + 3,
                ),
                max: Vec3::new(
                    cr_center.x + (diameter / 3) + 5,
                    cr_center.y + 3,
                    base - (2 * (diameter / 3)) - (diameter / 4) + 4,
                ),
            })
            .fill(gold.clone());
        // clear passage from stairway1 bottom to cardinals room
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    cr_center.x + (diameter / 3) - 1,
                    cr_center.y - 2,
                    base - (2 * (diameter / 3)) - (diameter / 4) + 2,
                ),
                max: Vec3::new(
                    cr_center.x + (diameter / 3) + 6,
                    cr_center.y + 2,
                    base - (2 * (diameter / 3)) - (diameter / 4) + 5,
                ),
            })
            .clear();
        // Cardinals room Sea Clerics
        let cr_sea_clerics_pos =
            (cr_center - 2).with_z(base - (2 * (diameter / 3)) - (diameter / 4) - 4);
        for _ in 0..(2 + ((RandomField::new(0).get((fl2_sea_clerics_pos).with_z(base))) % 3)) {
            painter.spawn(
                EntityInfo::at(cr_sea_clerics_pos.as_())
                    .with_asset_expect("common.entity.dungeon.sea_chapel.sea_cleric", &mut rng),
            )
        }
        // Cardinal
        let cr_cardinal_pos =
            (cr_center + 2).with_z(base - (2 * (diameter / 3)) - (diameter / 4) - 4);
        painter.spawn(
            EntityInfo::at(cr_cardinal_pos.as_())
                .with_asset_expect("common.entity.dungeon.sea_chapel.cardinal", &mut rng),
        );
        // stairway2 bottom
        painter
            .sphere(Aabb {
                min: (center_s2 - (diameter / 4))
                    .with_z(base - (2 * (diameter / 3)) + 3 - (diameter / 2)),
                max: (center_s2 + (diameter / 4)).with_z(base - (2 * (diameter / 3)) + 3),
            })
            .fill(white.clone());
        // stairway2 bottom gold ring
        painter
            .cylinder(Aabb {
                min: (center_s2 - (diameter / 4))
                    .with_z(base - (2 * (diameter / 3)) + 3 - (diameter / 4)),
                max: (center_s2 + (diameter / 4))
                    .with_z(base - (2 * (diameter / 3)) + 3 - (diameter / 4) + 1),
            })
            .fill(gold.clone());
        painter
            .sphere(Aabb {
                min: (center_s2 - (diameter / 4) + 1)
                    .with_z(base - (2 * (diameter / 3)) + 4 - (diameter / 2)),
                max: (center_s2 + (diameter / 4) - 1).with_z(base - (2 * (diameter / 3)) + 2),
            })
            .fill(water.clone());
        // stairway2 bottom clear entry
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s2.x - 5,
                    center_s2.y - 5,
                    base - (2 * (diameter / 3)) + 1,
                ),
                max: Vec3::new(
                    center_s2.x + 5,
                    center_s2.y + 5,
                    base - (2 * (diameter / 3)) + 2,
                ),
            })
            .clear();
        // stairway2 bottom water to basin
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s2.x - 4,
                    center_s2.y - 4,
                    base - (2 * (diameter / 3)) + 4 - (diameter / 2),
                ),
                max: Vec3::new(
                    center_s2.x + 4,
                    center_s2.y + 4,
                    base - (2 * (diameter / 3)) + 5 - (diameter / 2),
                ),
            })
            .fill(water.clone());
        // stairway2 tube
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s2.x - 5,
                    center_s2.y - 5,
                    base - (2 * (diameter / 3)) + 1,
                ),
                max: Vec3::new(
                    center_s2.x + 5,
                    center_s2.y + 5,
                    base - (diameter / 8) + diameter + 2,
                ),
            })
            .fill(white.clone());
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s2.x - 4,
                    center_s2.y - 4,
                    base - (2 * (diameter / 3)) + 1,
                ),
                max: Vec3::new(
                    center_s2.x + 4,
                    center_s2.y + 4,
                    base - (diameter / 8) + diameter + 1,
                ),
            })
            .clear();
        painter
            .cylinder(Aabb {
                min: Vec3::new(
                    center_s2.x - 3,
                    center_s2.y - 3,
                    base - (diameter / 8) + diameter + 1,
                ),
                max: Vec3::new(
                    center_s2.x + 3,
                    center_s2.y + 3,
                    base - (diameter / 8) + diameter + 2,
                ),
            })
            .clear();
        // stairway2 tube window1
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center_s2.x + 4,
                    center_s2.y - 1,
                    base + (diameter / 8) + (diameter / 2),
                ),
                max: Vec3::new(
                    center_s2.x + 5,
                    center_s2.y + 1,
                    base - (diameter / 8) + diameter - 4,
                ),
            })
            .fill(window_ver2.clone());
        // chest
        painter.rotated_sprite(
            Vec3::new(
                center_s2.x - 5,
                center_s2.y - 4,
                base - (2 * (diameter / 3)) - (diameter / 2) + 11,
            ),
            SpriteKind::DungeonChest1,
            2,
        );
        // bottom 2 sea crocodiles
        let bt_sea_croc_pos = center_s2.with_z(base - (2 * (diameter / 3)) - (diameter / 2) + 11);
        for _ in 0..(2 + ((RandomField::new(0).get((bt_sea_croc_pos).with_z(base))) % 3)) {
            painter.spawn(
                EntityInfo::at(bt_sea_croc_pos.as_())
                    .with_asset_expect("common.entity.wild.aggressive.sea_crocodile", &mut rng),
            )
        }
        // underwater chamber
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3)).with_z(base - (4 * diameter / 3) - (diameter / 6)),
                max: (center + (diameter / 3)).with_z(base - (2 * diameter / 3) - (diameter / 6)),
            })
            .without(
                painter.sphere(Aabb {
                    min: (center - (diameter / 3) + 1)
                        .with_z(base - (4 * diameter / 3) - (diameter / 6) + 1),
                    max: (center + (diameter / 3) - 1)
                        .with_z(base - (2 * diameter / 3) - (diameter / 6) - 1),
                }),
            )
            .fill(white.clone());
        // underwater chamber entries
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 3),
                    center.y - 2,
                    base - (3 * diameter / 3) - (diameter / 3) + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 3),
                    center.y + 2,
                    base - (3 * diameter / 3) - (diameter / 6) - 2,
                ),
            })
            .fill(water.clone());
        // underwater chamber entries
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 3),
                    center.y - 1,
                    base - (3 * diameter / 3) - (diameter / 6) - 2,
                ),
                max: Vec3::new(
                    center.x + (diameter / 3),
                    center.y + 1,
                    base - (3 * diameter / 3) - (diameter / 6) - 1,
                ),
            })
            .fill(water.clone());
        // underwater chamber gold ring white floor
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3)).with_z(base - (3 * diameter / 3) - (diameter / 6)),
                max: (center + (diameter / 3))
                    .with_z(base - (3 * diameter / 3) - (diameter / 6) + 1),
            })
            .fill(gold.clone());
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 3) + 1)
                    .with_z(base - (3 * diameter / 3) - (diameter / 6)),
                max: (center + (diameter / 3) - 1)
                    .with_z(base - (3 * diameter / 3) - (diameter / 6) + 1),
            })
            .fill(white.clone());
        // underwater chamber floor entry
        painter
            .cylinder(Aabb {
                min: (center - 2).with_z(base - (3 * diameter / 3) - (diameter / 6)),
                max: (center + 2).with_z(base - (3 * diameter / 3) - (diameter / 6) + 1),
            })
            .fill(water.clone());
        // fill underwater chamber halfway with air
        painter
            .sphere(Aabb {
                min: (center - (diameter / 3) + 1)
                    .with_z(base - (4 * diameter / 3) - (diameter / 6) + 1),
                max: (center + (diameter / 3) - 1)
                    .with_z(base - (2 * diameter / 3) - (diameter / 6) - 1),
            })
            .without(
                painter.cylinder(Aabb {
                    min: (center - (diameter / 3) + 1)
                        .with_z(base - (4 * diameter / 3) - (diameter / 6) + 1),
                    max: (center + (diameter / 3) - 1)
                        .with_z(base - (3 * diameter / 3) - (diameter / 6) + 1),
                }),
            )
            .clear();
        // chapel underwater chamber mobile1
        painter
            .cone(Aabb {
                min: (center - 2).with_z(base - (2 * diameter / 3) - (diameter / 6) - 7),
                max: (center + 3).with_z(base - (2 * diameter / 3) - (diameter / 6) - 5),
            })
            .fill(top.clone());
        painter
            .cylinder(Aabb {
                min: (center - 2).with_z(base - (2 * diameter / 3) - (diameter / 6) - 8),
                max: (center + 3).with_z(base - (2 * diameter / 3) - (diameter / 6) - 7),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: (center - 1).with_z(base - (2 * diameter / 3) - (diameter / 6) - 8),
                max: (center + 2).with_z(base - (2 * diameter / 3) - (diameter / 6) - 7),
            })
            .clear();
        // chapel underwater chamber mobile1 chain
        painter
            .aabb(Aabb {
                min: center.with_z(base - (2 * diameter / 3) - (diameter / 6) - 5),
                max: (center + 1).with_z(base - (2 * diameter / 3) - (diameter / 6) - 1),
            })
            .fill(gold_chain);
        // underwater chamber coral chest
        painter
            .cylinder(Aabb {
                min: (center - (diameter / 6) - 1)
                    .with_z(base - (3 * diameter / 3) - (diameter / 6) + 1),
                max: (center - (diameter / 6) + 3)
                    .with_z(base - (3 * diameter / 3) - (diameter / 6) + 2),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: (center - (diameter / 6))
                    .with_z(base - (3 * diameter / 3) - (diameter / 6) + 1),
                max: (center - (diameter / 6) + 2)
                    .with_z(base - (3 * diameter / 3) - (diameter / 6) + 2),
            })
            .fill(white.clone());
        // coral chest
        painter.rotated_sprite(
            (center - (diameter / 6)).with_z(base - (3 * diameter / 3) - (diameter / 6) + 2),
            SpriteKind::CoralChest,
            2,
        );
        // underwater chamber sea crocodiles & sea clerics
        let uwc_sea_croc_pos =
            (center + (diameter / 8)).with_z(base - (3 * diameter / 3) - (diameter / 6) + 2);
        for _ in 0..(3 + ((RandomField::new(0).get((uwc_sea_croc_pos).with_z(base))) % 5)) {
            painter.spawn(
                EntityInfo::at(uwc_sea_croc_pos.as_())
                    .with_asset_expect("common.entity.wild.aggressive.sea_crocodile", &mut rng),
            )
        }
        let uwc_sea_clerics_pos =
            (center + (diameter / 7)).with_z(base - (3 * diameter / 3) - (diameter / 6) + 2);
        for _ in 0..(2 + ((RandomField::new(0).get((uwc_sea_clerics_pos).with_z(base))) % 2)) {
            painter.spawn(
                EntityInfo::at(uwc_sea_clerics_pos.as_())
                    .with_asset_expect("common.entity.dungeon.sea_chapel.sea_cleric", &mut rng),
            );
        }
        // Holding Cell2
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) - (diameter / 8),
                    center.y + (diameter / 16),
                    base - (diameter / 8),
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + (diameter / 8),
                    center.y + (diameter / 16) + (diameter / 4),
                    base - (diameter / 8) + (diameter / 4),
                ),
            })
            .fill(white.clone());
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) - (diameter / 8) + 1,
                    center.y + (diameter / 16) - 1,
                    base - (diameter / 8) + 1,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) + (diameter / 8) - 1,
                    center.y + (diameter / 16) + (diameter / 4) + 1,
                    base - (diameter / 8) + (diameter / 4) - 1,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) - (diameter / 8) - 3,
                    center.y + (diameter / 16) + (diameter / 8) - 1,
                    base - (diameter / 8) + (diameter / 16) + 2,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) - (diameter / 8) + 1,
                    center.y + (diameter / 16) + (diameter / 4) - (diameter / 8) + 1,
                    base - (diameter / 8) + (diameter / 4) - (diameter / 16) - 2,
                ),
            })
            .clear();
        painter.sprite(
            Vec3::new(
                center.x - (diameter / 2) - (diameter / 16) + 4,
                center.y + (diameter / 6) - 1,
                base - (diameter / 8) + (diameter / 16) - 1,
            ),
            SpriteKind::DungeonChest1,
        );
        // Holding Cell2 glass barriers
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 2) - (diameter / 8),
                    center.y + (diameter / 16) + (diameter / 8) - 1,
                    base - (diameter / 8) + (diameter / 16) + 3,
                ),
                max: Vec3::new(
                    center.x - (diameter / 2) - (diameter / 8) + 1,
                    center.y + (diameter / 16) + (diameter / 4) - (diameter / 8) + 1,
                    base - (diameter / 8) + (diameter / 4) - (diameter / 16) - 3,
                ),
            })
            .fill(glass_barrier.clone());
        // Holding Cell3
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - (diameter / 8),
                    center.y - (diameter / 4) - (diameter / 16),
                    base - (diameter / 8),
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) + (diameter / 8),
                    center.y - (diameter / 16),
                    base - (diameter / 8) + (diameter / 4),
                ),
            })
            .fill(white.clone());
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) - (diameter / 8) + 1,
                    center.y - (diameter / 4) - (diameter / 16) + 1,
                    base - (diameter / 8) + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) + (diameter / 8) - 1,
                    center.y - (diameter / 16) - 1,
                    base - (diameter / 8) + (diameter / 4) - 1,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) + (diameter / 8) - 1,
                    center.y - (diameter / 4) - (diameter / 16) + (diameter / 8) - 1,
                    base - (diameter / 8) + (diameter / 16) + 2,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) + (diameter / 8) + 3,
                    center.y - (diameter / 16) - (diameter / 8) + 1,
                    base - (diameter / 8) + (diameter / 4) - (diameter / 16) - 2,
                ),
            })
            .clear();
        painter.sprite(
            Vec3::new(
                center.x + (diameter / 2),
                center.y - (diameter / 8) - (diameter / 16),
                base - (diameter / 8) + 2,
            ),
            SpriteKind::DungeonChest1,
        );
        // Holding Cell3 glass barriers
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x + (diameter / 2) + (diameter / 8) - 1,
                    center.y - (diameter / 4) - (diameter / 16) + (diameter / 8) - 1,
                    base - (diameter / 8) + (diameter / 16) + 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 2) + (diameter / 8),
                    center.y - (diameter / 16) - (diameter / 8) + 1,
                    base - (diameter / 8) + (diameter / 16) + 4,
                ),
            })
            .fill(glass_barrier.clone());
        // Holding Cell1
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 4),
                    center.y - (diameter / 2) - (diameter / 4),
                    base - (diameter / 4),
                ),
                max: Vec3::new(
                    center.x + (diameter / 4),
                    center.y - (diameter / 2) + (diameter / 4),
                    base - (diameter / 4) + (diameter / 2),
                ),
            })
            .fill(white.clone());
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 4) + 1,
                    center.y - (diameter / 2) - (diameter / 4) + 1,
                    base - (diameter / 4) + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 4 - 1),
                    center.y - (diameter / 2) + (diameter / 4) - 1,
                    base - (diameter / 4) + (diameter / 2) - 1,
                ),
            })
            .without(painter.cylinder(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 4) + 1,
                    center.y - (diameter / 2) - (diameter / 4) + 1,
                    base - (diameter / 4) + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 4 - 1),
                    center.y - (diameter / 2) + (diameter / 4) - 1,
                    base - 1,
                ),
            }))
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16),
                    center.y - (diameter / 2) - (diameter / 4) - 3,
                    base,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16),
                    center.y - (diameter / 2) - (diameter / 4) + 1,
                    base + 3,
                ),
            })
            .clear();
        // Holding Cell1 windows
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16),
                    center.y - (diameter / 2) - (diameter / 4),
                    base,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16),
                    center.y - (diameter / 2) - (diameter / 4) + 1,
                    base + 3,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 1,
                    center.y - (diameter / 2) - (diameter / 4),
                    base + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 1,
                    center.y - (diameter / 2) - (diameter / 4) + 1,
                    base + 2,
                ),
            })
            .fill(window_ver.clone());
        // chapel main room pulpit stairs1
        painter
            .ramp_inset(
                Aabb {
                    min: Vec3::new(center.x - 8, center.y - (diameter / 4) - 2, base - 3),
                    max: Vec3::new(center.x - 3, center.y - (diameter / 4) + 7, base + 2),
                },
                5,
                Dir::X,
            )
            .fill(white.clone());
        // chapel main room pulpit stairs2
        painter
            .ramp_inset(
                Aabb {
                    min: Vec3::new(center.x + 3, center.y - (diameter / 4) - 2, base - 3),
                    max: Vec3::new(center.x + 8, center.y - (diameter / 4) + 7, base + 2),
                },
                5,
                Dir::NegX,
            )
            .fill(white.clone());
        // chapel main room pulpit stairs2
        painter
            .ramp_inset(
                Aabb {
                    min: Vec3::new(center.x - 8, center.y + (diameter / 4) - 7, base - 3),
                    max: Vec3::new(center.x - 3, center.y + (diameter / 4) + 2, base + 2),
                },
                5,
                Dir::X,
            )
            .fill(white.clone());
        // chapel main room pulpit stairs4
        painter
            .ramp_inset(
                Aabb {
                    min: Vec3::new(center.x + 3, center.y + (diameter / 4) - 7, base - 3),
                    max: Vec3::new(center.x + 8, center.y + (diameter / 4) + 2, base + 2),
                },
                5,
                Dir::NegX,
            )
            .fill(white.clone());
        // Holding Cell1 passage to main room
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16),
                    center.y - (diameter / 3) + 3,
                    base - 4,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16),
                    center.y - (diameter / 20) - 1,
                    base + 3,
                ),
            })
            .fill(white.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 1,
                    center.y - (diameter / 3) + 3,
                    base - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 1,
                    center.y - (diameter / 20) - 1,
                    base,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 1,
                    center.y - (diameter / 3),
                    base - 2,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 1,
                    center.y - (diameter / 20) - 1,
                    base,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 2,
                    center.y - (diameter / 3) + 3,
                    base,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 2,
                    center.y - (diameter / 20) - 1,
                    base + 1,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 1,
                    center.y - (diameter / 3) + 3,
                    base - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 1,
                    center.y - (diameter / 3) + 4,
                    base + 1,
                ),
            })
            .fill(gold_decor.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 2,
                    center.y - (diameter / 3) + 3,
                    base - 2,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 2,
                    center.y - (diameter / 3) + 4,
                    base,
                ),
            })
            .fill(window_ver.clone());
        // holding cell1 coral chest
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x,
                    center.y - (diameter / 2) - (diameter / 4) + 7,
                    base - 1,
                ),
                max: Vec3::new(
                    center.x + 1,
                    center.y - (diameter / 2) - (diameter / 4) + 8,
                    base,
                ),
            })
            .fill(gold_decor.clone());
        painter.rotated_sprite(
            Vec3::new(
                center.x,
                center.y - (diameter / 2) - (diameter / 4) + 7,
                base,
            ),
            SpriteKind::CoralChest,
            0,
        );

        // Holding Cell1 Sea Clerics
        let hc1_sea_clerics_pos = Vec3::new(center.x, center.y - (diameter / 3), base + 3);
        for _ in 0..(3 + ((RandomField::new(0).get((hc1_sea_clerics_pos).with_z(base))) % 3)) {
            painter.spawn(
                EntityInfo::at(hc1_sea_clerics_pos.as_())
                    .with_asset_expect("common.entity.dungeon.sea_chapel.sea_cleric", &mut rng),
            );
        }
        // Holding Cell
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 4),
                    center.y + (diameter / 2) - (diameter / 4),
                    base - (diameter / 4),
                ),
                max: Vec3::new(
                    center.x + (diameter / 4),
                    center.y + (diameter / 2) + (diameter / 4),
                    base - (diameter / 4) + (diameter / 2),
                ),
            })
            .fill(white.clone());
        painter
            .sphere(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 4) + 1,
                    center.y + (diameter / 2) - (diameter / 4) + 1,
                    base - (diameter / 4) + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 4 - 1),
                    center.y + (diameter / 2) + (diameter / 4) - 1,
                    base - (diameter / 4) + (diameter / 2) - 1,
                ),
            })
            .without(painter.cylinder(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 4) + 1,
                    center.y + (diameter / 2) - (diameter / 4) + 1,
                    base - (diameter / 4) + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 4 - 1),
                    center.y + (diameter / 2) + (diameter / 4) - 1,
                    base - 1,
                ),
            }))
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16),
                    center.y + (diameter / 2) + (diameter / 4) - 1,
                    base,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16),
                    center.y + (diameter / 2) + (diameter / 4) + 3,
                    base + 3,
                ),
            })
            .clear();
        // Holding Cell glass barriers
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16),
                    center.y + (diameter / 2) + (diameter / 4) - 2,
                    base + 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16),
                    center.y + (diameter / 2) + (diameter / 4) - 1,
                    base + 2,
                ),
            })
            .fill(glass_barrier.clone());
        // Holding Cell passage to main room
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16),
                    center.y + (diameter / 20) + 1,
                    base - 4,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16),
                    center.y + (diameter / 3) - 3,
                    base + 3,
                ),
            })
            .fill(white.clone());
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 1,
                    center.y + (diameter / 20) + 1,
                    base - 3,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 1,
                    center.y + (diameter / 3) - 3,
                    base,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 1,
                    center.y + (diameter / 20) + 1,
                    base - 2,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 1,
                    center.y + (diameter / 3) - 1,
                    base,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 2,
                    center.y + (diameter / 20) + 1,
                    base,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 2,
                    center.y + (diameter / 3) - 3,
                    base + 1,
                ),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec3::new(
                    center.x - (diameter / 16) + 1,
                    center.y + (diameter / 3) - 4,
                    base - 1,
                ),
                max: Vec3::new(
                    center.x + (diameter / 16) - 1,
                    center.y + (diameter / 3) - 3,
                    base,
                ),
            })
            .fill(glass_barrier.clone());
        // stairway3 tube
        painter
            .cylinder(Aabb {
                min: Vec3::new(center_s3.x - 5, center_s3.y - 5, base + (diameter / 4)),
                max: Vec3::new(
                    center_s3.x + 5,
                    center_s3.y + 5,
                    base - (diameter / 8) + (diameter / 2) + 2,
                ),
            })
            .fill(white.clone());

        painter
            .cylinder(Aabb {
                min: Vec3::new(center_s3.x - 4, center_s3.y - 4, base + 2),
                max: Vec3::new(
                    center_s3.x + 4,
                    center_s3.y + 4,
                    base - (diameter / 8) + (diameter / 2) + 2,
                ),
            })
            .clear();
        // stairway3 tube window1
        painter
            .aabb(Aabb {
                min: Vec3::new(center_s3.x - 1, center_s3.y - 5, base + (diameter / 4) + 1),
                max: Vec3::new(
                    center_s3.x + 1,
                    center_s3.y - 4,
                    base - (diameter / 8) + (diameter / 2) - 2,
                ),
            })
            .fill(window_ver.clone());

        // stairway3 stairs
        let stair_radius3 = 4.5;
        let stairs_clear3 = painter.cylinder(Aabb {
            min: (center_s3 - stair_radius3 as i32).with_z(base - 1),
            max: (center_s3 + stair_radius3 as i32)
                .with_z(base - (diameter / 8) + (diameter / 2) + 2),
        });
        stairs_clear3
            .sample(spiral_staircase(
                center_s3.with_z(base - (diameter / 8) + (diameter / 2) + 2),
                stair_radius3,
                0.5,
                7.0,
            ))
            .fill(white.clone());
        // stairway4
        let center_s4 = Vec2::new(center.x + (diameter / 2) + 2, center.y + (diameter / 8));
        // stairway4 balcony2 entry
        painter
            .cylinder(Aabb {
                min: (center_s4 - 2).with_z(base - (diameter / 8) + (diameter / 2) - 7),
                max: (center_s4 + 3).with_z(base - (diameter / 8) + (diameter / 2) - 5),
            })
            .clear();
        // stairway4 stairs
        let stair_radius4 = 3.0;
        let stairs_clear4 = painter.cylinder(Aabb {
            min: (center_s4 - stair_radius4 as i32).with_z(base - (diameter / 8) - 4),
            max: (center_s4 + stair_radius4 as i32)
                .with_z(base - (diameter / 8) + (diameter / 2) - 4),
        });
        stairs_clear4
            .sample(spiral_staircase(
                center_s4.with_z(base - (diameter / 8) + (diameter / 2) - 4),
                stair_radius4,
                0.5,
                7.0,
            ))
            .fill(white.clone());
        // entry lanterns
        for dir in SQUARE_4 {
            let sq_corner = Vec2::new(center.x - (diameter / 2) + 3, center.y - 5);
            let pos = Vec3::new(
                sq_corner.x + (dir.x * (diameter - 7)),
                sq_corner.y + (dir.y * 9),
                base + 5,
            );
            painter.sprite(pos, SpriteKind::SeashellLantern);
        }
        // main room lanterns
        for dir in SQUARE_4 {
            let sq_corner = Vec2::new(center.x - 4, center.y - (diameter / 2) + 5);
            let pos = Vec3::new(
                sq_corner.x + (dir.y * 7),
                sq_corner.y + (dir.x * (diameter - 10)),
                base + 12,
            );
            painter.sprite(pos, SpriteKind::SeashellLantern);
        }
        // first floor lanterns
        for dir in SQUARE_4 {
            for d in 0..2 {
                let sq_corner = Vec2::new(center.x - 3 - d, center.y - (diameter / 8) - 2 - d);
                let pos = Vec3::new(
                    sq_corner.x + (dir.y * (5 + (2 * d))),
                    sq_corner.y + (dir.x * ((diameter / 4) + 2 + (2 * d))),
                    base - (diameter / 8) + (diameter / 2) + 2,
                );
                painter.sprite(pos, SpriteKind::SeashellLantern);
            }
        }
        // small floor lanterns
        for dir in SQUARE_4 {
            for d in 0..2 {
                let sq_corner = Vec2::new(center.x - 3 - d, center.y - (diameter / 8) - 2 - d);
                let pos = Vec3::new(
                    sq_corner.x + (dir.y * (5 + (2 * d))),
                    sq_corner.y + (dir.x * ((diameter / 4) + 2 + (2 * d))),
                    base + diameter - (diameter / 4) + 1,
                );
                painter.sprite(pos, SpriteKind::SeashellLantern);
            }
        }

        // top floor lanterns
        for dir in SQUARE_4 {
            for d in 0..2 {
                let sq_corner = Vec2::new(center.x - 3 - d, center.y - (diameter / 8) - 2 - d);
                let pos = Vec3::new(
                    sq_corner.x + (dir.y * (5 + (2 * d))),
                    sq_corner.y + (dir.x * ((diameter / 4) + 2 + (2 * d))),
                    base - (diameter / 8) + diameter + 2,
                );
                painter.sprite(pos, SpriteKind::SeashellLantern);
            }
        }
        // main room emblems 1
        for d in 0..2 {
            let emblem_pos = Vec3::new(
                center.x - d,
                center.y + (diameter / 3) + 6 - (d * (2 * (diameter / 3) + 10)),
                base + (diameter / 4) + 1,
            );
            painter.rotated_sprite(emblem_pos, SpriteKind::SeaDecorEmblem, 4 - (4 * d) as u8);
        }
        // main room emblems 2
        for d in 0..2 {
            let emblem_pos = Vec3::new(
                center.x - (diameter / 3) - 7 + (d * (2 * (diameter / 3) + 13)),
                center.y - d,
                base + (diameter / 4) + 1,
            );
            painter.rotated_sprite(emblem_pos, SpriteKind::SeaDecorEmblem, 6 - (4 * d) as u8);
        }
        // first floor emblems / top floor emblems
        for d in 0..2 {
            for e in 0..2 {
                let emblem_pos = Vec3::new(
                    center.x - d,
                    center.y - (diameter / 8) - 4 + (d * ((diameter / 4) + 6)),
                    base + (diameter / 2) + 1 + (e * (diameter / 2)),
                );
                painter.rotated_sprite(emblem_pos, SpriteKind::SeaDecorEmblem, 4 - (4 * d) as u8);
            }
        }

        // side buildings hut, pavillon, tower
        let bldg_corner = center - (diameter / 2);
        for dir in SQUARE_4 {
            let bldg_center = bldg_corner + dir * diameter;
            let bldg_variant = (RandomField::new(0).get((bldg_corner - dir).with_z(base))) % 10;
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
            let bldg_room_windows = painter
                .aabb(Aabb {
                    min: Vec3::new(
                        bldg_center.x - 1,
                        bldg_center.y - (bldg_diameter / 4),
                        base - (bldg_diameter / 15) + (bldg_diameter / 4) - 2,
                    ),
                    max: Vec3::new(
                        bldg_center.x + 1,
                        bldg_center.y + (bldg_diameter / 4),
                        base - (bldg_diameter / 15) + (bldg_diameter / 4) - 1,
                    ),
                })
                .without(painter.aabb(Aabb {
                    min: Vec3::new(
                        bldg_center.x - 1,
                        bldg_center.y - (bldg_diameter / 4) + 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 4) - 2,
                    ),
                    max: Vec3::new(
                        bldg_center.x + 1,
                        bldg_center.y + (bldg_diameter / 4) - 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 4) - 1,
                    ),
                }));
            let bldg_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 4)).with_z(base - (bldg_diameter / 15)),
                    max: (bldg_center + (bldg_diameter / 4))
                        .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
                })
                .without(
                    painter.cylinder(Aabb {
                        min: (bldg_center - (bldg_diameter / 4))
                            .with_z(base - (bldg_diameter / 15)),
                        max: (bldg_center + (bldg_diameter / 4))
                            .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4)),
                    }),
                );
            let bldg_washed_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 4)).with_z(base - (bldg_diameter / 15)),
                    max: (bldg_center + (bldg_diameter / 4) - 1)
                        .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
                })
                .without(
                    painter.cylinder(Aabb {
                        min: (bldg_center - (bldg_diameter / 4))
                            .with_z(base - (bldg_diameter / 15)),
                        max: (bldg_center + (bldg_diameter / 4))
                            .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 4)),
                    }),
                );
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
            let bldg_room2_windows1 = painter
                .aabb(Aabb {
                    min: Vec3::new(
                        bldg_center.x - 1,
                        bldg_center.y - (bldg_diameter / 6),
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                    ),
                    max: Vec3::new(
                        bldg_center.x + 1,
                        bldg_center.y + (bldg_diameter / 6),
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                    ),
                })
                .without(painter.aabb(Aabb {
                    min: Vec3::new(
                        bldg_center.x - 1,
                        bldg_center.y - (bldg_diameter / 6) + 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                    ),
                    max: Vec3::new(
                        bldg_center.x + 1,
                        bldg_center.y + (bldg_diameter / 6) - 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                    ),
                }));
            let bldg_room2_windows2 = painter
                .aabb(Aabb {
                    min: Vec3::new(
                        bldg_center.x - (bldg_diameter / 6),
                        bldg_center.y - 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                    ),
                    max: Vec3::new(
                        bldg_center.x + (bldg_diameter / 6),
                        bldg_center.y + 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                    ),
                })
                .without(painter.aabb(Aabb {
                    min: Vec3::new(
                        bldg_center.x - (bldg_diameter / 6) + 1,
                        bldg_center.y - 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 2,
                    ),
                    max: Vec3::new(
                        bldg_center.x + (bldg_diameter / 6) - 1,
                        bldg_center.y + 1,
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - 1,
                    ),
                }));
            let bldg_room2_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 6) + 1).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6),
                    ),
                    max: (bldg_center + (bldg_diameter / 6)).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                    ),
                })
                .without(
                    painter.cylinder(Aabb {
                        min: (bldg_center - (bldg_diameter / 6)).with_z(
                            base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6),
                        ),
                        max: (bldg_center + (bldg_diameter / 6))
                            .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
                    }),
                );
            let bldg_room2_washed_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 6)).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6),
                    ),
                    max: (bldg_center + (bldg_diameter / 6)).with_z(
                        base - (bldg_diameter / 15) + (bldg_diameter / 2) + (bldg_diameter / 6),
                    ),
                })
                .without(
                    painter.cylinder(Aabb {
                        min: (bldg_center - (bldg_diameter / 6)).with_z(
                            base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 6),
                        ),
                        max: (bldg_center + (bldg_diameter / 6))
                            .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2)),
                    }),
                );
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
                .without(painter.cylinder(Aabb {
                    min: (bldg_center - (bldg_diameter / 7)).with_z(
                        base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2,
                    ),
                    max: (bldg_center + (bldg_diameter / 7)).with_z(
                        base - (bldg_diameter / 15)
                            + tower_height
                            + (bldg_diameter / 4)
                            + (bldg_diameter / 7)
                            - 2,
                    ),
                }));
            let bldg_room3_top = painter
                .sphere(Aabb {
                    min: (bldg_center - (bldg_diameter / 7) + 1).with_z(
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
                .without(painter.cylinder(Aabb {
                    min: (bldg_center - (bldg_diameter / 7)).with_z(
                        base - (bldg_diameter / 15) + tower_height + (bldg_diameter / 4) - 2,
                    ),
                    max: (bldg_center + (bldg_diameter / 7)).with_z(
                        base - (bldg_diameter / 15)
                            + tower_height
                            + (bldg_diameter / 4)
                            + (bldg_diameter / 7)
                            - 2,
                    ),
                }));
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
            let bldg_room3_floor = painter
                .sphere(Aabb {
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
                })
                .without(painter.cylinder(Aabb {
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
                }));
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
                min: bldg_center.with_z(base - (bldg_diameter / 3) + 7),
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
                min: bldg_center.with_z(base - (bldg_diameter / 3) + 7),
                max: (bldg_center + 1)
                    .with_z(base - (bldg_diameter / 15) + (bldg_diameter / 2) - 4),
            };
            let bldg_water_puddle = Aabb {
                min: (bldg_center - 5).with_z(base - (bldg_diameter / 3) + 2),
                max: (bldg_center + 5).with_z(base - (bldg_diameter / 3) + 3),
            };
            let bldg_connect_entry = Aabb {
                min: (bldg_center - 4).with_z(base - (bldg_diameter / 3) + 2),
                max: (bldg_center + 4).with_z(base - (bldg_diameter / 3) + 3),
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
            let bldg_floor2_coral_chest_podium = Aabb {
                min: (bldg_center - 5).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 1,
                ),
                max: (bldg_center - 4).with_z(
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 2,
                ),
            };
            let bldg_floor2_coral_chest_pos = (bldg_center - 5).with_z(
                base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 2,
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
            let bldg_floor2_glass_barriers = Aabb {
                min: Vec3::new(
                    bldg_center.x + 3,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 1,
                ),
                max: Vec3::new(
                    bldg_center.x + 4,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 4,
                ),
            };
            let bldg_floor2_step = Aabb {
                min: Vec3::new(
                    bldg_center.x + 2,
                    bldg_center.y - 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10),
                ),
                max: Vec3::new(
                    bldg_center.x + 3,
                    bldg_center.y + 1,
                    base - (bldg_diameter / 15) + (bldg_diameter / 2) - (bldg_diameter / 10) + 1,
                ),
            };
            let bldg_connect_tube = Aabb {
                min: (bldg_center - (bldg_diameter / 10))
                    .with_z(base - (2 * (bldg_diameter / 3)) + 1),
                max: (bldg_center + (bldg_diameter / 10)).with_z(base - (bldg_diameter / 3) + 1),
            };
            let bldg_connect_water = Aabb {
                min: (bldg_center - (bldg_diameter / 10) + 1)
                    .with_z(base - (2 * (bldg_diameter / 3)) + 1),
                max: (bldg_center + (bldg_diameter / 10) - 1)
                    .with_z(base - (bldg_diameter / 3) + 2),
            };
            let bldg_connect_gate = Aabb {
                min: (bldg_center - 2).with_z(base - (bldg_diameter / 3) + 2),
                max: (bldg_center + 2).with_z(base - (bldg_diameter / 3) + 3),
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
            let bldg_cellar_sea_croc_pos = Vec3::new(
                bldg_center.x - (bldg_diameter / 8),
                bldg_center.y,
                base - (bldg_diameter / 3) + 3,
            );
            for _ in
                0..(1 + ((RandomField::new(0).get((bldg_cellar_sea_croc_pos).with_z(base))) % 2))
            {
                painter.spawn(
                    EntityInfo::at(bldg_cellar_sea_croc_pos.as_())
                        .with_asset_expect("common.entity.wild.aggressive.sea_crocodile", &mut rng),
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
                    bldg_room_windows.fill(window_ver.clone());
                    bldg_top.fill(top.clone());
                    bldg_washed_top.fill(washed.clone());
                    painter.sphere(bldg_room_clear).clear();
                    bldg_room_goldring.fill(gold.clone());
                    bldg_room_goldring_clear.clear();
                    painter.cylinder(bldg_room_floor).fill(white.clone());
                    painter.cylinder(bldg_hut_floors_clear).clear();
                    painter.cylinder(bldg_hut_ropefix1).fill(ropefix1.clone());
                    painter.aabb(bldg_hut_ropefix2).fill(ropefix2.clone());
                    painter.aabb(bldg_hut_rope).fill(rope.clone());
                    painter.cylinder(bldg_water_puddle).fill(water.clone());
                    painter.cylinder(bldg_connect_tube).fill(white.clone());
                    painter.cylinder(bldg_connect_water).fill(water.clone());
                    painter.cylinder(bldg_connect_entry).fill(white.clone());
                    painter.sprite(bldg_room_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_cellar_chest_pos, SpriteKind::DungeonChest1);
                    painter.sprite(bldg_floor_bed_pos, SpriteKind::Bed);
                    painter.sprite(bldg_floor_drawer_pos, SpriteKind::DrawerSmall);
                    painter.sprite(bldg_floor_potion_pos, SpriteKind::PotionMinor);
                    // bldg floor Sea Clerics
                    for _ in 0..(1
                        + ((RandomField::new(0).get((bldg_floor_sea_cleric_pos).with_z(base))) % 2))
                    {
                        painter.spawn(
                            EntityInfo::at(bldg_floor_sea_cleric_pos.as_()).with_asset_expect(
                                "common.entity.dungeon.sea_chapel.sea_cleric",
                                &mut rng,
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
                    bldg_top.fill(top.clone());
                    bldg_washed_top.fill(washed.clone());
                    painter.sphere(bldg_room_clear).clear();
                    bldg_room_goldring.fill(gold.clone());
                    bldg_room_goldring_clear.clear();
                    painter.cylinder(bldg_hut_floors_clear).clear();
                    painter.cylinder(bldg_hut_ropefix1).fill(ropefix1.clone());
                    painter.aabb(bldg_hut_ropefix2).fill(ropefix2.clone());
                    painter.aabb(bldg_hut_rope).fill(rope.clone());
                    painter.cylinder(bldg_water_puddle).fill(water.clone());
                    painter.cylinder(bldg_connect_tube).fill(white.clone());
                    painter.cylinder(bldg_connect_water).fill(water.clone());
                    painter.cylinder(bldg_connect_entry).fill(white.clone());
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
                    bldg_room_windows.fill(window_ver.clone());
                    bldg_top.fill(top.clone());
                    bldg_washed_top.fill(washed.clone());
                    painter.sphere(bldg_room2).fill(white.clone());
                    bldg_room2_windows1.fill(window_ver.clone());
                    bldg_room2_windows2.fill(window_ver2.clone());
                    bldg_room2_washed_top.fill(washed.clone());
                    bldg_room2_top.fill(top.clone());
                    painter.sphere(bldg_room2_clear).clear();
                    bldg_room2_goldring.fill(gold.clone());
                    bldg_room2_goldring_clear.clear();
                    painter.sphere(bldg_room_clear).clear();
                    bldg_room_goldring.fill(gold.clone());
                    bldg_room_goldring_clear.clear();
                    painter.cylinder(bldg_room_floor).fill(white.clone());
                    painter.cylinder(bldg_room2_floor).fill(white.clone());
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
                    painter
                        .aabb(bldg_floor2_glass_barriers)
                        .fill(glass_barrier.clone());
                    painter.cylinder(bldg_tower_floors_clear).clear();
                    painter.aabb(bldg_floor2_step).fill(gold_decor.clone());
                    bldg_room3_floor.fill(white.clone());
                    painter.cylinder(bldg_tower_ropefix1).fill(ropefix1.clone());
                    painter.aabb(bldg_tower_ropefix2).fill(ropefix2.clone());
                    painter.aabb(bldg_tower_rope).fill(rope.clone());
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
                    painter.cylinder(bldg_water_puddle).fill(water.clone());
                    painter.cylinder(bldg_connect_tube).fill(white.clone());
                    painter.cylinder(bldg_connect_water).fill(water.clone());
                    painter.cylinder(bldg_connect_entry).fill(white.clone());
                    painter.sprite(bldg_room_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor2_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor3_lantern_pos, SpriteKind::SeashellLantern);
                    painter.sprite(bldg_floor3_drawer_pos, SpriteKind::DrawerSmall);
                    painter.sprite(bldg_floor3_potion_pos, SpriteKind::PotionMinor);
                    painter.sprite(bldg_cellar_chest_pos, SpriteKind::DungeonChest1);
                    painter
                        .aabb(bldg_floor2_coral_chest_podium)
                        .fill(gold_decor.clone());
                    painter.rotated_sprite(bldg_floor2_coral_chest_pos, SpriteKind::CoralChest, 0);
                    painter.sprite(bldg_floor_bed_pos, SpriteKind::Bed);
                    painter.sprite(bldg_floor_drawer_pos, SpriteKind::DrawerSmall);
                    painter.sprite(bldg_floor_potion_pos, SpriteKind::PotionMinor);
                    // bldg floor Sea Clerics
                    for _ in 0..(1
                        + ((RandomField::new(0).get((bldg_floor_sea_cleric_pos).with_z(base))) % 2))
                    {
                        painter.spawn(
                            EntityInfo::at(bldg_floor_sea_cleric_pos.as_()).with_asset_expect(
                                "common.entity.dungeon.sea_chapel.sea_cleric",
                                &mut rng,
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
            let su_bldg_top = painter
                .sphere(Aabb {
                    min: (su_bldg_center - (su_bldg_diameter / 6))
                        .with_z(su_bldg_base - (su_bldg_diameter / 15)),
                    max: (su_bldg_center + (su_bldg_diameter / 6))
                        .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3)),
                })
                .without(
                    painter.cylinder(Aabb {
                        min: (su_bldg_center - (su_bldg_diameter / 6))
                            .with_z(su_bldg_base - (su_bldg_diameter / 15)),
                        max: (su_bldg_center + (su_bldg_diameter / 6)).with_z(
                            su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6),
                        ),
                    }),
                );
            let su_bldg_washed_top = painter
                .sphere(Aabb {
                    min: (su_bldg_center - (su_bldg_diameter / 6))
                        .with_z(su_bldg_base - (su_bldg_diameter / 15)),
                    max: (su_bldg_center + (su_bldg_diameter / 6) - 1)
                        .with_z(su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 3)),
                })
                .without(
                    painter.cylinder(Aabb {
                        min: (su_bldg_center - (su_bldg_diameter / 6))
                            .with_z(su_bldg_base - (su_bldg_diameter / 15)),
                        max: (su_bldg_center + (su_bldg_diameter / 6)).with_z(
                            su_bldg_base - (su_bldg_diameter / 15) + (su_bldg_diameter / 6),
                        ),
                    }),
                );
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
                min: (su_bldg_center).with_z(su_bldg_base - (su_bldg_diameter / 15) + 5),
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
            match su_bldg_variant {
                0..=5 => {
                    // common parts for small hut / small pavillon,
                    painter.sphere(su_bldg_bottom1).fill(white.clone());
                    painter.sphere(su_bldg_bottom2).fill(white.clone());
                    painter.sphere(su_bldg_room).fill(white.clone());
                    su_bldg_top.fill(top.clone());
                    su_bldg_washed_top.fill(washed.clone());
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
                            painter.sprite(su_bldg_floor_drawer_pos, SpriteKind::DrawerSmall);
                            painter.sprite(su_bldg_floor_potion_pos, SpriteKind::PotionMinor);
                            painter.sprite(su_bldg_floor_bed_pos, SpriteKind::Bed);
                        },
                        _ => {
                            // small pavillon
                            painter.aabb(su_bldg_pavillon_entries1).clear();
                            painter.aabb(su_bldg_pavillon_entries2).clear();
                            painter.aabb(su_bldg_pavillon_entries3).clear();
                            painter.aabb(su_bldg_pavillon_entries4).clear();
                            painter.aabb(su_bldg_pavillon_entries5).clear();
                            painter.aabb(su_bldg_pavillon_entries6).clear();
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
