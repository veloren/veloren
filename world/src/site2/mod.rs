mod tile;
mod plot;

use vek::*;
use common::store::{Store, Id};
use crate::util::Grid;
use self::{
    tile::TileGrid,
    plot::{Plot, PlotKind},
};
use rand::prelude::*;

#[derive(Default)]
pub struct Site {
    tiles: TileGrid,
    plots: Store<Plot>,
}

impl Site {
    pub fn bounds(&self) -> Aabr<i32> {
        let radius = tile::MAX_BLOCK_RADIUS;
        Aabr {
            min: -Vec2::broadcast(radius as i32),
            max: Vec2::broadcast(radius as i32),
        }
    }

    pub fn plots(&self) -> impl Iterator<Item=&Plot> + '_ {
        self.plots.values()
    }

    pub fn create_plot(&mut self, plot: Plot) -> Id<Plot> {
        self.plots.insert(plot)
    }

    pub fn generate(rng: &mut impl Rng) -> Self {
        let mut site = Site::default();

        for i in 0..10 {
            let dir = Vec2::<f32>::zero().map(|_| rng.gen_range(-1.0..1.0)).normalized();
            let search_pos = (dir * 32.0).map(|e| e as i32);

            site.tiles
                .find_near(search_pos, |tile| tile.is_empty())
                .map(|center| {
                    // TODO
                });
        }

        site
    }
}

pub fn test_site() -> Site {
    Site::generate(&mut thread_rng())
}
