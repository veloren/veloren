use super::Generator;
use crate::{
    block::block_from_structure,
    column::{ColumnGen, ColumnSample},
    sim::WorldSim,
    util::{seed_expan, Grid, Sampler, UnitChooser},
};
use common::{
    assets,
    terrain::{Block, BlockKind, Structure},
    vol::{ReadVol, Vox},
};
use lazy_static::lazy_static;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::sync::Arc;
use vek::*;

const CELL_SIZE: i32 = 24;

static UNIT_CHOOSER: UnitChooser = UnitChooser::new(0x100F4E37);

lazy_static! {
    pub static ref HOUSES: Vec<Arc<Structure>> =
        vec![
            assets::load_map("world.structure.human.house_1", |s: Structure| s
                .with_center(Vec3::new(8, 10, 2)))
            .unwrap(),
        ];
    pub static ref BLACKSMITHS: Vec<Arc<Structure>> = vec![
        assets::load_map("world.structure.human.blacksmith", |s: Structure| s
            .with_center(Vec3::new(16, 19, 9)))
        .unwrap(),
        assets::load_map("world.structure.human.mage_tower", |s: Structure| s
            .with_center(Vec3::new(13, 13, 4)))
        .unwrap(),
    ];
    pub static ref TOWNHALLS: Vec<Arc<Structure>> = vec![
        assets::load_map("world.structure.human.town_hall_spire", |s: Structure| s
            .with_center(Vec3::new(16, 16, 2)))
        .unwrap(),
        assets::load_map("world.structure.human.stables_1", |s: Structure| s
            .with_center(Vec3::new(16, 23, 2)))
        .unwrap(),
    ];
}

#[derive(Clone)]
pub enum Building {
    House,
    Blacksmith,
    TownHall,
}

#[derive(Clone)]
pub enum TownCell {
    Empty,
    Junction,
    Street,
    Building {
        kind: Building,
        wpos: Vec3<i32>,
        size: Vec2<i32>,
        units: (Vec2<i32>, Vec2<i32>),
        seed: u32,
    },
    PartOf(Vec2<i32>),
}

impl TownCell {
    fn is_road(&self) -> bool {
        match self {
            TownCell::Junction => true,
            TownCell::Street => true,
            _ => false,
        }
    }

    fn mergeable(&self) -> bool {
        match self {
            TownCell::Empty => true,
            TownCell::Building { size, .. } if *size == Vec2::one() => true,
            _ => false,
        }
    }
}

pub struct TownState {
    pub center: Vec2<i32>,
    pub radius: i32,
    pub grid: Grid<TownCell>,
}

impl TownState {
    pub fn generate(
        center: Vec2<i32>,
        seed: u32,
        gen: &mut ColumnGen,
        rng: &mut impl Rng,
    ) -> Option<Self> {
        let center_chunk = gen.sim.get_wpos(center)?;

        // First, determine whether the location is even appropriate for a town
        if center_chunk.chaos > 0.7 || center_chunk.near_cliffs {
            return None;
        }

        let radius = 192;

        let mut grid = Grid::new(
            TownCell::Empty,
            Vec2::broadcast(radius * 3 / (CELL_SIZE * 2)),
        );

        grid.set(grid.size() / 2, TownCell::Junction);

        let mut create_road = || {
            for _ in 0..10 {
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
                let start_pos = junctions.choose(rng).unwrap().0; // Can't fail

                // Choose a random direction and length for the road
                let road_dir = {
                    let dirs = [-1, 0, 1, 0, -1];
                    let idx = rng.gen_range(0, 4);
                    Vec2::new(dirs[idx], dirs[idx + 1])
                };
                let road_len = 2 + rng.gen_range(1, 3) * 2 + 1;

                // Make sure we aren't trying to create a road where a road already exists!
                match grid.get(start_pos + road_dir) {
                    Some(TownCell::Empty) => {}
                    _ => continue,
                }

                // Pave the road
                for i in 1..road_len {
                    let cell_pos = start_pos + road_dir * i;
                    if let Some(TownCell::Empty) = grid.get(cell_pos) {
                        grid.set(
                            cell_pos,
                            if i == road_len - 1 {
                                TownCell::Junction
                            } else {
                                TownCell::Street
                            },
                        );
                    } else {
                        grid.set(cell_pos, TownCell::Junction);
                        break;
                    }
                }

                break;
            }
        };

        // Create roads
        for _ in 0..12 {
            create_road();
        }

        // Place houses
        for x in 0..grid.size().x {
            for y in 0..grid.size().y {
                let pos = Vec2::new(x, y);
                let wpos = center + (pos - grid.size() / 2) * CELL_SIZE + CELL_SIZE / 2;

                // Is this cell near a road?
                let near_road = 'near_road: {
                    let dirs = [-1, 0, 1, 0];
                    let offs = rng.gen_range(0, 4);
                    for i in 0..4 {
                        let dir = Vec2::new(dirs[(offs + i) % 4], dirs[(offs + i + 1) % 4]);
                        if grid.get(pos + dir).unwrap_or(&TownCell::Empty).is_road() {
                            break 'near_road Some(dir);
                        }
                    }
                    None
                };

                match (near_road, grid.get_mut(pos)) {
                    (Some(dir), Some(cell @ TownCell::Empty)) if rng.gen_range(0, 6) > 0 => {
                        let alt = gen.get(wpos).map(|sample| sample.alt).unwrap_or(0.0) as i32;

                        *cell = TownCell::Building {
                            kind: Building::House,
                            wpos: Vec3::new(wpos.x, wpos.y, alt),
                            size: Vec2::one(),
                            units: (
                                Vec2::new(dir.y, dir.x) * (rng.gen_range(0, 1) * 2 - 1),
                                -dir,
                            ),
                            seed: rng.gen(),
                        };
                    }
                    _ => {}
                }
            }
        }

