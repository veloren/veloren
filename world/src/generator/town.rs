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
use std::{collections::HashSet, sync::Arc};
use vek::*;

const CELL_SIZE: i32 = 11;

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
    MemberOf(usize),
    Market,
    Wall,
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

pub enum Property {
    House(Rgb<u8>),
}

pub struct TownState {
    pub center: Vec2<i32>,
    pub radius: i32,
    pub grid: Grid<TownCell>,
    pub properties: Vec<Property>,
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
        for _ in 0..25 {
            create_road();
        }

        let markets = rng.gen_range(1, 3);
        let mut create_market = || {
            for _ in 0..30 {
                let start_pos = Vec2::new(
                    grid.size().x / 4 + rng.gen_range(0, grid.size().x / 2),
                    grid.size().y / 4 + rng.gen_range(0, grid.size().y / 2),
                );

                if let Some(TownCell::Empty) = grid.get(start_pos) {
                    let mut cells = HashSet::new();
                    cells.insert(start_pos);

                    let mut energy = 1000;
                    while energy > 0 {
                        energy -= 1;

                        let pos = cells.iter().choose(rng).copied().unwrap();

                        let dir = {
                            let dirs = [-1, 0, 1, 0, -1];
                            let idx = rng.gen_range(0, 4);
                            Vec2::new(dirs[idx], dirs[idx + 1])
                        };

                        if cells.contains(&(pos + dir)) {
                            continue;
                        }

                        if let Some(TownCell::Empty) = grid.get(pos + dir) {
                            cells.insert(pos + dir);
                            energy -= 10;
                        }
                    }

                    if cells.len() >= 9 && cells.len() <= 25 {
                        for cell in cells.iter() {
                            grid.set(*cell, TownCell::Market);
                        }

                        break;
                    }
                }
            }
        };

        for _ in 0..markets {
            create_market();
        }

        let mut properties = Vec::new();
        let mut place_house = || 'house: loop {
            let start_pos = 'start_pos: {
                for _ in 0..50 {
                    let pos = Vec2::new(
                        rng.gen_range(4, grid.size().x - 4),
                        rng.gen_range(4, grid.size().y - 4),
                    );

                    let dirs = [-1, 0, 1, 0, -1];
                    let road_neighbours = (0..4)
                        .filter(|idx| {
                            grid.get(pos + Vec2::new(dirs[*idx], dirs[*idx + 1]))
                                .map(|cell| cell.is_road())
                                .unwrap_or(false)
                        })
                        .count();

                    if road_neighbours > 0 {
                        if let Some(TownCell::Empty) = grid.get(pos) {
                            break 'start_pos pos;
                        }
                    }
                }

