use super::*;
use crate::{util::SQUARE_4, Land};
use common::terrain::{Block, BlockKind, SpriteKind};
use rand::prelude::*;
use vek::*;

pub struct House {
    door_tile: Vec2<i32>,
    tile_aabr: Aabr<i32>,
    bounds: Aabr<i32>,
    alt: i32,
    levels: u32,
    roof_color: Rgb<u8>,
}

impl House {
    pub fn generate(
        land: &Land,
        rng: &mut impl Rng,
        site: &Site,
        door_tile: Vec2<i32>,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        Self {
            door_tile,
            tile_aabr,
            bounds: Aabr {
                min: site.tile_wpos(tile_aabr.min),
                max: site.tile_wpos(tile_aabr.max),
            },
            alt: land.get_alt_approx(site.tile_center_wpos(door_tile)) as i32 + 2,
            levels: rng.gen_range(1..2 + (tile_aabr.max - tile_aabr.min).product() / 6) as u32,
            roof_color: {
                let colors = [
                    Rgb::new(21, 43, 48),
                    Rgb::new(11, 23, 38),
                    Rgb::new(45, 28, 21),
                    Rgb::new(10, 55, 40),
                    Rgb::new(5, 35, 15),
                    Rgb::new(40, 5, 11),
                    Rgb::new(55, 45, 11),
                ];
                *colors.choose(rng).unwrap()
            },
        }
    }
}

impl Structure for House {
    fn render<F: FnMut(Primitive) -> Id<Primitive>, G: FnMut(Fill)>(
        &self,
        site: &Site,
        mut prim: F,
        mut fill: G,
    ) {
        let storey = 5;
        let roof = storey * self.levels as i32;
        let foundations = 12;

        // Walls
        let inner = prim(Primitive::Aabb(Aabb {
            min: (self.bounds.min + 1).with_z(self.alt),
            max: self.bounds.max.with_z(self.alt + roof),
        }));
        let outer = prim(Primitive::Aabb(Aabb {
            min: self.bounds.min.with_z(self.alt - foundations),
            max: (self.bounds.max + 1).with_z(self.alt + roof),
        }));
        fill(Fill {
            prim: outer,
            block: Block::new(BlockKind::Rock, Rgb::new(181, 170, 148)),
        });
        fill(Fill {
            prim: inner,
            block: Block::empty(),
        });
        let walls = prim(Primitive::Xor(outer, inner));

        // wall pillars
        let mut pillars_y = prim(Primitive::Empty);
        for x in self.tile_aabr.min.x..self.tile_aabr.max.x + 2 {
            let pillar = prim(Primitive::Aabb(Aabb {
                min: site
                    .tile_wpos(Vec2::new(x, self.tile_aabr.min.y))
                    .with_z(self.alt),
                max: (site.tile_wpos(Vec2::new(x, self.tile_aabr.max.y + 1)) + Vec2::unit_x())
                    .with_z(self.alt + roof),
            }));
            pillars_y = prim(Primitive::Or(pillars_y, pillar));
        }
        let mut pillars_x = prim(Primitive::Empty);
        for y in self.tile_aabr.min.y..self.tile_aabr.max.y + 2 {
            let pillar = prim(Primitive::Aabb(Aabb {
                min: site
                    .tile_wpos(Vec2::new(self.tile_aabr.min.x, y))
                    .with_z(self.alt),
                max: (site.tile_wpos(Vec2::new(self.tile_aabr.max.x + 1, y)) + Vec2::unit_y())
                    .with_z(self.alt + roof),
            }));
            pillars_x = prim(Primitive::Or(pillars_x, pillar));
        }
        let pillars = prim(Primitive::And(pillars_x, pillars_y));
        fill(Fill {
            prim: pillars,
            block: Block::new(BlockKind::Wood, Rgb::new(55, 25, 8)),
        });

        // For each storey...
        for i in 0..self.levels + 1 {
            let height = storey * i as i32;
            let window_height = storey - 3;

            // Windows x axis
            {
                let mut windows = prim(Primitive::Empty);
                for y in self.tile_aabr.min.y..self.tile_aabr.max.y {
                    let window = prim(Primitive::Aabb(Aabb {
                        min: (site.tile_wpos(Vec2::new(self.tile_aabr.min.x, y))
                            + Vec2::unit_y() * 2)
                            .with_z(self.alt + height + 2),
                        max: (site.tile_wpos(Vec2::new(self.tile_aabr.max.x, y + 1))
                            + Vec2::new(1, -1))
                        .with_z(self.alt + height + 2 + window_height),
                    }));
                    windows = prim(Primitive::Or(windows, window));
                }
                fill(Fill {
                    prim: prim(Primitive::And(walls, windows)),
                    block: Block::air(SpriteKind::Window1).with_ori(2).unwrap(),
                });
            }
            // Windows y axis
            {
                let mut windows = prim(Primitive::Empty);
                for x in self.tile_aabr.min.x..self.tile_aabr.max.x {
                    let window = prim(Primitive::Aabb(Aabb {
                        min: (site.tile_wpos(Vec2::new(x, self.tile_aabr.min.y))
                            + Vec2::unit_x() * 2)
                            .with_z(self.alt + height + 2),
                        max: (site.tile_wpos(Vec2::new(x + 1, self.tile_aabr.max.y))
                            + Vec2::new(-1, 1))
                        .with_z(self.alt + height + 2 + window_height),
                    }));
                    windows = prim(Primitive::Or(windows, window));
                }
                fill(Fill {
                    prim: prim(Primitive::And(walls, windows)),
                    block: Block::air(SpriteKind::Window1).with_ori(0).unwrap(),
                });
            }

            // Floor
            fill(Fill {
                prim: prim(Primitive::Aabb(Aabb {
                    min: self.bounds.min.with_z(self.alt + height),
                    max: (self.bounds.max + 1).with_z(self.alt + height + 1),
                })),
                block: Block::new(BlockKind::Rock, Rgb::new(89, 44, 14)),
            });
        }

        // Corner pillars
        // for &rpos in SQUARE_4.iter() {
        //     let pos = self.bounds.min + (self.bounds.max - self.bounds.min) * rpos;
        //     fill(Fill {
        //         prim: prim(Primitive::Aabb(Aabb {
        //             min: Vec3::new(pos.x - 1, pos.y - 1, self.alt - foundations),
        //             max: Vec3::new(pos.x + 1, pos.y + 1, self.alt + roof),
        //         })),
        //         block: Block::new(BlockKind::Wood, Rgb::new(89, 44, 14)),
        //     });
        // }

        let roof_lip = 2;
        let roof_height = (self.bounds.min - self.bounds.max)
            .map(|e| e.abs())
            .reduce_min()
            / 2
            + roof_lip
            + 1;

        // Roof
        fill(Fill {
            prim: prim(Primitive::Pyramid {
                aabb: Aabb {
                    min: (self.bounds.min - roof_lip).with_z(self.alt + roof),
                    max: (self.bounds.max + 1 + roof_lip).with_z(self.alt + roof + roof_height),
                },
                inset: roof_height,
            }),
            block: Block::new(BlockKind::Wood, self.roof_color),
        });

        // Foundations
        fill(Fill {
            prim: prim(Primitive::Aabb(Aabb {
                min: (self.bounds.min - 1).with_z(self.alt - foundations),
                max: (self.bounds.max + 2).with_z(self.alt + 1),
            })),
            block: Block::new(BlockKind::Rock, Rgb::new(31, 33, 32)),
        });
    }
}
