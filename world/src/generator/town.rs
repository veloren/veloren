use super::Generator;
use crate::{
    column::ColumnSample,
    sim::WorldSim,
    util::{seed_expan, Grid, Sampler},
};
use common::terrain::{Block, BlockKind};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use vek::*;

const CELL_SIZE: i32 = 24;

#[derive(Clone)]
pub enum TownCell {
    Empty,
    Junction,
    Road,
    House,
}

pub struct TownState {
    pub center: Vec2<i32>,
    pub radius: i32,
    pub grid: Grid<TownCell>,
}

impl TownState {
    pub fn generate(center: Vec2<i32>, seed: u32, sim: &mut WorldSim) -> Option<Self> {
        let center_chunk = sim.get_wpos(center)?;

        // First, determine whether the location is even appropriate for a town
        if center_chunk.chaos > 0.5 || center_chunk.near_cliffs {
            return None;
        }

        let radius = 150;

        let mut grid = Grid::new(TownCell::Empty, Vec2::broadcast(radius * 2 / CELL_SIZE));

        grid.set(grid.size() / 2, TownCell::Junction);

        let mut create_road = || loop {
            let junctions = grid
                .iter()
                .filter(|(_, cell)| {
                    if let TownCell::Junction = cell {
                        true
                    } else {
                        false
                    }
                })
                .collect::<Vec<_>>();

            // Choose an existing junction for the road to start from
            let start_pos = junctions.choose(&mut sim.rng).unwrap().0; // Can't fail

            // Choose a random direction and length for the road
            let road_dir = {
                let dirs = [-1, 0, 1, 0, -1];
                let idx = sim.rng.gen_range(0, 4);
                Vec2::new(dirs[idx], dirs[idx + 1])
            };
            let road_len = sim.rng.gen_range(1, 4) * 2 + 1;

            // Make sure we aren't trying to create a road where a road already exists!
            match grid.get(start_pos + road_dir) {
                Some(TownCell::Empty) => {}
                _ => continue,
            }

            // Pave the road
            for i in 1..road_len {
                let cell_pos = start_pos + road_dir * i;
                if let Some(TownCell::Empty) = grid.get(cell_pos) {
                    grid.set(cell_pos, TownCell::Road);
                }
            }
            grid.set(start_pos + road_dir * road_len, TownCell::Junction);

            break;
        };

        for _ in 0..8 {
            create_road();
        }

        Some(Self {
            center,
            radius,
            grid,
        })
    }

    fn get_cell(&self, wpos: Vec2<i32>) -> &TownCell {
        self.grid
            .get((wpos - self.center + self.radius) / CELL_SIZE)
            .unwrap_or(&TownCell::Empty)
    }
}

pub struct TownGen;

impl<'a> Sampler<'a> for TownGen {
    type Index = (&'a TownState, Vec3<i32>, &'a ColumnSample<'a>);
    type Sample = Option<Block>;

    fn get(&self, (town, wpos, sample): Self::Index) -> Self::Sample {
        match town.get_cell(Vec2::from(wpos)) {
            TownCell::Road if wpos.z < sample.alt as i32 + 4 => {
                Some(Block::new(BlockKind::Normal, Rgb::new(255, 200, 150)))
            }
            TownCell::Junction if wpos.z < sample.alt as i32 + 4 => {
                Some(Block::new(BlockKind::Normal, Rgb::new(255, 200, 250)))
            }
            _ => None,
        }
    }
}

impl<'a> Generator<'a, TownState> for TownGen {
    fn get_z_limits(
        &self,
        town: &'a TownState,
        wpos: Vec2<i32>,
        sample: &ColumnSample,
    ) -> (f32, f32) {
        (sample.alt - 32.0, sample.alt + 64.0)
    }
}
