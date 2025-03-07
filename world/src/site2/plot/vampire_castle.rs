use super::*;
use crate::{
    Land,
    site2::gen::wall_staircase,
    util::{CARDINALS, DIAGONALS, NEIGHBORS, RandomField, sampler::Sampler},
};
use common::{generation::EntityInfo, terrain::SpriteKind};
use rand::prelude::*;
use std::{f32::consts::TAU, sync::Arc};
use vek::*;

pub struct CastleData {
    plot_base: i32,
    center: Vec2<i32>,
    side_bldg_pos_1: Vec2<i32>,
    side_bldg_pos_2: Vec2<i32>,
    side_bldg_positions: Vec<Vec2<i32>>,
    tower_positions: Vec<Vec2<i32>>,
}

pub struct VampireCastle {
    bounds: Aabr<i32>,
    pub(crate) alt: i32,
    pub(crate) castle_data: CastleData,
}
impl VampireCastle {
    pub fn generate(land: &Land, _rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        let center = bounds.center();
        let plot_base = land.get_alt_approx(center) as i32;
        let castle_length = 24;
        let castle_width = 18;
        let tower_base = plot_base + 1;
        let mut tower_positions = vec![];
        let mut side_bldg_positions = vec![];
        let towers = 12.0;
        let tower_radius_raw = 105;
        let tower_phi = TAU / towers;
        let side_bldg_var = (RandomField::new(0).get(center.with_z(plot_base)) % 2) as i32;
        let side_bldg_pos_1 = Vec2::new(
            center.x,
            center.y - (2 * (castle_length + 4)) + side_bldg_var * (4 * (castle_length + 4)),
        );
        let side_bldg_pos_2 = Vec2::new(
            center.x,
            center.y + (2 * (castle_length + 4)) - side_bldg_var * (4 * (castle_length + 4)),
        );
        side_bldg_positions.push(side_bldg_pos_1);
        side_bldg_positions.push(side_bldg_pos_2);

        // castle towers
        for dir in DIAGONALS {
            let tower_pos = Vec2::new(
                center.x + dir.x * (castle_length + 10),
                center.y + dir.y * (castle_width + 10),
            );
            tower_positions.push(tower_pos);
        }
        // outer towers
        for n in 1..=towers as i32 {
            let tower_pos_var = RandomField::new(0).get((center + n).with_z(tower_base)) % 10;
            let tower_radius = tower_radius_raw + tower_pos_var as i32;
            let tower_pos = Vec2::new(
                center.x + (tower_radius as f32 * ((n as f32 * tower_phi).cos())) as i32,
                center.y + (tower_radius as f32 * ((n as f32 * tower_phi).sin())) as i32,
            );

            if RandomField::new(0).get((center + n).with_z(tower_base)) % 8 < 3 {
                tower_positions.push(tower_pos)
            } else {
                side_bldg_positions.push(tower_pos);
            }
        }
        let castle_data = CastleData {
            plot_base,
            center,
            side_bldg_pos_1,
            side_bldg_pos_2,
            side_bldg_positions,
            tower_positions,
        };

        Self {
            bounds,
            alt: land.get_alt_approx(site.tile_center_wpos((tile_aabr.max - tile_aabr.min) / 2))
                as i32,
            castle_data,
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            waypoints: false,
            trees: wpos.distance_squared(self.bounds.center()) > (75_i32).pow(2),
            ..SpawnRules::default()
        }
    }
}

