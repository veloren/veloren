use super::*;
use crate::{Land, util::SQUARE_4};
use common::terrain::{Block, BlockKind};
use vek::*;
use rand::prelude::*;

pub struct House {
    door_tile: Vec2<i32>,
    bounds: Aabr<i32>,
    alt: i32,
    levels: u32,
}

impl House {
    pub fn generate(land: &Land, rng: &mut impl Rng, site: &Site, door_tile: Vec2<i32>, tile_aabr: Aabr<i32>) -> Self {
        Self {
            door_tile,
            bounds: Aabr {
                min: site.tile_wpos(tile_aabr.min),
                max: site.tile_wpos(tile_aabr.max),
            },
            alt: land.get_alt_approx(site.tile_center_wpos(door_tile)) as i32,
            levels: rng.gen_range(1..3),
        }
    }
}

impl Structure for House {
    fn render<F: FnMut(Primitive) -> Id<Primitive>, G: FnMut(Fill)>(
        &self,
        mut emit_prim: F,
        mut emit_fill: G,
    ) {
        let storey = 8;
        let roof = storey * self.levels as i32;
        let foundations = 8;

        // Walls
        let wall = emit_prim(Primitive::Aabb(Aabb {
            min: Vec3::new(self.bounds.min.x, self.bounds.min.y, self.alt - foundations),
            max: Vec3::new(self.bounds.max.x, self.bounds.max.y, self.alt + roof),
        }));
        let inner = emit_prim(Primitive::Aabb(Aabb {
            min: Vec3::new(self.bounds.min.x + 1, self.bounds.min.y + 1, self.alt + 0),
            max: Vec3::new(self.bounds.max.x - 1, self.bounds.max.y - 1, self.alt + roof),
        }));
        emit_fill(Fill {
            prim: emit_prim(Primitive::Xor(wall, inner)),
            block: Block::new(BlockKind::Rock, Rgb::new(181, 170, 148)),
        });

        // Floor
        for i in 0..self.levels + 1 {
            let height = storey * i as i32;
            emit_fill(Fill {
                prim: emit_prim(Primitive::Aabb(Aabb {
                    min: Vec3::new(self.bounds.min.x, self.bounds.min.y, self.alt + height + 0),
                    max: Vec3::new(self.bounds.max.x, self.bounds.max.y, self.alt + height + 1),
                })),
                block: Block::new(BlockKind::Rock, Rgb::new(89, 44, 14)),
            });
        }

        // Corner pillars
        for &rpos in SQUARE_4.iter() {
            let pos = self.bounds.min + (self.bounds.max - self.bounds.min) * rpos;
            emit_fill(Fill {
                prim: emit_prim(Primitive::Aabb(Aabb {
                    min: Vec3::new(pos.x - 1, pos.y - 1, self.alt - foundations),
                    max: Vec3::new(pos.x + 1, pos.y + 1, self.alt + roof),
                })),
                block: Block::new(BlockKind::Wood, Rgb::new(89, 44, 14)),
            });
        }


        let roof_lip = 3;
        let roof_height = (self.bounds.min - self.bounds.max).map(|e| e.abs()).reduce_min() / 2 + roof_lip;

        // Roof
        emit_fill(Fill {
            prim: emit_prim(Primitive::Pyramid {
                aabb: Aabb {
                    min: Vec3::new(self.bounds.min.x - roof_lip, self.bounds.min.y - roof_lip, self.alt + roof),
                    max: Vec3::new(self.bounds.max.x + roof_lip, self.bounds.max.y + roof_lip, self.alt + roof + roof_height),
                },
                inset: roof_height,
            }),
            block: Block::new(BlockKind::Wood, Rgb::new(21, 43, 48)),
        });

        // Foundations
        emit_fill(Fill {
            prim: emit_prim(Primitive::Aabb(Aabb {
                min: Vec3::new(self.bounds.min.x - 1, self.bounds.min.y - 1, self.alt - foundations),
                max: Vec3::new(self.bounds.max.x + 1, self.bounds.max.y + 1, self.alt + 1),
            })),
            block: Block::new(BlockKind::Rock, Rgb::new(31, 33, 32)),
        });
    }
}
