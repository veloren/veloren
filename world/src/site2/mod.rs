mod plot;
mod tile;

use self::{
    plot::{Plot, PlotKind},
    tile::TileGrid,
};
use crate::{
    site::SpawnRules,
    util::Grid,
    Canvas,
};
use common::store::{Id, Store};
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

        for i in 0..10 {
            let dir = Vec2::<f32>::zero()
                .map(|_| rng.gen_range(-1.0..1.0))
                .normalized();
            let search_pos = (dir * 32.0).map(|e| e as i32);

            site.tiles
                .find_near(search_pos, |tile| tile.is_empty())
                .map(|center| {
                    // TODO
                });
        }

        site
    }

    pub fn render(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        // TODO
    }
}

pub fn test_site() -> Site { Site::generate(&mut thread_rng()) }