impl Structure for VampireCastle {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_vampire_castle\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "render_vampire_castle"))]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let mut thread_rng = thread_rng();
        let brick = Fill::Brick(BlockKind::Rock, Rgb::new(80, 75, 85), 24);
        let roof_color = Fill::Block(Block::new(BlockKind::GlowingRock, Rgb::new(30, 37, 55)));
        let wood = Fill::Brick(BlockKind::Rock, Rgb::new(71, 33, 11), 12);
        let chain = Fill::Block(Block::air(SpriteKind::MetalChain));
        let window_ver = Fill::Block(Block::air(SpriteKind::WitchWindow));
        let window_ver2 = Fill::Block(Block::air(SpriteKind::WitchWindow));
        let key_door = Fill::Block(Block::air(SpriteKind::VampireKeyDoor));
        let key_hole = Fill::Block(Block::air(SpriteKind::VampireKeyhole));
        let onewaydoor = Fill::Block(Block::air(SpriteKind::OneWayWall).with_ori(2).unwrap());
        let candles = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 4 {
                0 => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
                _ => Block::air(SpriteKind::Candle),
            })
        }));
        let candles_lite = Fill::Sampling(Arc::new(|wpos| {
            Some(match (RandomField::new(0).get(wpos)) % 30 {
                0 => Block::air(SpriteKind::Candle),
                _ => Block::new(BlockKind::Air, Rgb::new(0, 0, 0)),
            })
        }));
        // castle
        let center = self.castle_data.center;
        let plot_base = self.castle_data.plot_base;
        let side_bldg_pos_1 = self.castle_data.side_bldg_pos_1;
        let side_bldg_pos_2 = self.castle_data.side_bldg_pos_2;
        let side_bldg_positions = &self.castle_data.side_bldg_positions;
        let tower_positions = &self.castle_data.tower_positions;

        let castle_base = plot_base + 1;
        let castle_length = 24;
        let castle_width = 18;
        let castle_height = castle_length;
        // entry
        let entry_length = 12;
        let entry_width = 9;
        let entry_height = entry_length;
        let entry_base = castle_base - 2;
        // towers
        let tower_base = plot_base + 1;
        let tower_width = 16;
        let tower_height_raw = 70;
        let roof_width = tower_width + 2;
        let roof_height = 3 * (tower_width / 2);
        let top_height = 20;
        // side buildings
        let side_bldg_length = 12;
        let side_bldg_width = 9;
        let side_bldg_height = 12;
        let side_bldg_roof_height_raw = 32;
        let side_bldg_base_raw = castle_base;
        let side_bldg_var = (RandomField::new(0).get(center.with_z(plot_base)) % 2) as i32;
        let side_bldg_stairs_pos = side_bldg_pos_1;
        let mut bat_positions = vec![];
        let mut harlequin_positions = vec![];
        let mut random_npc_positions = vec![];
        // castle main entry
        let entry_pos = Vec2::new(center.x - castle_length - 20, center.y);
        let entry_pillar_pos = Vec2::new(center.x - castle_length - 24, center.y);
        painter
            .aabb(Aabb {
                min: Vec2::new(
                    entry_pos.x - entry_length - 10,
                    entry_pos.y - entry_width - 10,
                )
                .with_z(entry_base + (entry_height / 2) + (2 * entry_height)),
                max: Vec2::new(
                    entry_pos.x + entry_length + 10,
                    entry_pos.y + entry_width + 10,
                )
                .with_z(entry_base + (entry_height / 2) + (3 * entry_height)),
            })
            .fill(brick.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(
                    entry_pos.x - entry_length - 9,
                    entry_pos.y - entry_width - 9,
                )
                .with_z(entry_base + (entry_height / 2) + (2 * entry_height)),
                max: Vec2::new(
                    entry_pos.x + entry_length + 9,
                    entry_pos.y + entry_width + 9,
                )
                .with_z(entry_base + (entry_height / 2) + (3 * entry_height)),
            })
            .fill(roof_color.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(
                    entry_pos.x - entry_length - 5,
                    entry_pos.y - entry_width - 5,
                )
                .with_z(entry_base + (entry_height / 2) + (2 * entry_height) - 3),
                max: Vec2::new(
                    entry_pos.x + entry_length + 5,
                    entry_pos.y + entry_width + 5,
                )
                .with_z(entry_base + (entry_height / 2) + (2 * entry_height)),
            })
            .fill(brick.clone());
        // entry decor
        let entry_decor_limiter = painter.aabb(Aabb {
            min: Vec2::new(
                entry_pos.x - entry_length - 9,
                entry_pos.y - entry_width - 9,
            )
            .with_z(entry_base + (entry_height / 2) + (2 * entry_height) - 1),
            max: Vec2::new(
                entry_pos.x + entry_length + 9,
                entry_pos.y + entry_width + 9,
            )
            .with_z(entry_base + (entry_height / 2) + (2 * entry_height)),
        });
        let entry_decor_var = RandomField::new(0).get(center.with_z(tower_base)) % 12;
        let entry_decors = 12.0 + entry_decor_var as f32;
        let entry_phi = TAU / entry_decors;
        let entry_decor_radius = entry_length + 10;
        for d in 1..=entry_decors as i32 {
            let entry_decors_pos = Vec2::new(
                entry_pos.x + (entry_decor_radius as f32 * ((d as f32 * entry_phi).cos())) as i32,
                entry_pos.y + (entry_decor_radius as f32 * ((d as f32 * entry_phi).sin())) as i32,
            );
            painter
                .line(
                    entry_pos.with_z(entry_base + (entry_height / 2) + (2 * entry_height) - 1),
                    entry_decors_pos
                        .with_z(entry_base + (entry_height / 2) + (2 * entry_height) - 1),
                    1.0,
                )
                .intersect(entry_decor_limiter)
                .fill(brick.clone());
        }
        // entry roof carve
        for c in 0..2 {
            let w_carve_pos = Vec2::new(
                entry_pos.x,
                entry_pos.y - (2 * entry_width) + (c * (4 * entry_width)),
            );
            let l_carve_pos = Vec2::new(
                entry_pos.x - (4 * (entry_length / 2)) + (c * (8 * (entry_length / 2))),
                entry_pos.y,
            );
            painter
                .superquadric(
                    Aabb {
                        min: Vec2::new(
                            w_carve_pos.x - entry_length - 20,
                            w_carve_pos.y - entry_width - 10,
                        )
                        .with_z(entry_base + (entry_height / 2) + (2 * entry_height)),
                        max: Vec2::new(
                            w_carve_pos.x + entry_length + 20,
                            w_carve_pos.y + entry_width + 10,
                        )
                        .with_z(entry_base + (6 * entry_height)),
                    },
                    2.0,
                )
                .clear();

            painter
                .superquadric(
                    Aabb {
                        min: Vec2::new(
                            l_carve_pos.x - entry_length - 10,
                            l_carve_pos.y - entry_width - 20,
                        )
                        .with_z(entry_base + (entry_height / 2) + (2 * entry_height)),
                        max: Vec2::new(
                            l_carve_pos.x + entry_length + 10,
                            l_carve_pos.y + entry_width + 20,
                        )
                        .with_z(entry_base + (6 * entry_height)),
                    },
                    2.0,
                )
                .clear();
        }
        for p in 0..2 {
            let entry_pyramid_pos = Vec2::new(
                entry_pos.x - (entry_length - 4) + (p * (2 * (entry_length - 4))),
                entry_pos.y,
            );
            painter
                .pyramid(Aabb {
                    min: (entry_pyramid_pos - entry_length)
                        .with_z(entry_base + (entry_height / 2) + (2 * entry_height)),
                    max: (entry_pyramid_pos + entry_length).with_z(
                        entry_base
                            + (entry_height / 2)
                            + (2 * entry_height)
                            + (2 * entry_length)
                            + 1,
                    ),
                })
                .fill(roof_color.clone());
        }

        // entry pillars
        for dir in DIAGONALS {
            let pillar_pos = entry_pillar_pos + dir * (entry_length - 4);
            painter
                .cylinder(Aabb {
                    min: (pillar_pos - 4).with_z(entry_base - 2),
                    max: (pillar_pos + 4)
                        .with_z(entry_base + (entry_height / 2) + (2 * entry_height) - 2),
                })
                .fill(brick.clone());
        }

        // castle roof
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 10, center.y - castle_width - 10)
                    .with_z(castle_base + (castle_height / 2) + (2 * castle_height)),
                max: Vec2::new(center.x + castle_length + 10, center.y + castle_width + 10)
                    .with_z(castle_base + (castle_height / 2) + (3 * castle_height)),
            })
            .fill(brick.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 9, center.y - castle_width - 9)
                    .with_z(castle_base + (castle_height / 2) + (2 * castle_height)),
                max: Vec2::new(center.x + castle_length + 9, center.y + castle_width + 9)
                    .with_z(castle_base + (castle_height / 2) + (3 * castle_height)),
            })
            .fill(roof_color.clone());
        // roof carve
        for c in 0..2 {
            let w_carve_pos = Vec2::new(
                center.x,
                center.y - (2 * castle_width) + (c * (4 * castle_width)),
            );
            let l_carve_pos = Vec2::new(
                center.x - (4 * (castle_length / 2)) + (c * (8 * (castle_length / 2))),
                center.y,
            );
            painter
                .superquadric(
                    Aabb {
                        min: Vec2::new(
                            w_carve_pos.x - castle_length - 40,
                            w_carve_pos.y - castle_width - 20,
                        )
                        .with_z(castle_base + (castle_height / 2) + (2 * castle_height)),
                        max: Vec2::new(
                            w_carve_pos.x + castle_length + 40,
                            w_carve_pos.y + castle_width + 20,
                        )
                        .with_z(castle_base + (6 * castle_height)),
                    },
                    2.0,
                )
                .clear();

            painter
                .superquadric(
                    Aabb {
                        min: Vec2::new(
                            l_carve_pos.x - castle_length - 20,
                            l_carve_pos.y - castle_width - 40,
                        )
                        .with_z(castle_base + (castle_height / 2) + (2 * castle_height)),
                        max: Vec2::new(
                            l_carve_pos.x + castle_length + 20,
                            l_carve_pos.y + castle_width + 40,
                        )
                        .with_z(castle_base + (6 * castle_height)),
                    },
                    2.0,
                )
                .clear();
        }
        // towers
        let tower_access = RandomField::new(0).get(center.with_z(plot_base)) % 4;
        let harlequin_0 = (RandomField::new(0).get(center.with_z(tower_base)) % 4) as usize;
        let harlequin_1 = (RandomField::new(0).get(center.with_z(tower_base + 1)) % 4) as usize;
        for (t, tower_center) in tower_positions.iter().enumerate() {
            let height_var = RandomField::new(0).get(tower_center.with_z(tower_base)) % 30;
            let tower_height = tower_height_raw + height_var as i32;
            let top_base = tower_base + tower_height;
            let cone_height = 20;
            let tower_radius = (tower_width - 6) as f32;
            let access = t == tower_access as usize;
            let tower_top_var = if access {
                0
            } else if t < 4 {
                1
            } else {
                RandomField::new(0).get((tower_center).with_z(tower_base)) % 3
            };
            if t < 4 && access {
                // tower top chain
                painter
                    .line(
                        (tower_center + ((center - tower_center) / 2))
                            .with_z(castle_base + (3 * castle_height) + 5),
                        (tower_center + ((center - tower_center) / 2))
                            .with_z(top_base + top_height - 1),
                        1.0,
                    )
                    .fill(chain.clone());
                painter
                    .line(
                        tower_center.with_z(top_base + top_height),
                        (tower_center + ((center - tower_center) / 2))
                            .with_z(top_base + top_height),
                        1.0,
                    )
                    .fill(wood.clone());
            };
            // tower
            painter
                .cylinder(Aabb {
                    min: (tower_center - tower_width + 6).with_z(tower_base - castle_height - 30),
                    max: (tower_center + 1 + tower_width - 6).with_z(tower_base + tower_height),
                })
                .fill(brick.clone());
            // tower top
            painter
                .cylinder(Aabb {
                    min: (tower_center - tower_width + 1).with_z(top_base),
                    max: (tower_center + 1 + tower_width - 1).with_z(top_base + top_height),
                })
                .fill(brick.clone());

            // carve outs
            let carve_limiter = painter.aabb(Aabb {
                min: (tower_center - (tower_width / 2) - 3).with_z(top_base),
                max: (tower_center + 1 + (tower_width / 2) + 3).with_z(top_base + top_height),
            });
            for dir in CARDINALS {
                let carve_pos = tower_center + dir * (tower_width + (tower_width / 4));
                painter
                    .superquadric(
                        Aabb {
                            min: (carve_pos - tower_width + 3).with_z(top_base + 1),
                            max: (carve_pos + tower_width - 3).with_z(top_base + top_height - 1),
                        },
                        2.5,
                    )
                    .intersect(carve_limiter)
                    .clear()
            }
            // outside decor
            let decor_var = RandomField::new(0).get(tower_center.with_z(tower_base)) % 6;
            let decor_radius = (tower_width / 3) * 2;
            let decors = 4.0 + decor_var as f32;
            let decor_phi = TAU / decors;
            for n in 1..=decors as i32 {
                let pos = Vec2::new(
                    tower_center.x + (decor_radius as f32 * ((n as f32 * decor_phi).cos())) as i32,
                    tower_center.y + (decor_radius as f32 * ((n as f32 * decor_phi).sin())) as i32,
                );
                painter
                    .cubic_bezier(
                        pos.with_z(tower_base + (tower_height / 2) + (tower_height / 6)),
                        (pos - ((tower_center - pos) / 4))
                            .with_z(tower_base + (tower_height / 2) + (tower_height / 3)),
                        (pos - ((tower_center - pos) / 2)).with_z(
                            tower_base
                                + (tower_height / 2)
                                + (tower_height / 3)
                                + (tower_height / 6),
                        ),
                        pos.with_z(top_base + 1),
                        1.5,
                    )
                    .fill(brick.clone());
            }
            // top platform outside low
            painter
                .cylinder(Aabb {
                    min: (tower_center - (3 * (tower_width / 2)) + 11).with_z(top_base - 5),
                    max: (tower_center + 1 + (3 * (tower_width / 2)) - 11).with_z(top_base - 4),
                })
                .fill(brick.clone());
            painter
                .cylinder(Aabb {
                    min: (tower_center - (3 * (tower_width / 2)) + 10).with_z(top_base - 4),
                    max: (tower_center + 1 + (3 * (tower_width / 2)) - 10).with_z(top_base - 2),
                })
                .fill(brick.clone());
            painter
                .cylinder(Aabb {
                    min: (tower_center - (3 * (tower_width / 2)) + 8).with_z(top_base - 2),
                    max: (tower_center + 1 + (3 * (tower_width / 2)) - 8).with_z(top_base),
                })
                .fill(brick.clone());
            painter
                .cylinder(Aabb {
                    min: (tower_center - (3 * (tower_width / 2)) + 6).with_z(top_base),
                    max: (tower_center + 1 + (3 * (tower_width / 2)) - 6).with_z(top_base + 2),
                })
                .fill(brick.clone());
            // top platform outside high
            painter
                .cylinder(Aabb {
                    min: (tower_center - (3 * (tower_width / 2)) + 5).with_z(top_base + top_height),
                    max: (tower_center + 1 + (3 * (tower_width / 2)) - 5)
                        .with_z(top_base + top_height + 2),
                })
                .fill(brick.clone());
            // repaint tower room
            painter
                .cylinder(Aabb {
                    min: (tower_center - tower_width + 2).with_z(top_base),
                    max: (tower_center + 1 + tower_width - 2).with_z(top_base + top_height),
                })
                .fill(brick.clone());
            // top windows
            for h in 0..2 {
                // clear windows
                painter
                    .line(
                        Vec2::new(tower_center.x - tower_width, tower_center.y)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        Vec2::new(tower_center.x + 1 + tower_width, tower_center.y)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        2.5,
                    )
                    .clear();
                painter
                    .line(
                        Vec2::new(tower_center.x, tower_center.y - tower_width)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        Vec2::new(tower_center.x, tower_center.y + 1 + tower_width)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        2.5,
                    )
                    .clear();
            }
            // top window sprites
            let top_window_limiter = painter.aabb(Aabb {
                min: (tower_center - tower_width + 2).with_z(top_base + 1),
                max: (tower_center + 1 + tower_width - 2).with_z(top_base + tower_height - 1),
            });
            for h in 0..2 {
                painter
                    .line(
                        Vec2::new(tower_center.x - tower_width, tower_center.y)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        Vec2::new(tower_center.x + 1 + tower_width, tower_center.y)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        2.5,
                    )
                    .intersect(top_window_limiter)
                    .fill(window_ver2.clone());
                painter
                    .line(
                        Vec2::new(tower_center.x, tower_center.y - tower_width)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        Vec2::new(tower_center.x, tower_center.y + 1 + tower_width)
                            .with_z(top_base + (top_height / 2) - 3 + (4 * h)),
                        2.5,
                    )
                    .intersect(top_window_limiter)
                    .fill(window_ver.clone());
            }
            // tower top window sills
            painter
                .aabb(Aabb {
                    min: Vec2::new(tower_center.x - 1, tower_center.y - tower_width)
                        .with_z(top_base + (top_height / 2) - 5),
                    max: Vec2::new(tower_center.x + 2, tower_center.y + tower_width + 1)
                        .with_z(top_base + (top_height / 2) - 4),
                })
                .fill(wood.clone());

            painter
                .aabb(Aabb {
                    min: Vec2::new(tower_center.x - tower_width, tower_center.y - 1)
                        .with_z(top_base + (top_height / 2) - 5),
                    max: Vec2::new(tower_center.x + tower_width + 1, tower_center.y + 2)
                        .with_z(top_base + (top_height / 2) - 4),
                })
                .fill(wood.clone());
            // clear top room
            painter
                .cylinder(Aabb {
                    min: (tower_center - tower_width + 3).with_z(top_base + 1),
                    max: (tower_center + 1 + tower_width - 3).with_z(top_base + top_height),
                })
                .clear();
            // top room candles and wood ring
            painter
                .cylinder(Aabb {
                    min: (tower_center - tower_width + 3).with_z(top_base + 1),
                    max: (tower_center + 1 + tower_width - 3).with_z(top_base + 2),
                })
                .fill(candles_lite.clone());
            painter
                .cylinder(Aabb {
                    min: (tower_center - tower_width + 3).with_z(top_base + top_height - 1),
                    max: (tower_center + 1 + tower_width - 3).with_z(top_base + top_height),
                })
                .fill(wood.clone());
            painter
                .cylinder(Aabb {
                    min: (tower_center - tower_width + 8).with_z(top_base + 1),
                    max: (tower_center + 1 + tower_width - 8).with_z(top_base + top_height),
                })
                .clear();
            // tower windows and decor
            let tower_window_limiter = painter.aabb(Aabb {
                min: (tower_center + 1 - tower_radius as i32).with_z(tower_base),
                max: (tower_center + tower_radius as i32).with_z(tower_base + tower_height + 1),
            });
            let decor_var = RandomField::new(0).get(tower_center.with_z(tower_base)) % 10;
            let decors = 10.0 + decor_var as f32;
            let tower_decor_phi = TAU / decors;
            let decor_radius = (tower_width / 2) + 4;
            for h in 0..2 {
                for n in 0..2 {
                    // tower decor
                    for d in 1..=decors as i32 {
                        let decor_pos = Vec2::new(
                            tower_center.x
                                + (decor_radius as f32 * ((d as f32 * tower_decor_phi).cos()))
                                    as i32,
                            tower_center.y
                                + (decor_radius as f32 * ((d as f32 * tower_decor_phi).sin()))
                                    as i32,
                        );
                        painter
                            .line(
                                tower_center.with_z(
                                    tower_base + (tower_height / 3) + ((tower_height / 3) * n),
                                ),
                                decor_pos.with_z(
                                    tower_base + (tower_height / 3) + ((tower_height / 3) * n),
                                ),
                                1.0,
                            )
                            .fill(brick.clone());
                    }
                    // clear windows
                    painter
                        .line(
                            Vec2::new(tower_center.x - tower_width + 5, tower_center.y).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            Vec2::new(tower_center.x + 1 + tower_width - 5, tower_center.y).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            2.5,
                        )
                        .clear();
                    painter
                        .line(
                            Vec2::new(tower_center.x, tower_center.y - tower_width + 5).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            Vec2::new(tower_center.x, tower_center.y + 1 + tower_width - 5).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            2.5,
                        )
                        .clear();
                    // tower window sprites
                    painter
                        .line(
                            Vec2::new(tower_center.x - tower_width + 4, tower_center.y).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            Vec2::new(tower_center.x + 1 + tower_width - 4, tower_center.y).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            2.5,
                        )
                        .intersect(tower_window_limiter)
                        .fill(window_ver2.clone());
                    painter
                        .line(
                            Vec2::new(tower_center.x, tower_center.y - tower_width + 4).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            Vec2::new(tower_center.x, tower_center.y + 1 + tower_width - 4).with_z(
                                tower_base
                                    + (tower_height / 4)
                                    + (4 * h)
                                    + ((tower_height / 4) * n),
                            ),
                            2.5,
                        )
                        .intersect(tower_window_limiter)
                        .fill(window_ver.clone());
                    // tower window sills
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(
                                tower_center.x - 1,
                                tower_center.y - (tower_width / 2) - 3,
                            )
                            .with_z(tower_base + (tower_height / 4) - 3 + ((tower_height / 4) * n)),
                            max: Vec2::new(
                                tower_center.x + 2,
                                tower_center.y + (tower_width / 2) + 4,
                            )
                            .with_z(tower_base + (tower_height / 4) - 2 + ((tower_height / 4) * n)),
                        })
                        .fill(wood.clone());

                    painter
                        .aabb(Aabb {
                            min: Vec2::new(
                                tower_center.x - (tower_width / 2) - 3,
                                tower_center.y - 1,
                            )
                            .with_z(tower_base + (tower_height / 4) - 3 + ((tower_height / 4) * n)),
                            max: Vec2::new(
                                tower_center.x + (tower_width / 2) + 4,
                                tower_center.y + 2,
                            )
                            .with_z(tower_base + (tower_height / 4) - 2 + ((tower_height / 4) * n)),
                        })
                        .fill(wood.clone());
                }
            }
            // roof
            painter
                .cylinder(Aabb {
                    min: (tower_center - roof_width).with_z(top_base + top_height),
                    max: (tower_center + 1 + roof_width)
                        .with_z(top_base + top_height + roof_height),
                })
                .fill(brick.clone());
            let tower_roof_filling = painter.cylinder(Aabb {
                min: (tower_center - roof_width + 1).with_z(top_base + top_height),
                max: (tower_center + 1 + roof_width - 1)
                    .with_z(top_base + top_height + roof_height),
            });
            if tower_top_var > 0 {
                tower_roof_filling.fill(roof_color.clone());
            } else {
                tower_roof_filling.clear();
            }
            // roof carve outs
            for dir in NEIGHBORS {
                let carve_pos = tower_center + dir * roof_width;
                let carve_z_offset = roof_height / 20;

                painter
                    .superquadric(
                        Aabb {
                            min: (carve_pos - 3 * (tower_width / 4))
                                .with_z(top_base + top_height + carve_z_offset),
                            max: (carve_pos + 3 * (tower_width / 4))
                                .with_z(top_base + top_height + (2 * roof_height) + carve_z_offset),
                        },
                        2.5,
                    )
                    .clear();
            }
            if tower_top_var > 0 {
                // cones
                painter
                    .cone(Aabb {
                        min: (tower_center - (roof_width / 3) - 2).with_z(top_base + top_height),
                        max: (tower_center + (roof_width / 3) + 2)
                            .with_z(top_base + cone_height + (4 * roof_height) - 4),
                    })
                    .fill(roof_color.clone());

                for dir in DIAGONALS {
                    let cone_center = tower_center + dir * 6;
                    let cone_var = RandomField::new(0).get(cone_center.with_z(tower_base)) % 60;
                    painter
                        .cone(Aabb {
                            min: (cone_center - 4).with_z(top_base + top_height + 2),
                            max: (cone_center + 4)
                                .with_z(top_base + top_height + 30 + cone_var as i32),
                        })
                        .fill(roof_color.clone());
                }
            } else {
                // top platform variant
                painter
                    .cylinder(Aabb {
                        min: (tower_center - (3 * (tower_width / 2)) + 6)
                            .with_z(top_base + top_height),
                        max: (tower_center + 1 + (3 * (tower_width / 2)) - 6)
                            .with_z(top_base + top_height + 2),
                    })
                    .fill(brick.clone());
            }

            // tower clear
            let tower_radius = (tower_width - 7) as f32;
            let add_on = if tower_top_var > 0 { 0 } else { top_height + 1 };
            painter
                .cylinder(Aabb {
                    min: (tower_center + 1 - tower_radius as i32)
                        .with_z(tower_base - castle_height + 1),
                    max: (tower_center + tower_radius as i32)
                        .with_z(tower_base + tower_height + 1 + add_on),
                })
                .clear();
            // entries and floor for far towers
            if t > 3 {
                for dir in DIAGONALS {
                    let entry_pos = tower_center + (dir * ((tower_width / 3) + 3));
                    painter
                        .line(
                            tower_center.with_z(tower_base + (top_height / 3) + 5),
                            entry_pos.with_z(tower_base + (top_height / 3)),
                            4.5,
                        )
                        .clear();
                    painter
                        .cylinder(Aabb {
                            min: (tower_center - tower_width + 7)
                                .with_z(tower_base - castle_height + 1),
                            max: (tower_center + 1 + tower_width - 7)
                                .with_z(tower_base + (top_height / 3)),
                        })
                        .fill(brick.clone());
                }
                // floor candles
                painter
                    .cylinder(Aabb {
                        min: (tower_center - tower_width + 8).with_z(tower_base + (top_height / 3)),
                        max: (tower_center + 1 + tower_width - 8)
                            .with_z(tower_base + (top_height / 3) + 1),
                    })
                    .fill(candles_lite.clone());
            }
            if tower_top_var < 1 {
                // center clear
                let ground_floor = if t < 4 {
                    -castle_height - 1
                } else {
                    top_height / 3
                };
                painter
                    .cylinder(Aabb {
                        min: (tower_center - 2).with_z(tower_base + ground_floor),
                        max: (tower_center + 3).with_z(tower_base + tower_height + 1 + add_on),
                    })
                    .clear();
                if t < 4 && access {
                    // tower top entry door
                    painter
                        .cylinder(Aabb {
                            min: (tower_center + 1 - tower_radius as i32)
                                .with_z(tower_base + tower_height + add_on),
                            max: (tower_center + tower_radius as i32)
                                .with_z(tower_base + tower_height + 1 + add_on),
                        })
                        .fill(key_door.clone());
                    painter
                        .cylinder(Aabb {
                            min: (tower_center).with_z(tower_base + tower_height + add_on),
                            max: (tower_center + 1).with_z(tower_base + tower_height + 1 + add_on),
                        })
                        .fill(key_hole.clone());
                    painter
                        .cylinder(Aabb {
                            min: (tower_center + 1 - tower_radius as i32)
                                .with_z(tower_base + tower_height + 1 + add_on),
                            max: (tower_center + tower_radius as i32)
                                .with_z(tower_base + tower_height + 2 + add_on),
                        })
                        .fill(onewaydoor.clone());
                    painter
                        .cylinder(Aabb {
                            min: (tower_center + 2 - tower_radius as i32)
                                .with_z(tower_base + tower_height + 1 + add_on),
                            max: (tower_center - 1 + tower_radius as i32)
                                .with_z(tower_base + tower_height + 2 + add_on),
                        })
                        .clear();
                    let top_bat_pos = tower_center.with_z(tower_base + tower_height + 2 + add_on);
                    bat_positions.push(top_bat_pos);
                }
            }
            let bat_z = if t > 3 {
                tower_base + (top_height / 3)
            } else {
                castle_base - castle_height + 1
            };
            let bat_pos_bottom = tower_center.with_z(bat_z);
            bat_positions.push(bat_pos_bottom);
            // tower platforms, chests and harlequins
            for p in 0..4 {
                painter
                    .cylinder(Aabb {
                        min: (tower_center - (tower_width / 4) - 3)
                            .with_z(tower_base + (p * (tower_height / 3))),
                        max: (tower_center - (tower_width / 4) + 3)
                            .with_z(tower_base + (p * (tower_height / 3)) + 1),
                    })
                    .fill(roof_color.clone());
                painter.sprite(
                    (tower_center - (tower_width / 4) + 1)
                        .with_z(tower_base + 1 + (p * (tower_height / 3))),
                    SpriteKind::Candle,
                );
            }
            let chest_pos =
                (tower_center + (tower_width / 2)).with_z(tower_base + tower_height + 1);
            let rand_npc_pos =
                (tower_center + (tower_width / 2) + 1).with_z(tower_base + tower_height + 1);
            let chest_var = RandomField::new(0).get(chest_pos) % 2;
            if chest_var > 0 {
                painter.sprite(chest_pos, SpriteKind::DungeonChest3);
                random_npc_positions.push(rand_npc_pos);
            }
            if t == harlequin_0 {
                let harlequin_pos_0 =
                    (tower_center - (tower_width / 2)).with_z(tower_base + tower_height + 1);
                harlequin_positions.push(harlequin_pos_0);
            }
            if t == harlequin_1 {
                let harlequin_pos_1 =
                    (tower_center - (tower_width / 2) + 1).with_z(tower_base + tower_height + 1);
                harlequin_positions.push(harlequin_pos_1);
            }
        }
        let mut harlequin_2_positions = vec![];
        for pos in side_bldg_positions.iter().skip(2) {
            harlequin_2_positions.push(*pos)
        }
        for pos in tower_positions.iter().skip(4) {
            harlequin_2_positions.push(*pos)
        }
        let harlequin_2 = (RandomField::new(0).get(center.with_z(tower_base))
            % harlequin_2_positions.len() as u32) as usize;
        let harlequin_pos_2 = harlequin_2_positions[harlequin_2].with_z(tower_base + 8);
        harlequin_positions.push(harlequin_pos_2);
        // castle base
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 3, center.y - castle_width - 3)
                    .with_z(castle_base),
                max: Vec2::new(center.x + castle_length + 3, center.y + castle_width + 3)
                    .with_z(castle_base + (castle_height / 2)),
            })
            .fill(brick.clone());
        // castle room
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 5, center.y - castle_width - 5)
                    .with_z(castle_base + (castle_height / 2)),
                max: Vec2::new(center.x + castle_length + 5, center.y + castle_width + 5)
                    .with_z(castle_base + (castle_height / 2) + (2 * castle_height)),
            })
            .fill(brick.clone());
        // castle decor and windows
        let castle_limiter = painter.aabb(Aabb {
            min: Vec2::new(center.x - castle_length - 6, center.y - castle_width - 6)
                .with_z(castle_base),
            max: Vec2::new(center.x + castle_length + 6, center.y + castle_width + 6)
                .with_z(castle_base + (castle_height / 2) + (2 * castle_height)),
        });
        let castle_window_limiter = painter.aabb(Aabb {
            min: Vec2::new(center.x - castle_length - 4, center.y - castle_width - 4)
                .with_z(castle_base + (castle_height / 2) + 1),
            max: Vec2::new(center.x + castle_length + 4, center.y + castle_width + 4)
                .with_z(castle_base + (castle_height / 2) + (2 * castle_height) - 1),
        });
        // castle decor
        let castle_decor_var = RandomField::new(0).get(center.with_z(tower_base)) % 25;
        let castle_decors = 25.0 + castle_decor_var as f32;
        let castle_decor_phi = TAU / castle_decors;
        let castle_decor_radius = castle_length + 10;
        // tower decor
        for d in 1..=castle_decors as i32 {
            let castle_decor_pos = Vec2::new(
                center.x
                    + (castle_decor_radius as f32 * ((d as f32 * castle_decor_phi).cos())) as i32,
                center.y
                    + (castle_decor_radius as f32 * ((d as f32 * castle_decor_phi).sin())) as i32,
            );
            for l in 0..2 {
                painter
                    .line(
                        center.with_z(
                            castle_base + (castle_height / 2) + ((2 * l) * castle_height) - 1,
                        ),
                        castle_decor_pos.with_z(
                            castle_base + (castle_height / 2) + ((2 * l) * castle_height) - 1,
                        ),
                        1.0,
                    )
                    .intersect(castle_limiter)
                    .fill(brick.clone());
                painter
                    .line(
                        center.with_z(
                            castle_base + 3 + castle_height + (l * ((castle_height / 2) + 5)),
                        ),
                        castle_decor_pos.with_z(
                            castle_base + 3 + castle_height + (l * ((castle_height / 2) + 5)),
                        ),
                        1.0,
                    )
                    .intersect(castle_limiter)
                    .fill(brick.clone());
            }
        }
        // castle windows
        for h in 0..2 {
            for s in 1..=2 {
                for r in 0..=2 {
                    // clear windows
                    painter
                        .line(
                            Vec2::new(
                                center.x - castle_length - 5,
                                center.y - (castle_width / 2) + (r * (castle_width / 2)),
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            Vec2::new(
                                center.x + 1 + castle_length + 5,
                                center.y - (castle_width / 2) + (r * (castle_width / 2)),
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            2.5,
                        )
                        .clear();
                    painter
                        .line(
                            Vec2::new(
                                center.x - (castle_length / 2) + (r * (castle_length / 2)),
                                center.y - castle_width - 5,
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            Vec2::new(
                                center.x - (castle_length / 2) + (r * (castle_length / 2)),
                                center.y + 1 + castle_width + 5,
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            2.5,
                        )
                        .clear();

                    // castle window sills
                    painter
                        .aabb(Aabb {
                            min: Vec2::new(
                                center.x - 1 - (castle_length / 2) + (r * (castle_length / 2)),
                                center.y - castle_width - 6,
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 2)) + (3 * (s - 1))),
                            max: Vec2::new(
                                center.x + 2 - (castle_length / 2) + (r * (castle_length / 2)),
                                center.y + castle_width + 6,
                            )
                            .with_z(
                                castle_base + 1 + (s * ((castle_height / 2) + 2)) + (3 * (s - 1)),
                            ),
                        })
                        .fill(wood.clone());

                    painter
                        .aabb(Aabb {
                            min: Vec2::new(
                                center.x - castle_length - 6,
                                center.y - 1 - (castle_width / 2) + (r * (castle_width / 2)),
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 2)) + (3 * (s - 1))),
                            max: Vec2::new(
                                center.x + castle_length + 6,
                                center.y + 2 - (castle_width / 2) + (r * (castle_width / 2)),
                            )
                            .with_z(
                                castle_base + 1 + (s * ((castle_height / 2) + 2)) + (3 * (s - 1)),
                            ),
                        })
                        .fill(wood.clone());
                    // castle window sprites
                    painter
                        .line(
                            Vec2::new(
                                center.x - castle_length - 5,
                                center.y - (castle_width / 2) + (r * (castle_width / 2)),
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            Vec2::new(
                                center.x + 1 + castle_length + 5,
                                center.y - (castle_width / 2) + (r * (castle_width / 2)),
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            2.5,
                        )
                        .intersect(castle_window_limiter)
                        .fill(window_ver2.clone());
                    painter
                        .line(
                            Vec2::new(
                                center.x - (castle_length / 2) + (r * (castle_length / 2)),
                                center.y - castle_width - 5,
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            Vec2::new(
                                center.x - (castle_length / 2) + (r * (castle_length / 2)),
                                center.y + 1 + castle_width + 5,
                            )
                            .with_z(castle_base + (s * ((castle_height / 2) + 5)) + (4 * h)),
                            2.5,
                        )
                        .intersect(castle_window_limiter)
                        .fill(window_ver.clone());
                }
            }
        }
        // main entry stairs
        painter
            .line(
                entry_pos.with_z(castle_base - 2),
                center.with_z(castle_base + castle_height - 5),
                5.0,
            )
            .fill(brick.clone());
        painter
            .line(
                entry_pos.with_z(castle_base),
                center.with_z(castle_base + castle_height - 3),
                5.0,
            )
            .clear();
        painter
            .aabb(Aabb {
                min: (entry_pos - 10).with_z(castle_base - 5),
                max: (entry_pos + 10).with_z(castle_base),
            })
            .fill(brick.clone());
        // main entry door
        painter
            .horizontal_cylinder(
                Aabb {
                    min: Vec2::new(center.x - castle_length - 3, center.y - 4)
                        .with_z(entry_base + 6),
                    max: Vec2::new(center.x - castle_length - 2, center.y + 5)
                        .with_z(entry_base + 15),
                },
                Dir::NegX,
            )
            .fill(onewaydoor.clone());
        // castle cellar
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 2, center.y - castle_width - 2)
                    .with_z(castle_base - castle_height),
                max: Vec2::new(center.x + castle_length + 2, center.y + castle_width + 2)
                    .with_z(castle_base + (castle_height / 8)),
            })
            .fill(brick.clone());
        // cellar stairs to side_bldgs
        painter
            .line(
                center.with_z(castle_base - castle_height - 3),
                side_bldg_stairs_pos.with_z(side_bldg_base_raw),
                6.0,
            )
            .fill(brick.clone());
        painter
            .line(
                center.with_z(castle_base - castle_height - 3),
                side_bldg_stairs_pos.with_z(side_bldg_base_raw),
                5.0,
            )
            .clear();
        // clear castle cellar
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 1, center.y - castle_width - 1)
                    .with_z(castle_base - castle_height + 1),
                max: Vec2::new(center.x + castle_length + 1, center.y + castle_width + 1)
                    .with_z(castle_base + (castle_height / 8) - 1),
            })
            .clear();

        random_npc_positions.push(center.with_z(castle_base - castle_height + 1));

        // castle cellar tower entries
        for dir in DIAGONALS {
            let entry_pos = Vec2::new(
                center.x + dir.x * (castle_length + 2),
                center.y + dir.y * (castle_width + 2),
            );
            painter
                .line(
                    entry_pos.with_z(castle_base - castle_height),
                    entry_pos.with_z(castle_base - castle_height + 6),
                    8.0,
                )
                .clear();
            painter
                .cylinder(Aabb {
                    min: (entry_pos - 8).with_z(castle_base - castle_height - 5),
                    max: (entry_pos + 8).with_z(castle_base - castle_height + 1),
                })
                .fill(brick.clone());
        }
        // castle cellar floor
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 20, center.y - castle_width - 20)
                    .with_z(castle_base - castle_height - 8),
                max: Vec2::new(center.x + castle_length + 20, center.y + castle_width + 20)
                    .with_z(castle_base - castle_height + 1),
            })
            .fill(brick.clone());
        // cellar wood decor and candles
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 1, center.y - castle_width - 1)
                    .with_z(castle_base + (castle_height / 8) - 6),
                max: Vec2::new(center.x + castle_length + 1, center.y + castle_width + 1)
                    .with_z(castle_base + (castle_height / 8) - 1),
            })
            .fill(wood.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length + 1, center.y - castle_width + 1)
                    .with_z(castle_base + (castle_height / 8) - 6),
                max: Vec2::new(center.x + castle_length - 1, center.y + castle_width - 1)
                    .with_z(castle_base + (castle_height / 8) - 1),
            })
            .clear();
        let cellar_podium_limiter = painter.aabb(Aabb {
            min: Vec2::new(center.x - castle_length - 1, center.y - castle_width - 1)
                .with_z(castle_base - castle_height + 1),
            max: Vec2::new(center.x + castle_length + 1, center.y + castle_width + 1)
                .with_z(castle_base - castle_height + 4),
        });
        let mut cellar_beam_postions = vec![];
        for dir in DIAGONALS {
            let beam_pos = Vec2::new(
                center.x + (dir.x * (castle_length / 3)),
                center.y + (dir.y * (castle_width + 1)),
            );
            cellar_beam_postions.push(beam_pos);
        }
        for b in 0..2 {
            let beam_pos = Vec2::new(
                center.x - castle_length - 2 + (b * ((2 * castle_length) + 3)),
                center.y,
            );
            cellar_beam_postions.push(beam_pos);
        }
        for beam_pos in cellar_beam_postions {
            for n in 0..3 {
                painter
                    .cylinder_with_radius(
                        beam_pos.with_z(castle_base - castle_height + 1 + n),
                        (5 - n) as f32,
                        1.0,
                    )
                    .intersect(cellar_podium_limiter)
                    .fill(match n {
                        2 => candles.clone(),
                        _ => wood.clone(),
                    });
            }
            painter
                .line(
                    beam_pos.with_z(castle_base - castle_height + 1),
                    beam_pos.with_z(castle_base + (castle_height / 8) - 5),
                    2.0,
                )
                .fill(wood.clone());
        }
        // side buildings
        for (b, side_bldg_pos) in side_bldg_positions.iter().enumerate() {
            let side_bldg_roof_height = side_bldg_roof_height_raw
                + (RandomField::new(0).get(side_bldg_pos.with_z(tower_base)) % 12) as i32;
            let side_bldg_base = if b > 1 {
                side_bldg_base_raw - (side_bldg_height / 2) - side_bldg_roof_height
            } else {
                side_bldg_base_raw
            };
            // side_bldg roof
            let side_bldg_roof = painter.aabb(Aabb {
                min: Vec2::new(
                    side_bldg_pos.x - side_bldg_length - 10,
                    side_bldg_pos.y - side_bldg_width - 10,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height)),
                max: Vec2::new(
                    side_bldg_pos.x + side_bldg_length + 10,
                    side_bldg_pos.y + side_bldg_width + 10,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (3 * side_bldg_height)),
            });
            side_bldg_roof.fill(brick.clone());
            painter
                .aabb(Aabb {
                    min: Vec2::new(
                        side_bldg_pos.x - side_bldg_length - 9,
                        side_bldg_pos.y - side_bldg_width - 9,
                    )
                    .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height)),
                    max: Vec2::new(
                        side_bldg_pos.x + side_bldg_length + 9,
                        side_bldg_pos.y + side_bldg_width + 9,
                    )
                    .with_z(side_bldg_base + side_bldg_roof_height + (3 * side_bldg_height)),
                })
                .fill(roof_color.clone());
            // side_bldg room
            painter
                .aabb(Aabb {
                    min: Vec2::new(
                        side_bldg_pos.x - side_bldg_length - 5,
                        side_bldg_pos.y - side_bldg_width - 5,
                    )
                    .with_z(side_bldg_base - 2),
                    max: Vec2::new(
                        side_bldg_pos.x + side_bldg_length + 5,
                        side_bldg_pos.y + side_bldg_width + 5,
                    )
                    .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height)),
                })
                .fill(brick.clone());
            // side_bldg roof carve
            for c in 0..2 {
                let w_carve_pos = Vec2::new(
                    side_bldg_pos.x,
                    side_bldg_pos.y - (2 * side_bldg_width) + (c * (4 * side_bldg_width)),
                );
                let l_carve_pos = Vec2::new(
                    side_bldg_pos.x - (4 * (side_bldg_length / 2))
                        + (c * (8 * (side_bldg_length / 2))),
                    side_bldg_pos.y,
                );
                painter
                    .superquadric(
                        Aabb {
                            min: Vec2::new(
                                w_carve_pos.x - side_bldg_length - 20,
                                w_carve_pos.y - side_bldg_width - 10,
                            )
                            .with_z(
                                side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height),
                            ),
                            max: Vec2::new(
                                w_carve_pos.x + side_bldg_length + 20,
                                w_carve_pos.y + side_bldg_width + 10,
                            )
                            .with_z(
                                side_bldg_base + side_bldg_roof_height - (side_bldg_height / 2)
                                    + (6 * side_bldg_height),
                            ),
                        },
                        2.0,
                    )
                    .intersect(side_bldg_roof)
                    .clear();

                painter
                    .superquadric(
                        Aabb {
                            min: Vec2::new(
                                l_carve_pos.x - side_bldg_length - 10,
                                l_carve_pos.y - side_bldg_width - 20,
                            )
                            .with_z(
                                side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height),
                            ),
                            max: Vec2::new(
                                l_carve_pos.x + side_bldg_length + 10,
                                l_carve_pos.y + side_bldg_width + 20,
                            )
                            .with_z(
                                side_bldg_base + side_bldg_roof_height - (side_bldg_height / 2)
                                    + (6 * side_bldg_height),
                            ),
                        },
                        2.0,
                    )
                    .intersect(side_bldg_roof)
                    .clear();
            }
            for p in 0..2 {
                let pyramid_pos = Vec2::new(
                    side_bldg_pos.x - (side_bldg_length - 4) + (p * (2 * (side_bldg_length - 4))),
                    side_bldg_pos.y,
                );
                painter
                    .pyramid(Aabb {
                        min: (pyramid_pos - side_bldg_length).with_z(
                            side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height),
                        ),
                        max: (pyramid_pos + side_bldg_length).with_z(
                            side_bldg_base
                                + side_bldg_roof_height
                                + (2 * side_bldg_height)
                                + (2 * side_bldg_length)
                                + 1,
                        ),
                    })
                    .fill(roof_color.clone());
            }

            // side_bldg decor
            let side_bldg_decor_limiter_1 = painter.aabb(Aabb {
                min: Vec2::new(
                    side_bldg_pos.x - side_bldg_length - 9,
                    side_bldg_pos.y - side_bldg_width - 9,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 1),
                max: Vec2::new(
                    side_bldg_pos.x + side_bldg_length + 9,
                    side_bldg_pos.y + side_bldg_width + 9,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height)),
            });
            let side_bldg_decor_limiter_2 = painter.aabb(Aabb {
                min: Vec2::new(
                    side_bldg_pos.x - side_bldg_length - 6,
                    side_bldg_pos.y - side_bldg_width - 6,
                )
                .with_z(side_bldg_base + 8),
                max: Vec2::new(
                    side_bldg_pos.x + side_bldg_length + 6,
                    side_bldg_pos.y + side_bldg_width + 6,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 2),
            });
            let side_bldg_window_limiter = painter.aabb(Aabb {
                min: Vec2::new(
                    side_bldg_pos.x - side_bldg_length - 4,
                    side_bldg_pos.y - side_bldg_width - 4,
                )
                .with_z(side_bldg_base),
                max: Vec2::new(
                    side_bldg_pos.x + side_bldg_length + 4,
                    side_bldg_pos.y + side_bldg_width + 4,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 2),
            });
            let side_bldg_decor_rows = (side_bldg_roof_height + (2 * side_bldg_height)) / 6;
            for r in 0..=6 {
                let side_bldg_decor_limiter = if r == 0 {
                    side_bldg_decor_limiter_1
                } else {
                    side_bldg_decor_limiter_2
                };
                let side_bldg_decor_var =
                    RandomField::new(0).get((side_bldg_pos + r).with_z(tower_base)) % 12;
                let side_bldg_decors = 12.0 + side_bldg_decor_var as f32;
                let side_bldg_phi = TAU / side_bldg_decors;
                let side_bldg_decor_radius = side_bldg_length + 10;
                for d in 1..=side_bldg_decors as i32 {
                    let side_bldg_decors_pos = Vec2::new(
                        side_bldg_pos.x
                            + (side_bldg_decor_radius as f32 * ((d as f32 * side_bldg_phi).cos()))
                                as i32,
                        side_bldg_pos.y
                            + (side_bldg_decor_radius as f32 * ((d as f32 * side_bldg_phi).sin()))
                                as i32,
                    );
                    painter
                        .line(
                            side_bldg_pos.with_z(
                                side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height)
                                    - 1
                                    - (r * side_bldg_decor_rows),
                            ),
                            side_bldg_decors_pos.with_z(
                                side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height)
                                    - 1
                                    - (r * side_bldg_decor_rows),
                            ),
                            1.0,
                        )
                        .intersect(side_bldg_decor_limiter)
                        .fill(brick.clone());
                }
            }
            // side bldg close, windows, gangway to main room
            if b < 2 {
                // side bldg windows
                for r in 0..2 {
                    for t in 0..=2 {
                        for h in 0..2 {
                            // clear windows
                            painter
                                .line(
                                    Vec2::new(
                                        side_bldg_pos.x - side_bldg_length - 7,
                                        side_bldg_pos.y - 1 - (side_bldg_width / 2)
                                            + (r * ((side_bldg_length) - 2)),
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    Vec2::new(
                                        side_bldg_pos.x + 1 + side_bldg_length + 7,
                                        side_bldg_pos.y - 1 - (side_bldg_width / 2)
                                            + (r * ((side_bldg_length) - 2)),
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    2.5,
                                )
                                .clear();
                            painter
                                .line(
                                    Vec2::new(
                                        side_bldg_pos.x - (side_bldg_length / 2)
                                            + (r * side_bldg_width),
                                        side_bldg_pos.y - side_bldg_width - 7,
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    Vec2::new(
                                        side_bldg_pos.x - (side_bldg_length / 2)
                                            + (r * side_bldg_width),
                                        side_bldg_pos.y + 1 + side_bldg_width + 7,
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    2.5,
                                )
                                .clear();

                            // side bldg window sprites
                            painter
                                .line(
                                    Vec2::new(
                                        side_bldg_pos.x - side_bldg_length - 7,
                                        side_bldg_pos.y - 1 - (side_bldg_width / 2)
                                            + (r * ((side_bldg_length) - 2)),
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    Vec2::new(
                                        side_bldg_pos.x + 1 + side_bldg_length + 7,
                                        side_bldg_pos.y - 1 - (side_bldg_width / 2)
                                            + (r * ((side_bldg_length) - 2)),
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    2.5,
                                )
                                .intersect(side_bldg_window_limiter)
                                .fill(window_ver2.clone());
                            painter
                                .line(
                                    Vec2::new(
                                        side_bldg_pos.x - (side_bldg_length / 2)
                                            + (r * side_bldg_width),
                                        side_bldg_pos.y - side_bldg_width - 7,
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    Vec2::new(
                                        side_bldg_pos.x - (side_bldg_length / 2)
                                            + (r * side_bldg_width),
                                        side_bldg_pos.y + 1 + side_bldg_width + 7,
                                    )
                                    .with_z(
                                        side_bldg_base
                                            + (side_bldg_roof_height / 4)
                                            + (t * (side_bldg_roof_height / 3))
                                            + (4 * h),
                                    ),
                                    2.5,
                                )
                                .intersect(side_bldg_window_limiter)
                                .fill(window_ver.clone());
                        }
                        // side_bldg window sills
                        painter
                            .aabb(Aabb {
                                min: Vec2::new(
                                    side_bldg_pos.x - 1 - (side_bldg_length / 2)
                                        + (r * ((side_bldg_length) - 3)),
                                    side_bldg_pos.y - side_bldg_width - 6,
                                )
                                .with_z(
                                    side_bldg_base - 2
                                        + (side_bldg_roof_height / 4)
                                        + (t * (side_bldg_roof_height / 3)),
                                ),
                                max: Vec2::new(
                                    side_bldg_pos.x + 2 - (side_bldg_length / 2)
                                        + (r * (side_bldg_length - 3)),
                                    side_bldg_pos.y + side_bldg_width + 6,
                                )
                                .with_z(
                                    side_bldg_base - 1
                                        + (side_bldg_roof_height / 4)
                                        + (t * (side_bldg_roof_height / 3)),
                                ),
                            })
                            .fill(wood.clone());

                        painter
                            .aabb(Aabb {
                                min: Vec2::new(
                                    side_bldg_pos.x - side_bldg_length - 6,
                                    side_bldg_pos.y - 2 - (side_bldg_width / 2)
                                        + (r * (side_bldg_width + 1)),
                                )
                                .with_z(
                                    side_bldg_base - 2
                                        + (side_bldg_roof_height / 4)
                                        + (t * (side_bldg_roof_height / 3)),
                                ),
                                max: Vec2::new(
                                    side_bldg_pos.x + side_bldg_length + 6,
                                    side_bldg_pos.y + 1 - (side_bldg_width / 2)
                                        + (r * (side_bldg_width + 1)),
                                )
                                .with_z(
                                    side_bldg_base - 1
                                        + (side_bldg_roof_height / 4)
                                        + (t * (side_bldg_roof_height / 3)),
                                ),
                            })
                            .fill(wood.clone());
                    }
                }
                // top gangway
                let gangway_handle = 2;
                // mark loot entry with light
                painter
                    .line(
                        center.with_z(side_bldg_base + (2 * castle_height) + gangway_handle),
                        side_bldg_pos_2
                            .with_z(side_bldg_base + (2 * castle_height) + gangway_handle),
                        6.0,
                    )
                    .fill(roof_color.clone());
                painter
                    .line(
                        center.with_z(side_bldg_base + (2 * castle_height) + gangway_handle),
                        side_bldg_pos.with_z(side_bldg_base + (2 * castle_height) + gangway_handle),
                        5.0,
                    )
                    .fill(wood.clone());
                painter
                    .line(
                        center.with_z(side_bldg_base + (2 * castle_height) + gangway_handle),
                        side_bldg_pos.with_z(side_bldg_base + (2 * castle_height) + gangway_handle),
                        4.0,
                    )
                    .clear();
            }
            // clear side_bldg room
            painter
                .aabb(Aabb {
                    min: Vec2::new(
                        side_bldg_pos.x - side_bldg_length - 3,
                        side_bldg_pos.y - side_bldg_width - 3,
                    )
                    .with_z(side_bldg_base),
                    max: Vec2::new(
                        side_bldg_pos.x + side_bldg_length + 3,
                        side_bldg_pos.y + side_bldg_width + 3,
                    )
                    .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 1),
                })
                .clear();
            // side bldg room decor
            for r in 0..2 {
                let row = r * (side_bldg_roof_height + (2 * side_bldg_height) - 2);
                painter
                    .aabb(Aabb {
                        min: Vec2::new(
                            side_bldg_pos.x - side_bldg_length - 3,
                            side_bldg_pos.y - side_bldg_width - 3,
                        )
                        .with_z(side_bldg_base + row),
                        max: Vec2::new(
                            side_bldg_pos.x + side_bldg_length + 3,
                            side_bldg_pos.y + side_bldg_width + 3,
                        )
                        .with_z(side_bldg_base + row + 1),
                    })
                    .fill(wood.clone());
                painter
                    .aabb(Aabb {
                        min: Vec2::new(
                            side_bldg_pos.x - side_bldg_length - 1,
                            side_bldg_pos.y - side_bldg_width - 1,
                        )
                        .with_z(side_bldg_base + row),
                        max: Vec2::new(
                            side_bldg_pos.x + side_bldg_length + 1,
                            side_bldg_pos.y + side_bldg_width + 1,
                        )
                        .with_z(side_bldg_base + row + 1),
                    })
                    .clear();
            }
            // wood decor and podiums with candle sprites
            let podium_limiter_1 = painter.aabb(Aabb {
                min: Vec2::new(
                    side_bldg_pos.x - side_bldg_length - 3,
                    side_bldg_pos.y - side_bldg_width - 3,
                )
                .with_z(side_bldg_base),
                max: Vec2::new(
                    side_bldg_pos.x + side_bldg_length + 3,
                    side_bldg_pos.y + side_bldg_width + 3,
                )
                .with_z(side_bldg_base + 3),
            });
            let podium_limiter_2 = painter.aabb(Aabb {
                min: Vec2::new(
                    side_bldg_pos.x - side_bldg_length - 3,
                    side_bldg_pos.y - side_bldg_width - 3,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 10),
                max: Vec2::new(
                    side_bldg_pos.x + side_bldg_length + 3,
                    side_bldg_pos.y + side_bldg_width + 3,
                )
                .with_z(side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 6),
            });
            let mut side_bldg_beam_pos = vec![];
            for dir in DIAGONALS {
                let corner_pos = Vec2::new(
                    side_bldg_pos.x + (dir.x * (side_bldg_length + 2)),
                    side_bldg_pos.y + (dir.y * (side_bldg_width + 2)),
                );
                side_bldg_beam_pos.push(corner_pos);
            }
            for corner_pos in &side_bldg_beam_pos {
                for n in 0..3 {
                    painter
                        .cylinder_with_radius(
                            corner_pos.with_z(side_bldg_base + n),
                            (5 - n) as f32,
                            1.0,
                        )
                        .intersect(podium_limiter_1)
                        .fill(match n {
                            2 => candles.clone(),
                            _ => wood.clone(),
                        });
                    painter
                        .cylinder_with_radius(
                            corner_pos.with_z(
                                side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height)
                                    - 10
                                    + n,
                            ),
                            (2 + n) as f32,
                            1.0,
                        )
                        .intersect(podium_limiter_2)
                        .fill(wood.clone());
                }
                painter
                    .cylinder_with_radius(
                        corner_pos.with_z(
                            side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 7,
                        ),
                        4.0,
                        1.0,
                    )
                    .intersect(podium_limiter_2)
                    .fill(candles.clone());
            }
            if b < 2 {
                // side_bldg stairway
                let side_bldg_room_stairs = painter.aabb(Aabb {
                    min: Vec2::new(
                        side_bldg_pos.x - side_bldg_length - 3,
                        side_bldg_pos.y - side_bldg_width - 3,
                    )
                    .with_z(side_bldg_base),
                    max: Vec2::new(
                        side_bldg_pos.x + side_bldg_length + 3,
                        side_bldg_pos.y + side_bldg_width + 3,
                    )
                    .with_z(side_bldg_base + (2 * castle_height) + 2),
                });
                side_bldg_room_stairs
                    .sample(wall_staircase(
                        side_bldg_pos.with_z(side_bldg_base + 1),
                        (side_bldg_width + 3) as f32,
                        side_bldg_roof_height as f32,
                    ))
                    .fill(candles_lite.clone());
                side_bldg_room_stairs
                    .sample(wall_staircase(
                        side_bldg_pos.with_z(side_bldg_base),
                        (side_bldg_width + 3) as f32,
                        side_bldg_roof_height as f32,
                    ))
                    .fill(wood.clone());
                // platforms
                for dir in DIAGONALS {
                    let chain_pos = Vec2::new(
                        side_bldg_pos.x + dir.x * (side_bldg_length - 4),
                        side_bldg_pos.y + dir.y * (side_bldg_width - 4),
                    );
                    painter
                        .aabb(Aabb {
                            min: chain_pos.with_z(castle_base + side_bldg_height - 1),
                            max: (chain_pos + 1).with_z(
                                side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 1,
                            ),
                        })
                        .fill(chain.clone());
                }
                for p in 1..=4 {
                    let npc_var =
                        RandomField::new(0).get(side_bldg_pos.with_z(castle_base + p)) % 5;

                    painter
                        .aabb(Aabb {
                            min: Vec2::new(
                                side_bldg_pos.x - side_bldg_length + 4,
                                side_bldg_pos.y - side_bldg_width + 4,
                            )
                            .with_z(castle_base + (side_bldg_height * p) - 2),
                            max: Vec2::new(
                                side_bldg_pos.x + side_bldg_length - 3,
                                side_bldg_pos.y + side_bldg_width - 3,
                            )
                            .with_z(castle_base + (side_bldg_height * p) - 1),
                        })
                        .fill(wood.clone());
                    if npc_var > 0 {
                        random_npc_positions
                            .push(side_bldg_pos.with_z(castle_base + (side_bldg_height * p) + 2));
                    }
                }
            } else {
                // side bldg far, foundation, entries, stairs for surrounding side bldg
                // side bldg entries
                let entry_limiter = painter.aabb(Aabb {
                    min: (side_bldg_pos - (2 * side_bldg_length))
                        .with_z(side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2)),
                    max: (side_bldg_pos + (2 * side_bldg_length)).with_z(
                        side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2) + 7,
                    ),
                });
                for dir in CARDINALS {
                    let entry_pos = side_bldg_pos + dir * (side_bldg_length + 4);
                    painter
                        .line(
                            side_bldg_pos.with_z(
                                side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2),
                            ),
                            entry_pos.with_z(
                                side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2),
                            ),
                            6.5,
                        )
                        .intersect(entry_limiter)
                        .clear();
                }
                let side_bldg_npc_pos = side_bldg_pos
                    .with_z(side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2) + 2);
                random_npc_positions.push(side_bldg_npc_pos);

                // foundation
                painter
                    .aabb(Aabb {
                        min: Vec2::new(
                            side_bldg_pos.x - side_bldg_length - 20,
                            side_bldg_pos.y - side_bldg_width - 20,
                        )
                        .with_z(side_bldg_base - 40),
                        max: Vec2::new(
                            side_bldg_pos.x + side_bldg_length + 20,
                            side_bldg_pos.y + side_bldg_width + 20,
                        )
                        .with_z(side_bldg_base + 2),
                    })
                    .fill(brick.clone());
                painter
                    .aabb(Aabb {
                        min: Vec2::new(
                            side_bldg_pos.x - side_bldg_length - 8,
                            side_bldg_pos.y - side_bldg_width - 8,
                        )
                        .with_z(side_bldg_base + 1),
                        max: Vec2::new(
                            side_bldg_pos.x + side_bldg_length + 8,
                            side_bldg_pos.y + side_bldg_width + 8,
                        )
                        .with_z(side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2)),
                    })
                    .fill(brick.clone());
                // stairs
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(
                                side_bldg_pos.x - (2 * side_bldg_length) - 4,
                                side_bldg_pos.y - side_bldg_width - 12,
                            )
                            .with_z(side_bldg_base + 2),
                            max: Vec2::new(
                                side_bldg_pos.x + side_bldg_length + 8,
                                side_bldg_pos.y - side_bldg_width - 8,
                            )
                            .with_z(
                                side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2),
                            ),
                        },
                        (2 * side_bldg_length) + 20,
                        Dir::X,
                    )
                    .fill(brick.clone());
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(
                                side_bldg_pos.x - side_bldg_length - 8,
                                side_bldg_pos.y + side_bldg_width + 8,
                            )
                            .with_z(side_bldg_base + 2),
                            max: Vec2::new(
                                side_bldg_pos.x + (2 * side_bldg_length) + 4,
                                side_bldg_pos.y + side_bldg_width + 12,
                            )
                            .with_z(
                                side_bldg_base + side_bldg_roof_height + (side_bldg_height / 2),
                            ),
                        },
                        (2 * side_bldg_length) + 20,
                        Dir::NegX,
                    )
                    .fill(brick.clone());
            }
            // side bldg wood beams
            for corner_pos in side_bldg_beam_pos {
                painter
                    .line(
                        corner_pos.with_z(side_bldg_base),
                        corner_pos.with_z(
                            side_bldg_base + side_bldg_roof_height + (2 * side_bldg_height) - 1,
                        ),
                        2.0,
                    )
                    .fill(wood.clone());
            }
        }
        // re clear floor cellar entry
        painter
            .cylinder(Aabb {
                min: (side_bldg_stairs_pos - 6).with_z(side_bldg_base_raw - 2),
                max: (side_bldg_stairs_pos + 6).with_z(side_bldg_base_raw - 1),
            })
            .clear();
        // cellar door to side_bldg
        painter
            .cylinder(Aabb {
                min: (side_bldg_stairs_pos - 7).with_z(side_bldg_base_raw - 1),
                max: (side_bldg_stairs_pos + 7).with_z(side_bldg_base_raw),
            })
            .fill(key_door.clone());
        painter
            .cylinder(Aabb {
                min: (side_bldg_stairs_pos - 2).with_z(side_bldg_base_raw - 1),
                max: (side_bldg_stairs_pos - 1).with_z(side_bldg_base_raw),
            })
            .fill(key_hole.clone());
        // clear castle room
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 3, center.y - castle_width - 3)
                    .with_z(castle_base + (castle_height / 2) + 1),
                max: Vec2::new(center.x + castle_length + 3, center.y + castle_width + 3)
                    .with_z(castle_base + (castle_height / 2) + (2 * castle_height) - 1),
            })
            .clear();
        // side_bldg exit to main roon
        let entry_side = if side_bldg_var < 1 {
            side_bldg_width + 3
        } else {
            -(side_bldg_width + 4)
        };
        painter
            .horizontal_cylinder(
                Aabb {
                    min: Vec2::new(side_bldg_pos_1.x - 4, side_bldg_pos_1.y + entry_side)
                        .with_z(side_bldg_base_raw + (2 * castle_height) - 2),
                    max: Vec2::new(side_bldg_pos_1.x + 5, side_bldg_pos_1.y + entry_side + 1)
                        .with_z(side_bldg_base_raw + (2 * castle_height) + 7),
                },
                Dir::NegY,
            )
            .fill(key_door.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(side_bldg_pos_1.x, side_bldg_pos_1.y + entry_side)
                    .with_z(side_bldg_base_raw + (2 * castle_height)),
                max: Vec2::new(side_bldg_pos_1.x + 1, side_bldg_pos_1.y + entry_side + 1)
                    .with_z(side_bldg_base_raw + (2 * castle_height) + 1),
            })
            .fill(key_hole.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(side_bldg_pos_1.x - 2, side_bldg_pos_1.y + entry_side - 7)
                    .with_z(side_bldg_base_raw + (2 * castle_height) - 2),
                max: Vec2::new(side_bldg_pos_1.x + 3, side_bldg_pos_1.y + entry_side + 8)
                    .with_z(side_bldg_base_raw + (2 * castle_height) - 1),
            })
            .fill(wood.clone());
        painter.sprite(
            side_bldg_pos_2.with_z(side_bldg_base_raw),
            SpriteKind::DungeonChest3,
        );
        // main room bossfight
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 3, center.y - castle_width - 3)
                    .with_z(castle_base + (2 * castle_height) - 7),
                max: Vec2::new(center.x + castle_length + 3, center.y + castle_width + 3)
                    .with_z(castle_base + (2 * castle_height) - 6),
            })
            .fill(candles_lite.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length - 3, center.y - castle_width - 3)
                    .with_z(castle_base + (2 * castle_height) - 8),
                max: Vec2::new(center.x + castle_length + 3, center.y + castle_width + 3)
                    .with_z(castle_base + (2 * castle_height) - 7),
            })
            .fill(wood.clone());
        painter
            .aabb(Aabb {
                min: Vec2::new(center.x - castle_length + 3, center.y - castle_width + 3)
                    .with_z(castle_base + (2 * castle_height) - 8),
                max: Vec2::new(center.x + castle_length - 3, center.y + castle_width - 3)
                    .with_z(castle_base + (2 * castle_height) - 6),
            })
            .clear();
        // castle room decor
        for r in 0..2 {
            let row = r * ((2 * castle_height) - 3);
            painter
                .aabb(Aabb {
                    min: Vec2::new(center.x - castle_length - 3, center.y - castle_width - 3)
                        .with_z(castle_base + (castle_height / 2) + 1 + row),
                    max: Vec2::new(center.x + castle_length + 3, center.y + castle_width + 3)
                        .with_z(castle_base + (castle_height / 2) + 2 + row),
                })
                .fill(wood.clone());
            painter
                .aabb(Aabb {
                    min: Vec2::new(center.x - castle_length - 1, center.y - castle_width - 1)
                        .with_z(castle_base + (castle_height / 2) + 1 + row),
                    max: Vec2::new(center.x + castle_length + 1, center.y + castle_width + 1)
                        .with_z(castle_base + (castle_height / 2) + 2 + row),
                })
                .clear();
        }

        // wood decor and podiums with candle sprites
        let castle_podium_limiter_1 = painter.aabb(Aabb {
            min: Vec2::new(center.x - castle_length - 3, center.y - castle_width - 3)
                .with_z(castle_base + (castle_height / 2) + 1),
            max: Vec2::new(center.x + castle_length + 3, center.y + castle_width + 3)
                .with_z(castle_base + (castle_height / 2) + 4),
        });
        let castle_podium_limiter_2 = painter.aabb(Aabb {
            min: Vec2::new(center.x - castle_length - 3, center.y - castle_width - 3)
                .with_z(castle_base + (2 * castle_height) - 10),
            max: Vec2::new(center.x + castle_length + 3, center.y + castle_width + 3)
                .with_z(castle_base + (2 * castle_height) - 6),
        });
        let mut wood_beam_postions = vec![];
        let mut outer_beam_postions = vec![];
        for dir in DIAGONALS {
            let beam_pos_outer = Vec2::new(
                center.x + (dir.x * (castle_length + 2)),
                center.y + (dir.y * (castle_width + 2)),
            );
            let beam_pos_inner = Vec2::new(
                center.x + (dir.x * (castle_length / 4)),
                center.y + (dir.y * (castle_width + 2)),
            );
            wood_beam_postions.push(beam_pos_outer);
            outer_beam_postions.push(beam_pos_outer);
            wood_beam_postions.push(beam_pos_inner);
        }
        for beam_pos in wood_beam_postions {
            for n in 0..2 {
                painter
                    .cylinder_with_radius(
                        beam_pos.with_z(castle_base + (castle_height / 2) + 1 + n),
                        (5 - n) as f32,
                        1.0,
                    )
                    .intersect(castle_podium_limiter_1)
                    .fill(wood.clone());
            }
        }
        for beam_pos in outer_beam_postions {
            for n in 0..3 {
                painter
                    .cylinder_with_radius(
                        beam_pos.with_z(castle_base + (2 * castle_height) - 10 + n),
                        (2 + n) as f32,
                        1.0,
                    )
                    .intersect(castle_podium_limiter_2)
                    .fill(wood.clone());
            }
            painter
                .cylinder_with_radius(
                    beam_pos.with_z(castle_base + (2 * castle_height) - 7),
                    4.0,
                    1.0,
                )
                .intersect(castle_podium_limiter_2)
                .fill(candles.clone());
            painter
                .line(
                    beam_pos.with_z(castle_base + (castle_height / 2) + 1),
                    beam_pos.with_z(castle_base + (castle_height / 2) + (2 * castle_height) - 1),
                    2.0,
                )
                .fill(wood.clone());
        }
        // boss
        let boss_pos = center.with_z(castle_base + (castle_height / 2) + 2);
        painter.spawn(EntityInfo::at(boss_pos.as_()).with_asset_expect(
            "common.entity.dungeon.vampire.bloodmoon_bat",
            &mut thread_rng,
            None,
        ));
        // bats
        for bat_pos in bat_positions {
            for _ in 0..2 {
                painter.spawn(EntityInfo::at(bat_pos.as_()).with_asset_expect(
                    "common.entity.dungeon.vampire.vampire_bat",
                    &mut thread_rng,
                    None,
                ))
            }
        }
        // harlequins
        for harlequin_pos in harlequin_positions {
            painter.spawn(EntityInfo::at(harlequin_pos.as_()).with_asset_expect(
                "common.entity.dungeon.vampire.harlequin",
                &mut thread_rng,
                None,
            ))
        }
        for npc_pos in random_npc_positions {
            spawn_random_entity(npc_pos, painter);
        }
    }
}

pub fn spawn_random_entity(pos: Vec3<i32>, painter: &Painter) {
    let mut rng = thread_rng();
    let entities = [
        "common.entity.dungeon.vampire.strigoi",
        "common.entity.dungeon.vampire.executioner",
        "common.entity.dungeon.vampire.bloodservant",
    ];
    let random_entity_index = rng.gen_range(0..entities.len());
    let random_entity = entities[random_entity_index];
    painter.spawn(EntityInfo::at(pos.as_()).with_asset_expect(random_entity, &mut rng, None));
}