                break 'house;
            };

            let mut cells = HashSet::new();
            cells.insert(start_pos);

            let mut growth_energy = rng.gen_range(50, 160);
            while growth_energy > 0 {
                growth_energy -= 1;

                let pos = cells.iter().choose(rng).copied().unwrap();

                let dir = {
                    let dirs = [-1, 0, 1, 0, -1];
                    let idx = rng.gen_range(0, 4);
                    Vec2::new(dirs[idx], dirs[idx + 1])
                };

                if cells.contains(&(pos + dir)) {
                    continue;
                }

                match grid.get(pos + dir) {
                    Some(TownCell::Empty) => {
                        growth_energy -= 10;
                        cells.insert(pos + dir);
                    }
                    _ => {}
                }
            }

            if cells.len() < 3 {
                break;
            }

            let property_idx = properties.len();

            for _ in 0..100 {
                let cell = match cells.iter().choose(rng) {
                    Some(cell) => *cell,
                    None => break,
                };

                let dirs = [-1, 0, 1, 0, -1];
                let neighbours = (0..4)
                    .filter(|idx| cells.contains(&(cell + Vec2::new(dirs[*idx], dirs[*idx + 1]))))
                    .count();

                if neighbours < 2 {
                    cells.remove(&cell);
                }
            }

            for cell in cells.iter() {
                grid.set(*cell, TownCell::MemberOf(property_idx));
            }

            if cells.len() > 0 {
                properties.push(Property::House(Rgb::new(rng.gen(), rng.gen(), rng.gen())));
            }

            break;
        };

        for _ in 0..40 {
            place_house();
        }

        /*
        let mut create_walls = || {
            for i in 0..grid.size().x {
                grid.set(Vec2::new(i, 0), TownCell::Wall);
                grid.set(Vec2::new(i, grid.size().y - 1), TownCell::Wall);
            }

            for j in 0..grid.size().y {
                grid.set(Vec2::new(0, j), TownCell::Wall);
                grid.set(Vec2::new(grid.size().x - 1, j), TownCell::Wall);
            }
        };
        */

        fn floodfill(
            mut opens: HashSet<Vec2<i32>>,
            grid: &Grid<TownCell>,
            mut f: impl FnMut(Vec2<i32>, &TownCell) -> bool,
        ) -> HashSet<Vec2<i32>> {
            let mut closed = HashSet::new();

            while opens.len() > 0 {
                let mut new_opens = HashSet::new();

                for open in opens.iter() {
                    for i in -1..2 {
                        for j in -1..2 {
                            let pos = *open + Vec2::new(i, j);

                            if let Some(cell) = grid.get(pos) {
                                if f(pos, cell) && !closed.contains(&pos) && !opens.contains(&pos) {
                                    new_opens.insert(pos);
                                }
                            }
                        }
                    }
                }

                closed = closed.union(&opens).copied().collect();
                opens = new_opens;
            }

            closed
        }

        let mut create_walls = || {
            let mut opens = HashSet::new();

            for i in 0..grid.size().x {
                opens.insert(Vec2::new(i, 0));
                opens.insert(Vec2::new(i, grid.size().y - 1));
            }

            for j in 0..grid.size().y {
                opens.insert(Vec2::new(0, j));
                opens.insert(Vec2::new(grid.size().x - 1, j));
            }

            let outer = floodfill(opens, &grid, |_, cell| {
                if let TownCell::Empty = cell {
                    true
                } else {
                    false
                }
            });

            let mut walls = HashSet::new();

            floodfill(
                [grid.size() / 2].iter().copied().collect(),
                &grid,
                |pos, _| {
                    if outer.contains(&pos) {
                        walls.insert(pos);
                        false
                    } else {
                        true
                    }
                },
            );

            for cell in walls.iter() {
                grid.set(*cell, TownCell::Wall);
            }
        };

        create_walls();

        let mut remove_extra_walls = || {
            for x in 0..grid.size().x {
                for y in 0..grid.size().y {
                    let pos = Vec2::new(x, y);
                    let mut wall_count = 0;
                    for i in 0..2 {
                        for j in 0..2 {
                            if let Some(TownCell::Wall) = grid.get(pos + Vec2::new(i, j)) {
                                wall_count += 1;
                            }
                        }
                    }

                    if wall_count == 4 {
                        grid.set(pos, TownCell::Empty);
                    }
                }
            }
        };

        remove_extra_walls();

        let mut variate_walls = || {
            for _ in 0..100 {
                let pos = Vec2::new(
                    rng.gen_range(0, grid.size().x - 1),
                    rng.gen_range(0, grid.size().y - 1),
                );

                let (mut wall_count, mut empty_count) = (0, 0);
                for i in 0..2 {
                    for j in 0..2 {
                        match grid.get(pos + Vec2::new(i, j)) {
                            Some(TownCell::Wall) => wall_count += 1,
                            Some(TownCell::Empty) => empty_count += 1,
                            _ => {}
                        }
                    }
                }

                // Swap!
                if (wall_count, empty_count) == (3, 1) {
                    let cell00 = grid.get(pos + Vec2::new(0, 0)).unwrap().clone();
                    let cell10 = grid.get(pos + Vec2::new(1, 0)).unwrap().clone();
                    let cell01 = grid.get(pos + Vec2::new(0, 1)).unwrap().clone();
                    let cell11 = grid.get(pos + Vec2::new(1, 1)).unwrap().clone();

                    grid.set(pos + Vec2::new(0, 0), cell11);
                    grid.set(pos + Vec2::new(1, 0), cell01);
                    grid.set(pos + Vec2::new(0, 1), cell10);
                    grid.set(pos + Vec2::new(1, 1), cell00);

                    break;
                }
            }
        };

        for _ in 0..100 {
            variate_walls();
        }

        /*
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
        */

        Some(Self {
            center,
            radius,
            grid,
            properties,
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
            TownCell::MemberOf(idx) => {
                if (wpos.z as f32) < height + 8.0 {
                    Some(Block::new(
                        BlockKind::Normal,
                        match town.properties[*idx] {
                            Property::House(col) => col,
                        },
                    ))
                } else {
                    None
                }
            }
            TownCell::Market => {
                if (wpos.z as f32) < height {
                    Some(Block::new(BlockKind::Normal, Rgb::new(255, 0, 0)))
                } else {
                    None
                }
            }
            TownCell::Wall => {
                if (wpos.z as f32) < height + 20.0 {
                    Some(Block::new(BlockKind::Normal, Rgb::new(100, 100, 100)))
                } else {
                    None
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
