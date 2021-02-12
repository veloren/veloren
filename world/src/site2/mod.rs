mod plot;
mod tile;

use self::{
    plot::{Plot, PlotKind},
    tile::{TileGrid, Tile, TileKind, TILE_SIZE},
};
use crate::{
    site::SpawnRules,
    util::Grid,
    Canvas,
};
use common::{
    terrain::{Block, BlockKind, SpriteKind},
    store::{Id, Store},
};
use rand::prelude::*;
use vek::*;

#[derive(Default)]
pub struct Site {
    pub(crate) origin: Vec2<i32>,
    tiles: TileGrid,
    plots: Store<Plot>,
}

impl Site {
    pub fn radius(&self) -> f32 {
        (tile::MAX_BLOCK_RADIUS.pow(2) as f32 * 2.0).sqrt()
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        if wpos.distance_squared(self.origin) < 100i32.pow(2) {
            SpawnRules {
                trees: false,
                ..SpawnRules::default()
            }
        } else {
            SpawnRules::default()
        }
    }

    pub fn bounds(&self) -> Aabr<i32> {
        let radius = tile::MAX_BLOCK_RADIUS;
        Aabr {
            min: -Vec2::broadcast(radius as i32),
            max: Vec2::broadcast(radius as i32),
        }
    }

    pub fn plots(&self) -> impl Iterator<Item = &Plot> + '_ { self.plots.values() }

    pub fn create_plot(&mut self, plot: Plot) -> Id<Plot> { self.plots.insert(plot) }

    pub fn generate(rng: &mut impl Rng) -> Self {
        let mut site = Site::default();

        for i in 0..100 {
            let dir = Vec2::<f32>::zero()
                .map(|_| rng.gen_range(-1.0..1.0))
                .normalized();
            let search_pos = (dir * rng.gen_range(0.0f32..1.0).powf(2.0) * 24.0).map(|e| e as i32);

            site.tiles
                .find_near(search_pos, |_, tile| tile.is_empty())
                .and_then(|center| site.tiles.grow_aabr(center, 6..16, Extent2::new(2, 2)).ok())
                .map(|aabr| {
                    let tile = match i % 2 {
                        0 => TileKind::Farmland { seed: i },
                        _ => TileKind::Building { levels: 1 + i % 3 },
                    };

                    for x in 0..aabr.size().w {
                        for y in 0..aabr.size().h {
                            let pos = aabr.min + Vec2::new(x, y);
                            site.tiles.set(pos, Tile::free(tile.clone()));
                        }
                    }
                });
        }

        site
    }

    pub fn wpos_tile(&self, wpos2d: Vec2<i32>) -> &Tile {
        self.tiles.get((wpos2d - self.origin).map(|e| e.div_euclid(TILE_SIZE as i32)))
    }

    pub fn render(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        canvas.foreach_col(|canvas, wpos2d, col| {
            match self.wpos_tile(wpos2d).kind {
                TileKind::Farmland { seed } => (-4..5).for_each(|z| canvas.map(
                    Vec3::new(wpos2d.x, wpos2d.y, col.alt as i32 + z),
                    |b| if [
                        BlockKind::Grass,
                        BlockKind::Earth,
                        BlockKind::Sand,
                        BlockKind::Snow,
                        BlockKind::Rock,
                    ]
                    .contains(&b.kind()) {
                        Block::new(BlockKind::Earth, Rgb::new(40, 5 + (seed % 32) as u8, 0))
                    } else {
                        b.with_sprite(SpriteKind::Empty)
                    },
                )),
                TileKind::Building { levels } => (-4..7 * levels as i32).for_each(|z| canvas.set(
                    Vec3::new(wpos2d.x, wpos2d.y, col.alt as i32 + z),
                    Block::new(BlockKind::Wood, Rgb::new(180, 150, 120))
                )),
                _ => {},
            }
        });
    }
}

pub fn test_site() -> Site { Site::generate(&mut thread_rng()) }
