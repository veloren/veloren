use super::*;
use crate::{Land, util::SQUARE_4};
use common::terrain::{Block, BlockKind, SpriteKind};
use vek::*;
use rand::prelude::*;

pub struct House {
    door_tile: Vec2<i32>,
    tile_aabr: Aabr<i32>,
    bounds: Aabr<i32>,
    alt: i32,
    levels: u32,
}

impl House {
    pub fn generate(land: &Land, rng: &mut impl Rng, site: &Site, door_tile: Vec2<i32>, tile_aabr: Aabr<i32>) -> Self {
        Self {
            door_tile,
            tile_aabr,
            bounds: Aabr {
                min: site.tile_wpos(tile_aabr.min),
                max: site.tile_wpos(tile_aabr.max),
            },
            alt: land.get_alt_approx(site.tile_center_wpos(door_tile)) as i32 + 2,
            levels: rng.gen_range(1..3),
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
        let storey = 6;
        let roof = storey * self.levels as i32;
        let foundations = 12;

        // Walls
        let outer = prim(Primitive::Aabb(Aabb {
            min: Vec3::new(self.bounds.min.x, self.bounds.min.y, self.alt - foundations),
            max: Vec3::new(self.bounds.max.x, self.bounds.max.y, self.alt + roof),
        }));
        let inner = prim(Primitive::Aabb(Aabb {
            min: Vec3::new(self.bounds.min.x + 1, self.bounds.min.y + 1, self.alt + 0),
            max: Vec3::new(self.bounds.max.x - 1, self.bounds.max.y - 1, self.alt + roof),
        }));
        let walls = prim(Primitive::Xor(outer, inner));
        fill(Fill {
            prim: walls,
            block: Block::new(BlockKind::Rock, Rgb::new(181, 170, 148)),
        });

        // wall pillars
        let mut pillars = prim(Primitive::Empty);
        for x in self.tile_aabr.min.x + 1..self.tile_aabr.max.x {
            let pillar = prim(Primitive::Aabb(Aabb {
                min: Vec3::from(site.tile_wpos(Vec2::new(x, self.tile_aabr.min.y))) + Vec3::unit_z() * self.alt,
                max: Vec3::from(site.tile_wpos(Vec2::new(x, self.tile_aabr.max.y)) + Vec2::unit_x()) + Vec3::unit_z() * (self.alt + roof),
            }));
            pillars = prim(Primitive::Or(pillars, pillar));
        }
        for y in self.tile_aabr.min.y + 1..self.tile_aabr.max.y {
            let pillar = prim(Primitive::Aabb(Aabb {
                min: Vec3::from(site.tile_wpos(Vec2::new(self.tile_aabr.min.x, y))) + Vec3::unit_z() * self.alt,
                max: Vec3::from(site.tile_wpos(Vec2::new(self.tile_aabr.max.x, y)) + Vec2::unit_y()) + Vec3::unit_z() * (self.alt + roof),
            }));
            pillars = prim(Primitive::Or(pillars, pillar));
        }
        fill(Fill {
            prim: prim(Primitive::And(walls, pillars)),
            block: Block::new(BlockKind::Wood, Rgb::new(89, 44, 14)),
        });

        // For each storey...
        for i in 0..self.levels + 1 {
            let height = storey * i as i32;

            // Windows x axis
            {
                let mut windows = prim(Primitive::Empty);
                for y in self.tile_aabr.min.y..self.tile_aabr.max.y {
                    let window = prim(Primitive::Aabb(Aabb {
                        min: Vec3::from(site.tile_wpos(Vec2::new(self.tile_aabr.min.x, y)) + Vec2::unit_y() * 2) + Vec3::unit_z() * (self.alt + height + 2),
                        max: Vec3::from(site.tile_wpos(Vec2::new(self.tile_aabr.max.x, y + 1)) - Vec2::unit_y() * 1) + Vec3::unit_z() * (self.alt + height + 5),
                    }));
                    windows = prim(Primitive::Or(windows, window));
                }
                fill(Fill {
                    prim: prim(Primitive::And(walls, windows)),
                    block: Block::air(SpriteKind::Window1)
                        .with_ori(2)
                        .unwrap(),
                });
            }
            // Windows y axis
            {
                let mut windows = prim(Primitive::Empty);
                for x in self.tile_aabr.min.x..self.tile_aabr.max.x {
                    let window = prim(Primitive::Aabb(Aabb {
                        min: Vec3::from(site.tile_wpos(Vec2::new(x, self.tile_aabr.min.y)) + Vec2::unit_x() * 2) + Vec3::unit_z() * (self.alt + height + 2),
                        max: Vec3::from(site.tile_wpos(Vec2::new(x + 1, self.tile_aabr.max.y)) - Vec2::unit_x() * 1) + Vec3::unit_z() * (self.alt + height + 5),
                    }));
                    windows = prim(Primitive::Or(windows, window));
                }
                fill(Fill {
                    prim: prim(Primitive::And(walls, windows)),
                    block: Block::air(SpriteKind::Window1)
                        .with_ori(0)
                        .unwrap(),
                });
            }

            // Floor
            fill(Fill {
                prim: prim(Primitive::Aabb(Aabb {
                    min: Vec3::new(self.bounds.min.x, self.bounds.min.y, self.alt + height + 0),
                    max: Vec3::new(self.bounds.max.x, self.bounds.max.y, self.alt + height + 1),
                })),
                block: Block::new(BlockKind::Rock, Rgb::new(89, 44, 14)),
            });
        }

        // Corner pillars
        for &rpos in SQUARE_4.iter() {
            let pos = self.bounds.min + (self.bounds.max - self.bounds.min) * rpos;
            fill(Fill {
                prim: prim(Primitive::Aabb(Aabb {
                    min: Vec3::new(pos.x - 1, pos.y - 1, self.alt - foundations),
                    max: Vec3::new(pos.x + 1, pos.y + 1, self.alt + roof),
                })),
                block: Block::new(BlockKind::Wood, Rgb::new(89, 44, 14)),
            });
        }


        let roof_lip = 3;
        let roof_height = (self.bounds.min - self.bounds.max).map(|e| e.abs()).reduce_min() / 2 + roof_lip;

        // Roof
        fill(Fill {
            prim: prim(Primitive::Pyramid {
                aabb: Aabb {
                    min: Vec3::new(self.bounds.min.x - roof_lip, self.bounds.min.y - roof_lip, self.alt + roof),
                    max: Vec3::new(self.bounds.max.x + roof_lip, self.bounds.max.y + roof_lip, self.alt + roof + roof_height),
                },
                inset: roof_height,
            }),
            block: Block::new(BlockKind::Wood, Rgb::new(21, 43, 48)),
        });

        // Foundations
        fill(Fill {
            prim: prim(Primitive::Aabb(Aabb {
                min: Vec3::new(self.bounds.min.x - 1, self.bounds.min.y - 1, self.alt - foundations),
                max: Vec3::new(self.bounds.max.x + 1, self.bounds.max.y + 1, self.alt + 1),
            })),
            block: Block::new(BlockKind::Rock, Rgb::new(31, 33, 32)),
        });
    }
}