        // Merge buildings
        for x in 0..grid.size().x {
            for y in 0..grid.size().y {
                let pos = Vec2::new(x, y);
                for offx in -1..1 {
                    for offy in -1..1 {
                        if grid
                            .iter_area(pos + Vec2::new(offx, offy), Vec2::broadcast(2))
                            .any(|cell| cell.map(|(_, cell)| !cell.mergeable()).unwrap_or(true))
                        {
                            continue;
                        }

                        match grid.get_mut(pos) {
                            Some(TownCell::Building {
                                kind, wpos, size, ..
                            }) => {
                                *kind = if rng.gen() {
                                    Building::Blacksmith
                                } else {
                                    Building::TownHall
                                };
                                *wpos += Vec3::new(CELL_SIZE / 2, CELL_SIZE / 2, 0)
                                    * (Vec2::new(offx, offy) * 2 + 1);
                                *size = Vec2::broadcast(2);
                            }
                            _ => continue,
                        }

                        for i in 0..2 {
                            for j in 0..2 {
                                let p = Vec2::new(i + offx, j + offy);
                                if pos + p != pos {
                                    grid.set(pos + p, TownCell::PartOf(pos));
                                }
                            }
                        }
                    }
                }
            }
        }

        Some(Self {
            center,
            radius,
            grid,
        })
    }

    fn get_cell(&self, wpos: Vec2<i32>) -> &TownCell {
        let rpos = wpos - self.center;
        match self
            .grid
            .get(rpos.map(|e| e.div_euclid(CELL_SIZE)) + self.grid.size() / 2)
            .unwrap_or(&TownCell::Empty)
        {
            TownCell::PartOf(pos) => self.grid.get(*pos).unwrap(),
            cell => cell,
        }
    }
}

pub struct TownGen;

impl<'a> Sampler<'a> for TownGen {
    type Index = (&'a TownState, Vec3<i32>, &'a ColumnSample<'a>, f32);
    type Sample = Option<Block>;

    fn get(&self, (town, wpos, sample, height): Self::Index) -> Self::Sample {
        match town.get_cell(Vec2::from(wpos)) {
            cell if cell.is_road() => {
                if (wpos.z as f32) < height - 1.0 {
                    Some(Block::new(
                        BlockKind::Normal,
                        Lerp::lerp(
                            Rgb::new(150.0, 120.0, 50.0),
                            Rgb::new(100.0, 70.0, 20.0),
                            sample.marble_small,
                        )
                        .map(|e| e as u8),
                    ))
                } else {
                    Some(Block::empty())
                }
            }
            TownCell::Building {
                kind,
                wpos: building_wpos,
                units,
                seed,
                ..
            } => {
                let rpos = wpos - building_wpos;
                let volumes: &'static [_] = match kind {
                    Building::House => &HOUSES,
                    Building::Blacksmith => &BLACKSMITHS,
                    Building::TownHall => &TOWNHALLS,
                };
                volumes[*seed as usize % volumes.len()]
                    .get(
                        Vec3::from(units.0) * rpos.x
                            + Vec3::from(units.1) * rpos.y
                            + Vec3::unit_z() * rpos.z,
                    )
                    .ok()
                    .and_then(|sb| {
                        block_from_structure(*sb, BlockKind::Normal, wpos, wpos.into(), 0, sample)
                    })
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
        (sample.alt - 32.0, sample.alt + 75.0)
    }
}
