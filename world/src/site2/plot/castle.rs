use super::*;
use crate::{util::SQUARE_4, Land};
use common::terrain::{Block, BlockKind, SpriteKind};
use rand::prelude::*;
use vek::*;

pub struct Castle {
    entrance_tile: Vec2<i32>,
    tile_aabr: Aabr<i32>,
    bounds: Aabr<i32>,
    alt: i32,
}

impl Castle {
    pub fn generate(
        land: &Land,
        rng: &mut impl Rng,
        site: &Site,
        entrance_tile: Vec2<i32>,
        tile_aabr: Aabr<i32>,
    ) -> Self {
        Self {
            entrance_tile,
            tile_aabr,
            bounds: Aabr {
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
        let thickness = 12;
        let parapet_height = 2;
        let parapet_width = 1;
        let downwards = 40;
        for x in 0..self.tile_aabr.size().w {
            for y in 0..self.tile_aabr.size().h {
                let tile_pos = self.tile_aabr.min + Vec2::new(x, y);
                let wpos_center = site.tile_center_wpos(tile_pos);
                let wpos = site.tile_wpos(tile_pos);
                match site.tiles.get(tile_pos).kind.clone() {
                    TileKind::Wall => {
                        let ori = if x == 0 || x == self.tile_aabr.size().w - 1 {
                            Vec2::new(1, 0)
                        } else {
                            Vec2::new(0, 1)
                        };
                        let wall = prim(Primitive::Aabb(Aabb {
                            min: wpos.with_z(self.alt),
                            max: (wpos + 7).with_z(self.alt + wall_height + parapet_height),
                        }));
                        let cut_path = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(
                                wpos.x + parapet_width * ori.x,
                                wpos.y + parapet_width * ori.y,
                                self.alt + wall_height,
                            ),
                            max: Vec3::new(
                                wpos.x + (7 - parapet_width) * ori.x + 7 * ori.y,
                                wpos.y + (7 - parapet_width) * ori.y + 7 * ori.x,
                                self.alt + wall_height + parapet_height,
                            ),
                        }));
                        let cut_sides1 = prim(Primitive::Aabb(Aabb {
                            min: Vec3::new(wpos.x, wpos.y, self.alt + wall_height + 1),
                            max: Vec3::new(
                                wpos.x + 7 * ori.x + ori.y,
                                wpos.y + 7 * ori.y + ori.x,
                                self.alt + wall_height + parapet_height,
                            ),
                        }));
                        fill(
                            prim(Primitive::Xor(wall, cut_path)),
                            Fill::Block(Block::new(BlockKind::Rock, Rgb::new(33, 33, 33))),
                        );
                        fill(cut_sides1, Fill::Block(Block::air(SpriteKind::Empty)));
                    },
                    _ => {
                        fill(
                            prim(Primitive::Aabb(Aabb {
                                min: wpos_center.with_z(self.alt + 9),
                                max: (wpos_center + 1).with_z(self.alt + 10),
                            })),
                            Fill::Block(Block::new(BlockKind::Rock, Rgb::new(255, 255, 255))),
                        );
                    },
                }
            }
        }
    }
}
