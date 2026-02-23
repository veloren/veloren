use std::{f32::consts::TAU, sync::Arc};

use crate::{
    Land,
    site::{Dir, Fill, Painter, Site, Structure, generation::spiral_staircase},
    util::{CARDINALS, DIAGONALS, RandomField, Sampler},
};
use common::{
    generation::{EntityInfo, SpecialEntity},
    terrain::{Block, BlockKind, SpriteKind},
};
use rand::RngExt;
use vek::*;

pub struct DesertCityArena {
    /// Approximate altitude of the door tile
    pub(crate) alt: i32,
    pub base: i32,
    pub center: Vec2<i32>,
    // arena
    // config
    length: i32,
    width: i32,
    height: i32,
    corner: i32,
    wall_th: i32,
    pillar_size: i32,
    top_height: i32,
    pub stand_dist: i32,
    pub stand_length: i32,
    pub stand_width: i32,
}

impl DesertCityArena {
    pub fn generate(
        land: &Land,
        _rng: &mut impl RngExt,
        site: &Site,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        let bounds = Aabr {
            min: site.tile_wpos(tile_aabr.min),
            max: site.tile_wpos(tile_aabr.max),
        };
        let alt = land.get_alt_approx(site.tile_center_wpos((tile_aabr.max - tile_aabr.min) / 2))
            as i32
            + 2;
        let base = alt + 1;
        let center = bounds.center();
        // arena
        // config
        let length = 160;
        let width = length / 2;
        let height = length / 6;
        let corner = length / 5;
        let wall_th = 3;
        let pillar_size = (length / 15) - wall_th;
        let top_height = 3;
        let stand_dist = length / 3;
        let stand_length = length / 6;
        let stand_width = length / 16;

        Self {
            alt,
            base,
            center,
            length,
            width,
            height,
            corner,
            wall_th,
            pillar_size,
            top_height,
            stand_dist,
            stand_length,
            stand_width,
        }
    }

    pub fn radius(&self) -> f32 { 100.0 }

    pub fn entity_at(
        &self,
        _pos: Vec3<i32>,
        _above_block: &Block,
        _dynamic_rng: &mut impl RngExt,
    ) -> Option<EntityInfo> {
        None
    }
}

impl Structure for DesertCityArena {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_arena\0";

