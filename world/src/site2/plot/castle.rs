use super::*;
use crate::Land;
use common::terrain::{Block, BlockKind};
use rand::prelude::*;
use vek::*;

pub struct Castle {
    _entrance_tile: Vec2<i32>,
    tile_aabr: Aabr<i32>,
    _bounds: Aabr<i32>,
    alt: i32,
}

impl Castle {
    pub fn generate(
        land: &Land,
        _rng: &mut impl Rng,
        site: &Site,
        entrance_tile: Vec2<i32>,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        Self {
            _entrance_tile: entrance_tile,
            tile_aabr,
            _bounds: Aabr {
                min: site.tile_wpos(tile_aabr.min),
                max: site.tile_wpos(tile_aabr.max),
            },
            alt: land.get_alt_approx(site.tile_center_wpos(entrance_tile)) as i32,
        }
    }
}

impl Structure for Castle {
    fn render<F: FnMut(Primitive) -> Id<Primitive>, G: FnMut(Id<Primitive>, Fill)>(
        &self,
        site: &Site,
        mut prim: F,
        mut fill: G,
    ) {
        let wall_height = 24;
        let _thickness = 12;
        let parapet_height = 2;
        let parapet_width = 1;
        let _downwards = 40;

        let tower_height = 12;

        let keep_levels = 3;
        let keep_level_height = 8;
        let _keep_height = wall_height + keep_levels * keep_level_height + 1;
        for x in 0..self.tile_aabr.size().w {
            for y in 0..self.tile_aabr.size().h {
                let tile_pos = self.tile_aabr.min + Vec2::new(x, y);
                let _wpos_center = site.tile_center_wpos(tile_pos);
                let wpos = site.tile_wpos(tile_pos);
                let ori = if x == 0 || x == self.tile_aabr.size().w - 1 {
                    Vec2::new(1, 0)
                } else {
                    Vec2::new(0, 1)
                };
                let ori_tower_x = if x == 0 {
                    Vec2::new(1, 0)
                } else {
                    Vec2::new(0, 0)
                };
                let ori_tower_y = if y == 0 {
                    Vec2::new(0, 1)
                } else {
                    Vec2::new(0, 0)
                };
                let ori_tower = ori_tower_x + ori_tower_y;
                match site.tiles.get(tile_pos).kind.clone() {
                    TileKind::Wall(_ori) => {
                        let wall = prim(Primitive::Aabb(Aabb {
                            min: wpos.with_z(self.alt),
                            max: (wpos + 6).with_z(self.alt + wall_height + parapet_height),
                        }));
                        let cut_path = prim(Primitive::Aabb(Aabb {
                            min: (wpos + (parapet_width * ori) as Vec2<i32>)
                                .with_z(self.alt + wall_height),
                            max: (wpos
                                + (6 - parapet_width) * ori as Vec2<i32>
                                + 6 * ori.yx() as Vec2<i32>)
                                .with_z(self.alt + wall_height + parapet_height),
                        }));
                        let cut_sides1 = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(wpos.x, wpos.y, self.alt + wall_height + 1),
                            max: Vec3::new(
                                wpos.x + 6 * ori.x + ori.y,
                                wpos.y + 6 * ori.y + ori.x,
                                self.alt + wall_height + parapet_height,
                            ),
                        }));
                        let pillar_start = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(wpos.x, wpos.y - 1, self.alt),
                            max: Vec3::new(wpos.x + 1, wpos.y + 7, self.alt + wall_height),
                        }));
                        let pillar_end = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(wpos.x + 5, wpos.y - 1, self.alt),
                            max: Vec3::new(wpos.x + 6, wpos.y + 7, self.alt + wall_height),
                        }));
                        let pillars = prim(Primitive::Or(pillar_start, pillar_end));
                        fill(
                            prim(Primitive::Or(wall, pillars)),
                            Fill::Block(Block::new(BlockKind::Rock, Rgb::new(33, 33, 33))),
                        );
                        fill(cut_path, Fill::Block(Block::empty()));
                        fill(cut_sides1, Fill::Block(Block::empty()));
                    },
                    TileKind::Tower => {
                        let tower_lower = prim(Primitive::Aabb(Aabb {
                            min: wpos.with_z(self.alt),
                            max: (wpos + 6).with_z(self.alt + wall_height + tower_height),
                        }));
                        let tower_lower_inner_x = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(
                                wpos.x + ori_tower.x,
                                wpos.y + parapet_width,
                                self.alt + wall_height,
                            ),
                            max: Vec3::new(
                                wpos.x + 6 + ori_tower.x - 1,
                                wpos.y + 6 - parapet_width,
                                self.alt + wall_height + tower_height / 3,
                            ),
                        }));
                        let tower_lower_inner_y = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(
                                wpos.x + parapet_width,
                                wpos.y + ori_tower.y,
                                self.alt + wall_height,
                            ),
                            max: Vec3::new(
                                wpos.x + 6 - parapet_width,
                                wpos.y + 6 + ori_tower.y - 1,
                                self.alt + wall_height + tower_height / 3,
                            ),
                        }));
                        let tower_lower_inner =
                            prim(Primitive::Or(tower_lower_inner_x, tower_lower_inner_y));
                        fill(
                            prim(Primitive::Xor(tower_lower, tower_lower_inner)),
                            Fill::Block(Block::new(BlockKind::Rock, Rgb::new(33, 33, 33))),
                        );
                        let tower_upper = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(
                                wpos.x - 1,
                                wpos.y - 1,
                                self.alt + wall_height + tower_height - 3i32,
                            ),
                            max: Vec3::new(
                                wpos.x + 7,
                                wpos.y + 7,
                                self.alt + wall_height + tower_height - 1i32,
                            ),
                        }));
                        let tower_upper2 = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(
                                wpos.x - 2,
                                wpos.y - 2,
                                self.alt + wall_height + tower_height - 1i32,
                            ),
                            max: Vec3::new(
                                wpos.x + 8,
                                wpos.y + 8,
                                self.alt + wall_height + tower_height,
                            ),
                        }));

                        fill(
                            prim(Primitive::Or(tower_upper, tower_upper2)),
                            Fill::Block(Block::new(BlockKind::Rock, Rgb::new(33, 33, 33))),
                        );

                        let roof_lip = 1;
                        let roof_height = 8 / 2 + roof_lip + 1;

                        // Roof
                        fill(
                            prim(Primitive::Pyramid {
                                aabb: Aabb {
                                    min: (wpos - 2 - roof_lip)
                                        .with_z(self.alt + wall_height + tower_height),
                                    max: (wpos + 8 + roof_lip).with_z(
                                        self.alt + wall_height + tower_height + roof_height,
                                    ),
                                },
                                inset: roof_height,
                            }),
                            Fill::Block(Block::new(BlockKind::Wood, Rgb::new(116, 20, 20))),
                        );
                    },
                    TileKind::Keep(kind) => {
                        match kind {
                            tile::KeepKind::Middle => {
                                for i in 0..keep_levels + 1 {
                                    let height = keep_level_height * i;
                                    fill(
                                        prim(Primitive::Aabb(Aabb {
                                            min: wpos.with_z(self.alt + height),
                                            max: (wpos + 6).with_z(self.alt + height + 1),
                                        })),
                                        Fill::Block(Block::new(
                                            BlockKind::Rock,
                                            Rgb::new(89, 44, 14),
                                        )),
                                    );
                                }
                            },
                            tile::KeepKind::Corner => {},
                            tile::KeepKind::Wall(_ori) => {
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
    }
}
