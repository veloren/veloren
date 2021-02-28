use super::*;
use crate::Land;
use common::terrain::{Block, BlockKind};
use vek::*;
use rand::prelude::*;

pub struct House {
    bounds: Aabr<i32>,
    alt: i32,
}

impl House {
    pub fn generate(land: &Land, rng: &mut impl Rng, site: &Site, tile_aabr: Aabr<i32>) -> Self {
        Self {
            bounds: Aabr {
                min: site.tile_wpos(tile_aabr.min),
                max: site.tile_wpos(tile_aabr.max),
            },
            alt: land.get_alt_approx(site.tile_center_wpos(tile_aabr.center())) as i32,
        }
    }
}

impl Structure for House {
    fn render<F: FnMut(Primitive) -> Id<Primitive>, G: FnMut(Fill)>(
        &self,
        mut emit_prim: F,
        mut emit_fill: G,
    ) {
        let wall = emit_prim(Primitive::Aabb(Aabb {
            min: Vec3::new(self.bounds.min.x, self.bounds.min.y, self.alt - 8),
            max: Vec3::new(self.bounds.max.x, self.bounds.max.y, self.alt + 16),
        }));
        let inner = emit_prim(Primitive::Aabb(Aabb {
            min: Vec3::new(self.bounds.min.x + 1, self.bounds.min.y + 1, self.alt - 8),
            max: Vec3::new(self.bounds.max.x - 1, self.bounds.max.y - 1, self.alt + 16),
        }));
        emit_fill(Fill {
            prim: emit_prim(Primitive::Xor(wall, inner)),
            block: Block::new(BlockKind::Rock, Rgb::new(150, 50, 10)),
        });
    }
}
