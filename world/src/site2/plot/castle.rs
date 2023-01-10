use super::*;
use crate::{util::SQUARE_4, Land};
use common::terrain::{Block, BlockKind};
use num::Integer;
use rand::prelude::*;
use vek::*;

pub struct Castle {
    tile_aabr: Aabr<i32>,
    _bounds: Aabr<i32>,
    gate_aabr: Aabr<i32>,
    gate_alt: i32,
    pub(crate) alt: i32,
}

impl Castle {
    pub fn generate(
        land: &Land,
        _rng: &mut impl Rng,
        site: &Site,
        tile_aabr: Aabr<i32>,
        gate_aabr: Aabr<i32>,
    ) -> Self {
        let alt = SQUARE_4
            .iter()
            .map(|corner| tile_aabr.min + (tile_aabr.max - tile_aabr.min - 1) * corner)
            .map(|pos| land.get_alt_approx(site.tile_center_wpos(pos)) as i32)
            .sum::<i32>()
            / 4;

        Self {
            tile_aabr,
            _bounds: Aabr {
                min: site.tile_wpos(tile_aabr.min),
                max: site.tile_wpos(tile_aabr.max),
            },
            gate_aabr,
            gate_alt: land.get_alt_approx(site.tile_center_wpos(gate_aabr.center())) as i32,
            alt,
        }
    }
}

impl Structure for Castle {
    #[cfg(feature = "use-dyn-lib")]
    const UPDATE_FN: &'static [u8] = b"render_castle\0";