    #[cfg_attr(feature = "be-dyn-lib", unsafe(export_name = "render_arena"))]
    fn render_inner(&self, _site: &Site, _land: &Land, painter: &Painter) {
        let base = self.base;
        let center = self.center;

        // arena
        // config
        let length = self.length;
        let width = self.width;
        let height = self.height;
        let corner = self.corner;
        let wall_th = self.wall_th;
        let pillar_size = self.pillar_size;
        let top_height = self.top_height;

        let sandstone = Fill::Sampling(Arc::new(|center| {
            Some(match (RandomField::new(0).get(center)) % 37 {
                0..=8 => Block::new(BlockKind::Rock, Rgb::new(245, 212, 129)),
                9..=17 => Block::new(BlockKind::Rock, Rgb::new(246, 214, 133)),
                18..=26 => Block::new(BlockKind::Rock, Rgb::new(247, 216, 136)),
                27..=35 => Block::new(BlockKind::Rock, Rgb::new(248, 219, 142)),
                _ => Block::new(BlockKind::Rock, Rgb::new(235, 178, 99)),
            })
        }));
        let color = Fill::Block(Block::new(BlockKind::Rock, Rgb::new(19, 48, 76)));
        let chain = Fill::Block(Block::air(SpriteKind::SeaDecorChain));
        let lantern = Fill::Block(Block::air(SpriteKind::Lantern));

        // clear area for entries
        for l in 0..8 {
            painter
                .aabb(Aabb {
                    min: (center - (length / 2) - l).with_z(base + l),
                    max: (center + (length / 2) + l).with_z(base + 1 + l),
                })
                .clear();
        }
        // top
        let top_1 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) - (2 * wall_th),
                center.y - (width / 2) - (2 * wall_th),
            )
            .with_z(base + height + wall_th),
            max: Vec2::new(
                center.x + (length / 2) + (2 * wall_th),
                center.y + (width / 2) + (2 * wall_th),
            )
            .with_z(base + height + wall_th + top_height),
        });
        let top_2 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) - (2 * wall_th) + corner,
                center.y - (width / 2) - corner - (2 * wall_th),
            )
            .with_z(base + height + wall_th),
            max: Vec2::new(
                center.x + (length / 2) + (2 * wall_th) - corner,
                center.y + (width / 2) + corner + (2 * wall_th),
            )
            .with_z(base + height + wall_th + top_height),
        });
        top_1.union(top_2).fill(sandstone.clone());
        let top_carve_1 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) - (2 * wall_th) + 1,
                center.y - (width / 2) - (2 * wall_th) + 1,
            )
            .with_z(base + height + wall_th + top_height - 2),
            max: Vec2::new(
                center.x + (length / 2) + (2 * wall_th) - 1,
                center.y + (width / 2) + (2 * wall_th) - 1,
            )
            .with_z(base + height + wall_th + top_height),
        });
        let top_carve_2 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) - (2 * wall_th) + corner + 1,
                center.y - (width / 2) - corner - (2 * wall_th) + 1,
            )
            .with_z(base + height + wall_th + top_height - 2),
            max: Vec2::new(
                center.x + (length / 2) + (2 * wall_th) - corner - 1,
                center.y + (width / 2) + corner + (2 * wall_th) - 1,
            )
            .with_z(base + height + wall_th + 3),
        });
        top_carve_1.union(top_carve_2).clear();

        let carve_dots = 80.0_f32;
        let carve_dots_radius = length + (length / 2);
        let phi_carve_dots = TAU / carve_dots;
        for n in 1..=carve_dots as i32 {
            let dot_pos = Vec2::new(
                center.x + (carve_dots_radius as f32 * ((n as f32 * phi_carve_dots).cos())) as i32,
                center.y + (carve_dots_radius as f32 * ((n as f32 * phi_carve_dots).sin())) as i32,
            );
            // top decor carve
            painter
                .line(
                    center.with_z(base + height + wall_th + 2),
                    dot_pos.with_z(base + height + wall_th + 2),
                    1.5,
                )
                .clear();
        }

        // pillars and spires
        let mut pillar_positions = vec![];
        let mut pillars = vec![];
        let mut spire_positions = vec![];

        for dir in DIAGONALS {
            let inner_square_pos = center + (dir * ((length / 2) - corner - wall_th));
            let outer_square_pos_1 = Vec2::new(
                center.x + (dir.x * ((length / 2) + (corner / 8) - wall_th)),
                center.y + (dir.y * ((width / 2) - wall_th)),
            );
            let outer_square_pos_2 = Vec2::new(
                center.x + (dir.x * ((length / 2) - corner - wall_th)),
                center.y + (dir.y * ((width / 2) + (5 * (corner / 4)) - wall_th)),
            );
            pillar_positions.push(inner_square_pos);
            pillar_positions.push(outer_square_pos_1);
            pillar_positions.push(outer_square_pos_2);

            let spire_pos_1 = Vec2::new(
                center.x + (dir.x * (length / 10)),
                center.y + (dir.y * (2 * (length / 5))),
            );

            let spire_pos_2 = Vec2::new(
                center.x + (dir.y * (2 * (length / 5))),
                center.y + (dir.x * (length / 11)),
            );
            spire_positions.push(spire_pos_1);
            spire_positions.push(spire_pos_2);
        }
        for pillar_pos in pillar_positions {
            let height_var = (RandomField::new(0).get(pillar_pos.with_z(base)) % 20) as i32;
            let pillar_height = height + 8 + height_var;
            pillars.push((pillar_pos, pillar_height));
        }
        for (pillar_pos, pillar_height) in &pillars {
            // pillar
            painter
                .aabb(Aabb {
                    min: (pillar_pos - pillar_size - wall_th).with_z(base - 10),
                    max: (pillar_pos + pillar_size + wall_th)
                        .with_z(base + pillar_height + wall_th),
                })
                .fill(sandstone.clone());
            // carve large
            painter
                .aabb(Aabb {
                    min: Vec2::new(
                        pillar_pos.x - pillar_size - wall_th,
                        pillar_pos.y - pillar_size,
                    )
                    .with_z(base),
                    max: Vec2::new(
                        pillar_pos.x + pillar_size + wall_th,
                        pillar_pos.y + pillar_size,
                    )
                    .with_z(base + pillar_height),
                })
                .clear();
            painter
                .aabb(Aabb {
                    min: Vec2::new(
                        pillar_pos.x - pillar_size,
                        pillar_pos.y - pillar_size - wall_th,
                    )
                    .with_z(base),
                    max: Vec2::new(
                        pillar_pos.x + pillar_size,
                        pillar_pos.y + pillar_size + wall_th,
                    )
                    .with_z(base + pillar_height),
                })
                .clear();
            // carve small
            for dir in DIAGONALS {
                for c in 0..((pillar_height / 2) - 1) {
                    let carve_pos = pillar_pos + (dir * (pillar_size + wall_th));
                    painter
                        .aabb(Aabb {
                            min: (carve_pos - 1).with_z(base + wall_th + (c * 2)),
                            max: (carve_pos + 1).with_z(base + 1 + wall_th + (c * 2)),
                        })
                        .clear();
                }
            }
            // upper decor
            for d in 0..3 {
                // d1
                painter
                    .horizontal_cylinder(
                        Aabb {
                            min: Vec2::new(
                                pillar_pos.x - pillar_size - 1,
                                pillar_pos.y - 7 + (d * 5),
                            )
                            .with_z(base + pillar_height - 6),
                            max: Vec2::new(
                                pillar_pos.x + pillar_size + 1,
                                pillar_pos.y - 3 + (d * 5),
                            )
                            .with_z(base + pillar_height),
                        },
                        Dir::X,
                    )
                    .fill(sandstone.clone());
                painter
                    .horizontal_cylinder(
                        Aabb {
                            min: Vec2::new(
                                pillar_pos.x - pillar_size - 1,
                                pillar_pos.y - 6 + (d * 5),
                            )
                            .with_z(base + pillar_height - 5),
                            max: Vec2::new(
                                pillar_pos.x + pillar_size + 1,
                                pillar_pos.y - 4 + (d * 5),
                            )
                            .with_z(base + pillar_height - 1),
                        },
                        Dir::X,
                    )
                    .clear();

                // d2
                painter
                    .horizontal_cylinder(
                        Aabb {
                            min: Vec2::new(
                                pillar_pos.x - 7 + (d * 5),
                                pillar_pos.y - pillar_size - 1,
                            )
                            .with_z(base + pillar_height - 6),
                            max: Vec2::new(
                                pillar_pos.x - 3 + (d * 5),
                                pillar_pos.y + pillar_size + 1,
                            )
                            .with_z(base + pillar_height),
                        },
                        Dir::Y,
                    )
                    .fill(sandstone.clone());
                painter
                    .horizontal_cylinder(
                        Aabb {
                            min: Vec2::new(
                                pillar_pos.x - 6 + (d * 5),
                                pillar_pos.y - pillar_size - 1,
                            )
                            .with_z(base + pillar_height - 5),
                            max: Vec2::new(
                                pillar_pos.x - 4 + (d * 5),
                                pillar_pos.y + pillar_size + 1,
                            )
                            .with_z(base + pillar_height - 1),
                        },
                        Dir::Y,
                    )
                    .clear();
            }
            // arches
            // a1
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size,
                            pillar_pos.y - pillar_size - wall_th + 1,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size,
                            pillar_pos.y + pillar_size + wall_th - 1,
                        )
                        .with_z(base + (4 * pillar_size) + 2),
                    },
                    Dir::Y,
                )
                .fill(sandstone.clone());
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size + 2,
                            pillar_pos.y - pillar_size - wall_th + 1,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size - 2,
                            pillar_pos.y + pillar_size + wall_th - 1,
                        )
                        .with_z(base + (4 * pillar_size)),
                    },
                    Dir::Y,
                )
                .clear();
            // a2
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size + 2,
                            pillar_pos.y - pillar_size - wall_th + 2,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size - 2,
                            pillar_pos.y + pillar_size + wall_th - 2,
                        )
                        .with_z(base + (4 * pillar_size)),
                    },
                    Dir::Y,
                )
                .fill(sandstone.clone());
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size + 4,
                            pillar_pos.y - pillar_size - wall_th + 2,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size - 4,
                            pillar_pos.y + pillar_size + wall_th - 2,
                        )
                        .with_z(base + (4 * pillar_size) - 2),
                    },
                    Dir::Y,
                )
                .clear();
            // b1
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size - wall_th + 1,
                            pillar_pos.y - pillar_size,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size + wall_th - 1,
                            pillar_pos.y + pillar_size,
                        )
                        .with_z(base + (4 * pillar_size) + 2),
                    },
                    Dir::X,
                )
                .fill(sandstone.clone());
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size - wall_th + 1,
                            pillar_pos.y - pillar_size + 2,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size + wall_th - 1,
                            pillar_pos.y + pillar_size - 2,
                        )
                        .with_z(base + (4 * pillar_size)),
                    },
                    Dir::X,
                )
                .clear();
            // b2
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size - wall_th + 2,
                            pillar_pos.y - pillar_size + 2,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size + wall_th - 2,
                            pillar_pos.y + pillar_size - 2,
                        )
                        .with_z(base + (4 * pillar_size)),
                    },
                    Dir::X,
                )
                .fill(sandstone.clone());
            painter
                .vault(
                    Aabb {
                        min: Vec2::new(
                            pillar_pos.x - pillar_size - wall_th + 2,
                            pillar_pos.y - pillar_size + 4,
                        )
                        .with_z(base),
                        max: Vec2::new(
                            pillar_pos.x + pillar_size + wall_th - 2,
                            pillar_pos.y + pillar_size - 4,
                        )
                        .with_z(base + (4 * pillar_size) - 2),
                    },
                    Dir::X,
                )
                .clear();
            // top
            painter
                .aabb(Aabb {
                    min: (pillar_pos - pillar_size - (2 * wall_th))
                        .with_z(base + pillar_height + wall_th),
                    max: (pillar_pos + pillar_size + (2 * wall_th))
                        .with_z(base + pillar_height + wall_th + top_height),
                })
                .fill(sandstone.clone());
            painter
                .aabb(Aabb {
                    min: (pillar_pos - pillar_size - (2 * wall_th) + 1)
                        .with_z(base + pillar_height + wall_th + top_height - 2),
                    max: (pillar_pos + pillar_size + (2 * wall_th) - 1)
                        .with_z(base + pillar_height + wall_th + top_height),
                })
                .clear();

            let pillar_inlay = painter.aabb(Aabb {
                min: (pillar_pos - pillar_size).with_z(base),
                max: (pillar_pos + pillar_size).with_z(base + pillar_height),
            });
            pillar_inlay.fill(color.clone());
            // decor
            for r in 0..(pillar_height - 3) {
                let dots = 8.0_f32 + (r / 3) as f32;
                let dots_radius = 2 * pillar_size;
                let phi_dots = TAU / dots;
                for n in 1..=dots as i32 {
                    let dot_pos = Vec2::new(
                        pillar_pos.x + (dots_radius as f32 * ((n as f32 * phi_dots).cos())) as i32,
                        pillar_pos.y + (dots_radius as f32 * ((n as f32 * phi_dots).sin())) as i32,
                    );
                    if dots == 16.0_f32 {
                        // top decor carve
                        painter
                            .line(
                                pillar_pos.with_z(base + pillar_height + wall_th + 2),
                                dot_pos.with_z(base + pillar_height + wall_th + 2),
                                1.5,
                            )
                            .clear();
                    }
                    // dots
                    painter
                        .line(
                            pillar_pos.with_z(base + (r * 2)),
                            dot_pos.with_z(base + (r * 2)),
                            0.5,
                        )
                        .intersect(pillar_inlay)
                        .fill(sandstone.clone());
                }
            }
        }

        // arena
        let outer_aabb_1 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) - wall_th,
                center.y - (width / 2) - wall_th,
            )
            .with_z(base - 10),
            max: Vec2::new(
                center.x + (length / 2) + wall_th,
                center.y + (width / 2) + wall_th,
            )
            .with_z(base + height + wall_th),
        });
        let outer_aabb_2 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) - wall_th + corner,
                center.y - (width / 2) - corner - wall_th,
            )
            .with_z(base - 10),
            max: Vec2::new(
                center.x + (length / 2) + wall_th - corner,
                center.y + (width / 2) + corner + wall_th,
            )
            .with_z(base + height + wall_th),
        });
        outer_aabb_1.union(outer_aabb_2).fill(sandstone.clone());
        // decor carve
        let clear_aabb_1 = painter.aabb(Aabb {
            min: Vec2::new(center.x - (length / 2) - wall_th, center.y - (width / 2)).with_z(base),
            max: Vec2::new(center.x + (length / 2) + wall_th, center.y + (width / 2))
                .with_z(base + height),
        });
        let clear_aabb_2 = painter.aabb(Aabb {
            min: Vec2::new(center.x - (length / 2), center.y - (width / 2) - wall_th).with_z(base),
            max: Vec2::new(center.x + (length / 2), center.y + (width / 2) + wall_th)
                .with_z(base + height),
        });
        clear_aabb_1.union(clear_aabb_2).clear();
        let clear_aabb_3 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) - wall_th + corner,
                center.y - (width / 2) - corner,
            )
            .with_z(base),
            max: Vec2::new(
                center.x + (length / 2) + wall_th - corner,
                center.y + (width / 2) + corner,
            )
            .with_z(base + height),
        });
        let clear_aabb_4 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) + corner,
                center.y - (width / 2) - wall_th - corner,
            )
            .with_z(base),
            max: Vec2::new(
                center.x + (length / 2) - corner,
                center.y + (width / 2) + wall_th + corner,
            )
            .with_z(base + height),
        });
        clear_aabb_3.union(clear_aabb_4).clear();
        // color inlay
        let inlay_aabb_1 = painter.aabb(Aabb {
            min: Vec2::new(center.x - (length / 2), center.y - (width / 2)).with_z(base),
            max: Vec2::new(center.x + (length / 2), center.y + (width / 2)).with_z(base + height),
        });
        let inlay_aabb_2 = painter.aabb(Aabb {
            min: Vec2::new(
                center.x - (length / 2) + corner,
                center.y - (width / 2) - corner,
            )
            .with_z(base),
            max: Vec2::new(
                center.x + (length / 2) - corner,
                center.y + (width / 2) + corner,
            )
            .with_z(base + height),
        });
        inlay_aabb_1.union(inlay_aabb_2).fill(color.clone());
        let inlay = inlay_aabb_1.union(inlay_aabb_2);
        for r in 0..(height - 3) {
            let dots = 50.0_f32 + (3 * r) as f32;
            let dots_radius = length + (length / 2);
            let phi_dots = TAU / dots;
            for n in 1..=dots as i32 {
                let dot_pos = Vec2::new(
                    center.x + (dots_radius as f32 * ((n as f32 * phi_dots).cos())) as i32,
                    center.y + (dots_radius as f32 * ((n as f32 * phi_dots).sin())) as i32,
                );
                // color decor
                painter
                    .line(
                        center.with_z(base + (r * 2)),
                        dot_pos.with_z(base + (r * 2)),
                        1.0,
                    )
                    .intersect(inlay)
                    .fill(sandstone.clone());
            }
        }
        // entries
        painter
            .vault(
                Aabb {
                    min: Vec2::new(center.x - (length / 2) - wall_th, center.y - 10).with_z(base),
                    max: Vec2::new(center.x + (length / 2) + wall_th, center.y + 10)
                        .with_z(base + (height / 2) + 8),
                },
                Dir::X,
            )
            .fill(sandstone.clone());
        painter
            .vault(
                Aabb {
                    min: Vec2::new(center.x - 10, center.y - (width / 2) - corner - wall_th)
                        .with_z(base),
                    max: Vec2::new(center.x + 10, center.y + (width / 2) + corner + wall_th)
                        .with_z(base + (height / 2) + 8),
                },
                Dir::Y,
            )
            .fill(sandstone.clone());
        painter
            .vault(
                Aabb {
                    min: Vec2::new(center.x - (length / 2) - wall_th, center.y - 10 + wall_th)
                        .with_z(base),
                    max: Vec2::new(center.x + (length / 2) + wall_th, center.y + 10 - wall_th)
                        .with_z(base + (height / 2) + 8 - wall_th),
                },
                Dir::X,
            )
            .clear();
        painter
            .vault(
                Aabb {
                    min: Vec2::new(
                        center.x - 10 + wall_th,
                        center.y - (width / 2) - corner - wall_th,
                    )
                    .with_z(base),
                    max: Vec2::new(
                        center.x + 10 - wall_th,
                        center.y + (width / 2) + corner + wall_th,
                    )
                    .with_z(base + (height / 2) + 8 - wall_th),
                },
                Dir::Y,
            )
            .clear();
        // center clear
        painter
            .aabb(Aabb {
                min: Vec2::new(
                    center.x - (length / 2) + corner + (2 * wall_th) + pillar_size,
                    center.y - (length / 2) + corner + (2 * wall_th) + pillar_size,
                )
                .with_z(base + height),
                max: Vec2::new(
                    center.x + (length / 2) - corner - (2 * wall_th) - pillar_size,
                    center.y + (length / 2) - corner - (2 * wall_th) - pillar_size,
                )
                .with_z(base + height + wall_th + top_height - 2),
            })
            .clear();
        painter
            .aabb(Aabb {
                min: Vec2::new(
                    center.x - (length / 2) + corner + wall_th,
                    center.y - (length / 2) + corner + wall_th,
                )
                .with_z(base - 1),
                max: Vec2::new(
                    center.x + (length / 2) - corner - wall_th,
                    center.y + (length / 2) - corner - wall_th,
                )
                .with_z(base + height),
            })
            .clear();
        // center decor
        for d in 0..((length / 12) - 2) {
            // d1
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - 3 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y - (length / 2) + corner + (2 * wall_th) + pillar_size - 1,
                        )
                        .with_z(base + height + wall_th + top_height - 7),
                        max: Vec2::new(
                            center.x + 3 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y + (length / 2) - corner - (2 * wall_th) - pillar_size + 1,
                        )
                        .with_z(base + height + wall_th + top_height - 1),
                    },
                    Dir::Y,
                )
                .fill(sandstone.clone());
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - 3 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y - (length / 2) + corner + (2 * wall_th) + pillar_size,
                        )
                        .with_z(base + height + wall_th + top_height - 7),
                        max: Vec2::new(
                            center.x + 3 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y + (length / 2) - corner - (2 * wall_th) - pillar_size,
                        )
                        .with_z(base + height + wall_th + top_height - 1),
                    },
                    Dir::Y,
                )
                .clear();
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - 2 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y - (length / 2) + corner + (2 * wall_th) + pillar_size - 2,
                        )
                        .with_z(base + height + wall_th + top_height - 6),
                        max: Vec2::new(
                            center.x + 2 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y + (length / 2) - corner - (2 * wall_th) - pillar_size + 2,
                        )
                        .with_z(base + height + wall_th + top_height - 2),
                    },
                    Dir::Y,
                )
                .fill(color.clone());
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - 2 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y - (length / 2) + corner + (2 * wall_th) + pillar_size - 1,
                        )
                        .with_z(base + height + wall_th + top_height - 6),
                        max: Vec2::new(
                            center.x + 2 - (length / 2)
                                + corner
                                + (3 * wall_th)
                                + pillar_size
                                + (6 * d)
                                + 2,
                            center.y + (length / 2) - corner - (2 * wall_th) - pillar_size + 1,
                        )
                        .with_z(base + height + wall_th + top_height - 2),
                    },
                    Dir::Y,
                )
                .clear();
        }
        for e in 0..(length / 14) {
            // d2
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - (length / 2) + corner + (2 * wall_th) + pillar_size - 1,
                            center.y - 3 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 7),
                        max: Vec2::new(
                            center.x + (length / 2) - corner - (2 * wall_th) - pillar_size + 1,
                            center.y + 3 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 1),
                    },
                    Dir::X,
                )
                .fill(sandstone.clone());
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - (length / 2) + corner + (2 * wall_th) + pillar_size,
                            center.y - 3 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 7),
                        max: Vec2::new(
                            center.x + (length / 2) - corner - (2 * wall_th) - pillar_size,
                            center.y + 3 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 1),
                    },
                    Dir::X,
                )
                .clear();
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - (length / 2) + corner + (2 * wall_th) + pillar_size - 2,
                            center.y - 2 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 6),
                        max: Vec2::new(
                            center.x + (length / 2) - corner - (2 * wall_th) - pillar_size + 2,
                            center.y + 2 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 2),
                    },
                    Dir::X,
                )
                .fill(color.clone());
            painter
                .horizontal_cylinder(
                    Aabb {
                        min: Vec2::new(
                            center.x - (length / 2) + corner + (2 * wall_th) + pillar_size - 1,
                            center.y - 2 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 6),
                        max: Vec2::new(
                            center.x + (length / 2) - corner - (2 * wall_th) - pillar_size + 1,
                            center.y + 2 - (length / 2)
                                + corner
                                + (2 * wall_th)
                                + pillar_size
                                + (6 * e)
                                + 5,
                        )
                        .with_z(base + height + wall_th + top_height - 2),
                    },
                    Dir::X,
                )
                .clear();
        }

        // entry steps
        for dir in CARDINALS {
            let step_pos = Vec2::new(
                center.x + (dir.x * (length / 2)),
                center.y + (dir.y * ((width / 2) + corner)),
            );
            for s in 0..9 {
                painter
                    .aabb(Aabb {
                        min: (step_pos - 7 - s).with_z(base - 2 - s),
                        max: (step_pos + 7 + s).with_z(base - 1 - s),
                    })
                    .fill(sandstone.clone());
            }
        }
        // clear rooms
        for r in 0..2 {
            let room_pos_1 = Vec2::new(
                center.x - (length / 2) + (length / 8) + (r * (length - (length / 4))),
                center.y,
            );
            let room_pos_2 = Vec2::new(
                center.x,
                center.y - (width / 2) - (corner / 2) + (r * (width + corner)),
            );

            painter
                .aabb(Aabb {
                    min: Vec2::new(
                        room_pos_1.x - (length / 8) + wall_th,
                        room_pos_1.y - (width / 2) + wall_th,
                    )
                    .with_z(base - 1),
                    max: Vec2::new(
                        room_pos_1.x + (length / 8) - wall_th,
                        room_pos_1.y + (width / 2) - wall_th,
                    )
                    .with_z(base + height),
                })
                .clear();
            painter
                .aabb(Aabb {
                    min: Vec2::new(
                        room_pos_2.x - (length / 2) + corner + wall_th,
                        room_pos_2.y - (corner / 2) + wall_th,
                    )
                    .with_z(base - 1),
                    max: Vec2::new(
                        room_pos_2.x + (length / 2) - corner - wall_th,
                        room_pos_2.y + (corner / 2) - wall_th,
                    )
                    .with_z(base + height),
                })
                .clear();
            // stands
            for s in 0..1 {
                // distance from center
                let stand_dist = self.stand_dist;
                let stand_length = self.stand_length;
                let stand_width = self.stand_width;
                let floor = s * (height + wall_th + top_height - 1);

                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(center.x - stand_length - 1, center.y - stand_dist)
                                .with_z(base - 1 + floor),
                            max: Vec2::new(
                                center.x + stand_length + 1,
                                center.y - stand_dist + stand_width,
                            )
                            .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::NegY,
                    )
                    .fill(color.clone());
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(center.x - stand_length, center.y - stand_dist)
                                .with_z(base - 1 + floor),
                            max: Vec2::new(
                                center.x + stand_length,
                                center.y - stand_dist + stand_width,
                            )
                            .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::NegY,
                    )
                    .fill(sandstone.clone());
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(
                                center.x - stand_length - 1,
                                center.y + stand_dist - stand_width,
                            )
                            .with_z(base - 1 + floor),
                            max: Vec2::new(center.x + stand_length + 1, center.y + stand_dist)
                                .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::Y,
                    )
                    .fill(color.clone());

                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(
                                center.x - stand_length,
                                center.y + stand_dist - stand_width,
                            )
                            .with_z(base - 1 + floor),
                            max: Vec2::new(center.x + stand_length, center.y + stand_dist)
                                .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::Y,
                    )
                    .fill(sandstone.clone());
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(center.x - stand_dist, center.y - stand_length - 1)
                                .with_z(base - 1 + floor),
                            max: Vec2::new(
                                center.x - stand_dist + stand_width,
                                center.y + stand_length + 1,
                            )
                            .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::NegX,
                    )
                    .fill(color.clone());
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(center.x - stand_dist, center.y - stand_length)
                                .with_z(base - 1 + floor),
                            max: Vec2::new(
                                center.x - stand_dist + stand_width,
                                center.y + stand_length,
                            )
                            .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::NegX,
                    )
                    .fill(sandstone.clone());
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(
                                center.x + stand_dist - stand_width,
                                center.y - stand_length - 1,
                            )
                            .with_z(base - 1 + floor),
                            max: Vec2::new(center.x + stand_dist, center.y + stand_length + 1)
                                .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::X,
                    )
                    .fill(color.clone());
                painter
                    .ramp_inset(
                        Aabb {
                            min: Vec2::new(
                                center.x + stand_dist - stand_width,
                                center.y - stand_length,
                            )
                            .with_z(base - 1 + floor),
                            max: Vec2::new(center.x + stand_dist, center.y + stand_length)
                                .with_z(base + (length / 16) - 1 + floor),
                        },
                        length / 16,
                        Dir::X,
                    )
                    .fill(sandstone.clone());
            }
        }
        for (pillar_pos, pillar_height) in pillars {
            let stairs_radius = pillar_size - 1;
            let stairs = painter.aabb(Aabb {
                min: (pillar_pos - stairs_radius).with_z(base - 1),
                max: (pillar_pos + stairs_radius).with_z(base + pillar_height + wall_th + 1),
            });
            stairs.clear();
            stairs
                .sample(spiral_staircase(
                    pillar_pos.with_z(base + pillar_height + wall_th + 1),
                    (stairs_radius + 2) as f32,
                    0.5,
                    ((pillar_height + wall_th + top_height) / 3) as f32,
                ))
                .fill(sandstone.clone());
        }
        for spire_pos in &spire_positions {
            let spire_height =
                (height / 3) + (RandomField::new(0).get(spire_pos.with_z(base)) % 6) as i32;
            painter
                .cylinder(Aabb {
                    min: (spire_pos - pillar_size - 1)
                        .with_z(base + height + wall_th + top_height - 2),
                    max: (spire_pos + pillar_size + 2)
                        .with_z(base + height + wall_th + top_height + 6),
                })
                .fill(sandstone.clone());
            painter
                .cylinder(Aabb {
                    min: (spire_pos - pillar_size).with_z(base + height + wall_th + top_height + 6),
                    max: (spire_pos + pillar_size + 1)
                        .with_z(base + height + wall_th + top_height + spire_height),
                })
                .fill(color.clone());
            for r in 0..((spire_height / 2) - 3) {
                let spire_dots = 16.0_f32 + (2 * r) as f32;
                let spire_dots_radius = pillar_size as f32 + 0.5;
                let phi_spire_dots = TAU / spire_dots;

                for n in 1..=spire_dots as i32 {
                    let spire_dot_pos = Vec2::new(
                        spire_pos.x
                            + (spire_dots_radius * ((n as f32 * phi_spire_dots).cos())) as i32,
                        spire_pos.y
                            + (spire_dots_radius * ((n as f32 * phi_spire_dots).sin())) as i32,
                    );
                    // color decor
                    painter
                        .line(
                            spire_pos.with_z(base + height + wall_th + top_height + 6 + (r * 2)),
                            spire_dot_pos
                                .with_z(base + height + wall_th + top_height + 6 + (r * 2)),
                            1.0,
                        )
                        .fill(sandstone.clone());
                }
            }

            painter
                .cylinder(Aabb {
                    min: (spire_pos - pillar_size - 1)
                        .with_z(base + height + wall_th + top_height + spire_height),
                    max: (spire_pos + pillar_size + 2)
                        .with_z(base + height + wall_th + top_height + spire_height + 1),
                })
                .fill(sandstone.clone());
            painter
                .sphere(Aabb {
                    min: (spire_pos - pillar_size)
                        .with_z(base + height + wall_th + top_height + spire_height - pillar_size),
                    max: (spire_pos + pillar_size + 1)
                        .with_z(base + height + wall_th + top_height + spire_height + pillar_size),
                })
                .fill(color.clone());
            painter
                .cone(Aabb {
                    min: (spire_pos - 2)
                        .with_z(base + height + wall_th + top_height + spire_height + pillar_size),
                    max: (spire_pos + 3).with_z(
                        base + height
                            + wall_th
                            + top_height
                            + spire_height
                            + pillar_size
                            + (spire_height / 3),
                    ),
                })
                .fill(sandstone.clone());
            // campfires & repair benches
            painter.spawn(
                EntityInfo::at((spire_pos - 2).with_z(base - 1).as_())
                    .into_special(SpecialEntity::Waypoint),
            );
            painter.spawn(EntityInfo::at(center.with_z(base).as_()).into_special(
                SpecialEntity::ArenaTotem {
                    range: length as f32,
                },
            ));
            painter.sprite((spire_pos + 2).with_z(base - 1), SpriteKind::RepairBench);

            // lamps
            let lamps = 8.0_f32;
            let lamps_radius = 3;
            let phi_lamps = TAU / lamps;
            for n in 1..=lamps as i32 {
                let lamp_pos = Vec2::new(
                    spire_pos.x + (lamps_radius as f32 * ((n as f32 * phi_lamps).cos())) as i32,
                    spire_pos.y + (lamps_radius as f32 * ((n as f32 * phi_lamps).sin())) as i32,
                );
                let lamp_var = (RandomField::new(0).get(lamp_pos.with_z(base)) % 8) as i32;
                painter
                    .aabb(Aabb {
                        min: lamp_pos.with_z(base + height - 8 - lamp_var),
                        max: (lamp_pos + 1).with_z(base + height),
                    })
                    .fill(chain.clone());
                painter
                    .aabb(Aabb {
                        min: lamp_pos.with_z(base + height - 9 - lamp_var),
                        max: (lamp_pos + 1).with_z(base + height - 8 - lamp_var),
                    })
                    .fill(lantern.clone());
            }
        }
    }
}