    #[cfg_attr(feature = "be-dyn-lib", export_name = "render_castle")]
    fn render_inner(&self, site: &Site, _land: &Land, painter: &Painter) {
        let wall_height = 24;
        let parapet_height = 2;
        let parapet_gap = 2;
        let parapet_offset = 2;
        let ts = TILE_SIZE as i32;
        let tower_height = 16;
        let keep_levels = 3;
        let keep_level_height = 8;
        let _keep_height = wall_height + keep_levels * keep_level_height + 1;
        let wall_rgb = Rgb::new(38, 46, 43);
        // Flatten inside of the castle
        painter.fill(
            painter.prim(Primitive::Aabb(Aabb {
                min: site.tile_wpos(self.tile_aabr.min).with_z(self.gate_alt),
                max: site
                    .tile_wpos(self.tile_aabr.max)
                    .with_z(self.alt + tower_height),
            })),
            Fill::Block(Block::empty()),
        );
        painter.fill(
            painter.prim(Primitive::Aabb(Aabb {
                min: site.tile_wpos(self.tile_aabr.min).with_z(self.gate_alt),
                max: site.tile_wpos(self.tile_aabr.max).with_z(self.gate_alt + 1),
            })),
            Fill::Block(Block::new(BlockKind::Rock, Rgb::new(55, 45, 65))),
        );
        for x in 0..self.tile_aabr.size().w {
            for y in 0..self.tile_aabr.size().h {
                let tile_pos = self.tile_aabr.min + Vec2::new(x, y);
                let wpos = site.tile_wpos(tile_pos);
                match site.tiles.get(tile_pos).kind.clone() {
                    TileKind::Wall(ori) => {
                        let dir = ori.to_vec2();
                        let wall = painter.prim(Primitive::Aabb(Aabb {
                            min: wpos.with_z(self.alt - 20),
                            max: (wpos + ts).with_z(self.alt + wall_height),
                        }));
                        // TODO Figure out logic to choose on on which site wall should be placed
                        // (inner, outer)
                        let parapet = painter.prim(Primitive::Aabb(Aabb {
                            min: (wpos - dir.yx()).with_z(self.alt + wall_height),
                            max: (wpos + ts * dir).with_z(self.alt + wall_height + parapet_height),
                        }));
                        let parapet2 = painter.prim(Primitive::Aabb(Aabb {
                            min: (wpos + ts * dir.yx()).with_z(self.alt + wall_height),
                            max: (wpos + (ts + 1) * dir.yx() + ts * dir)
                                .with_z(self.alt + wall_height + parapet_height),
                        }));
                        let cut_sides = painter.prim(Primitive::Aabb(Aabb {
                            min: (wpos + parapet_offset * dir - dir.yx())
                                .with_z(self.alt + wall_height + parapet_height - 1),
                            max: (wpos
                                + (ts + 1) * dir.yx()
                                + (parapet_offset + parapet_gap) * dir)
                                .with_z(self.alt + wall_height + parapet_height),
                        }));

                        painter.fill(wall, Fill::Brick(BlockKind::Rock, wall_rgb, 12));
                        let sides = painter.prim(Primitive::union(parapet, parapet2));
                        painter.fill(sides, Fill::Brick(BlockKind::Rock, wall_rgb, 12));
                        if (x + y).is_odd() {
                            painter.fill(
                                painter.prim(Primitive::Aabb(Aabb {
                                    min: (wpos + 2 * dir - dir.yx()).with_z(self.alt - 20),
                                    max: (wpos + 4 * dir + (ts + 1) * dir.yx())
                                        .with_z(self.alt + wall_height),
                                })),
                                Fill::Brick(BlockKind::Rock, wall_rgb, 12),
                            );
                        } else {
                            let window_top = painter.prim(Primitive::Aabb(Aabb {
                                min: (wpos + 2 * dir).with_z(self.alt + wall_height / 4 + 9),
                                max: (wpos + (ts - 2) * dir + dir.yx())
                                    .with_z(self.alt + wall_height / 4 + 12),
                            }));
                            let window_bottom = painter.prim(Primitive::Aabb(Aabb {
                                min: (wpos + 1 * dir).with_z(self.alt + wall_height / 4),
                                max: (wpos + (ts - 1) * dir + dir.yx())
                                    .with_z(self.alt + wall_height / 4 + 9),
                            }));
                            let window_top2 = painter.prim(Primitive::Aabb(Aabb {
                                min: (wpos + 2 * dir + (ts - 1) * dir.yx())
                                    .with_z(self.alt + wall_height / 4 + 9),
                                max: (wpos + (ts - 2) * dir + ts * dir.yx())
                                    .with_z(self.alt + wall_height / 4 + 12),
                            }));
                            let window_bottom2 = painter.prim(Primitive::Aabb(Aabb {
                                min: (wpos + 1 * dir + (ts - 1) * dir.yx())
                                    .with_z(self.alt + wall_height / 4),
                                max: (wpos + (ts - 1) * dir + ts * dir.yx())
                                    .with_z(self.alt + wall_height / 4 + 9),
                            }));

                            painter.fill(window_bottom, Fill::Block(Block::empty()));
                            painter.fill(window_top, Fill::Block(Block::empty()));
                            painter.fill(window_bottom2, Fill::Block(Block::empty()));
                            painter.fill(window_top2, Fill::Block(Block::empty()));
                        }
                        painter.fill(cut_sides, Fill::Block(Block::empty()));
                    },
                    TileKind::Tower(roof) => {
                        let tower_total_height =
                            self.alt + wall_height + parapet_height + tower_height;
                        let tower_lower = painter.prim(Primitive::Aabb(Aabb {
                            min: (wpos - 1).with_z(self.alt - 20),
                            max: (wpos + ts + 1).with_z(tower_total_height),
                        }));
                        painter.fill(tower_lower, Fill::Brick(BlockKind::Rock, wall_rgb, 12));
                        let tower_upper = painter.prim(Primitive::Aabb(Aabb {
                            min: (wpos - 2).with_z(tower_total_height - 4i32),
                            max: (wpos + ts + 2).with_z(tower_total_height - 2i32),
                        }));
                        let tower_upper2 = painter.prim(Primitive::Aabb(Aabb {
                            min: (wpos - 3).with_z(tower_total_height - 2i32),
                            max: (wpos + ts + 3).with_z(tower_total_height),
                        }));

                        painter.fill(
                            painter.prim(Primitive::union(tower_upper, tower_upper2)),
                            Fill::Brick(BlockKind::Rock, wall_rgb, 12),
                        );

                        match roof {
                            RoofKind::Pyramid => {
                                let roof_lip = 1;
                                let roof_height = (ts + 3) / 2 + roof_lip + 1;

                                painter.fill(
                                    painter.prim(Primitive::Pyramid {
                                        aabb: Aabb {
                                            min: (wpos - 3 - roof_lip).with_z(tower_total_height),
                                            max: (wpos + ts + 3 + roof_lip)
                                                .with_z(tower_total_height + roof_height),
                                        },
                                        inset: roof_height,
                                    }),
                                    Fill::Brick(BlockKind::Wood, Rgb::new(40, 5, 11), 10),
                                );
                            },
                            RoofKind::Parapet => {
                                let tower_top_outer = painter.prim(Primitive::Aabb(Aabb {
                                    min: (wpos - 3).with_z(
                                        self.alt + wall_height + parapet_height + tower_height,
                                    ),
                                    max: (wpos + ts + 3)
                                        .with_z(tower_total_height + parapet_height),
                                }));
                                let tower_top_inner = painter.prim(Primitive::Aabb(Aabb {
                                    min: (wpos - 2).with_z(tower_total_height),
                                    max: (wpos + ts + 2)
                                        .with_z(tower_total_height + parapet_height),
                                }));

                                tower_top_outer
                                    .union(tower_top_inner)
                                    .without(tower_top_outer.intersect(tower_top_inner))
                                    .fill(Fill::Brick(BlockKind::Rock, wall_rgb, 12));

                                for x in (wpos.x..wpos.x + ts).step_by(2 * parapet_gap as usize) {
                                    painter.fill(
                                        painter.prim(Primitive::Aabb(Aabb {
                                            min: Vec3::new(x, wpos.y - 3, tower_total_height + 1),
                                            max: Vec3::new(
                                                x + parapet_gap,
                                                wpos.y + ts + 3,
                                                tower_total_height + parapet_height,
                                            ),
                                        })),
                                        Fill::Block(Block::empty()),
                                    );
                                }
                                for y in (wpos.y..wpos.y + ts).step_by(2 * parapet_gap as usize) {
                                    painter.fill(
                                        painter.prim(Primitive::Aabb(Aabb {
                                            min: Vec3::new(wpos.x - 3, y, tower_total_height + 1),
                                            max: Vec3::new(
                                                wpos.x + ts + 3,
                                                y + parapet_gap,
                                                tower_total_height + parapet_height,
                                            ),
                                        })),
                                        Fill::Block(Block::empty()),
                                    );
                                }

                                for &cpos in SQUARE_4.iter() {
                                    let pos = wpos - 3 + (ts + 6) * cpos - cpos;
                                    let pos2 = wpos - 2 + (ts + 4) * cpos - cpos;
                                    painter.fill(
                                        painter.prim(Primitive::Aabb(Aabb {
                                            min: pos.with_z(tower_total_height - 2),
                                            max: (pos + 1)
                                                .with_z(tower_total_height + parapet_height),
                                        })),
                                        Fill::Block(Block::empty()),
                                    );
                                    painter.fill(
                                        painter.prim(Primitive::Aabb(Aabb {
                                            min: pos2.with_z(tower_total_height - 4),
                                            max: (pos2 + 1).with_z(tower_total_height - 2),
                                        })),
                                        Fill::Block(Block::empty()),
                                    );
                                }
                            },
                        }
                    },
                    TileKind::Keep(kind) => {
                        match kind {
                            KeepKind::Middle => {
                                for i in 0..keep_levels + 1 {
                                    let height = keep_level_height * i;
                                    painter.fill(
                                        painter.prim(Primitive::Aabb(Aabb {
                                            min: wpos.with_z(self.alt + height),
                                            max: (wpos + ts).with_z(self.alt + height + 1),
                                        })),
                                        Fill::Block(Block::new(
                                            BlockKind::Wood,
                                            Rgb::new(89, 44, 14),
                                        )),
                                    );
                                }
                            },
                            KeepKind::Corner => {},
                            KeepKind::Wall(_ori) => {
                                for i in 0..keep_levels + 1 {
                                    let _height = keep_level_height * i;
                                    // TODO clamp value in case of big heights
                                    let _window_height = keep_level_height - 3;
                                }
                            },
                        }
                    },
                    _ => {},
                }
            }
        }

        // Render gate here
        // TODO move this into tile loop
        let gate_aabb = Aabb {
            min: (site.tile_wpos(self.gate_aabr.min) + Vec2::unit_x()).with_z(self.gate_alt - 1),
            max: (site.tile_wpos(self.gate_aabr.max) - Vec2::unit_x())
                .with_z(self.alt + wall_height),
        };
        painter.fill(
            painter.prim(Primitive::Aabb(gate_aabb)),
            Fill::Brick(BlockKind::Rock, wall_rgb, 12),
        );
        painter.fill(
            painter.prim(Primitive::Aabb(Aabb {
                min: (gate_aabb.min + Vec3::unit_x() * 2 + Vec3::unit_z() * 2),
                max: (gate_aabb.max - Vec3::unit_x() * 2 - Vec3::unit_z() * 16),
            })),
            Fill::Block(Block::empty()),
        );
        let height = self.alt + wall_height - 17;
        for i in 1..5 {
            painter.fill(
                painter.prim(Primitive::Aabb(Aabb {
                    min: Vec3::new(gate_aabb.min.x + 2 + i, gate_aabb.min.y, height + i),
                    max: Vec3::new(gate_aabb.max.x - 2 - i, gate_aabb.max.y, height + i + 1),
                })),
                Fill::Block(Block::empty()),
            );
        }
        let height = self.alt + wall_height - 7;
        for x in (gate_aabb.min.x + 1..gate_aabb.max.x - 2).step_by(4) {
            painter.fill(
                painter.prim(Primitive::Aabb(Aabb {
                    min: Vec3::new(x, gate_aabb.min.y + 1, height - 13),
                    max: Vec3::new(x + 2, gate_aabb.min.y + 2, height),
                })),
                Fill::Brick(BlockKind::Rock, Rgb::new(27, 35, 32), 8),
            );
        }
        for z in (height - 12..height).step_by(4) {
            painter.fill(
                painter.prim(Primitive::Aabb(Aabb {
                    min: Vec3::new(gate_aabb.min.x + 2, gate_aabb.min.y + 1, z),
                    max: Vec3::new(gate_aabb.max.x - 2, gate_aabb.min.y + 2, z + 2),
                })),
                Fill::Brick(BlockKind::Rock, Rgb::new(27, 35, 32), 8),
            );
        }
    }
}
