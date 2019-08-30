mod util;
mod vol;

use super::{Generator, SpawnRules};
use crate::{
    block::block_from_structure,
    column::{ColumnGen, ColumnSample},
    sim::WorldSim,
    util::{seed_expan, Grid, Sampler, UnitChooser},
};
use common::{
    assets,
    terrain::{Block, BlockKind, Structure},
    vol::{ReadVol, Vox, WriteVol},
};
use hashbrown::HashSet;
use lazy_static::lazy_static;
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::{ops::Add, sync::Arc};
use vek::*;

use self::vol::{CellKind, ColumnKind, Module, TownCell, TownColumn, TownVol};

const CELL_SIZE: i32 = 9;
const CELL_HEIGHT: i32 = 9;

pub struct TownGen;

impl<'a> Sampler<'a> for TownGen {
    type Index = (&'a TownState, Vec3<i32>, &'a ColumnSample<'a>, f32);
    type Sample = Option<Block>;

    fn get(&self, (town, wpos, sample, height): Self::Index) -> Self::Sample {
        let cell_pos = (wpos - town.center)
            .map2(Vec3::new(CELL_SIZE, CELL_SIZE, CELL_HEIGHT), |e, sz| {
                e.div_euclid(sz)
            })
            .add(Vec3::from(town.vol.size() / 2));
        let inner_pos = (wpos - town.center)
            .map2(Vec3::new(CELL_SIZE, CELL_SIZE, CELL_HEIGHT), |e, sz| {
                e.rem_euclid(sz)
            });

        let cell = town.vol.get(cell_pos).ok()?;

        match (modules_from_kind(&cell.kind), &cell.module) {
            (Some(module_list), Some(module)) => {
                let transform = [
                    (Vec2::new(0, 0), Vec2::unit_x(), Vec2::unit_y()),
                    (Vec2::new(0, 1), -Vec2::unit_y(), Vec2::unit_x()),
                    (Vec2::new(1, 1), -Vec2::unit_x(), -Vec2::unit_y()),
                    (Vec2::new(1, 0), Vec2::unit_y(), -Vec2::unit_x()),
                ];

                module_list[module.vol_idx]
                    .0
                    .get(
                        Vec3::from(
                            transform[module.dir].0 * (CELL_SIZE - 1)
                                + transform[module.dir].1 * inner_pos.x
                                + transform[module.dir].2 * inner_pos.y,
                        ) + Vec3::unit_z() * inner_pos.z,
                    )
                    .ok()
                    .and_then(|sb| {
                        block_from_structure(*sb, BlockKind::Normal, wpos, wpos.into(), 0, sample)
                    })
            }
            _ => match cell.kind {
                CellKind::Empty => None,
                CellKind::Park => None,
                CellKind::Rock => Some(Block::new(BlockKind::Normal, Rgb::broadcast(100))),
                CellKind::Wall => Some(Block::new(BlockKind::Normal, Rgb::broadcast(175))),
                CellKind::Road => {
                    if (wpos.z as f32) < height - 1.0 {
                        Some(Block::new(
                            BlockKind::Normal,
                            Lerp::lerp(
                                Rgb::new(150.0, 140.0, 50.0),
                                Rgb::new(100.0, 95.0, 30.0),
                                sample.marble_small,
                            )
                            .map(|e| e as u8),
                        ))
                    } else {
                        Some(Block::empty())
                    }
                }
                CellKind::House(idx) => Some(Block::new(BlockKind::Normal, town.houses[idx].color)),
            },
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

    fn spawn_rules(&self, town: &'a TownState, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules { trees: false }
    }
}

struct House {
    color: Rgb<u8>,
}

pub struct TownState {
    center: Vec3<i32>,
    radius: i32,
    vol: TownVol,
    houses: Vec<House>,
}

impl TownState {
    pub fn generate(center: Vec2<i32>, gen: &mut ColumnGen, rng: &mut impl Rng) -> Option<Self> {
        let radius = rng.gen_range(12, 24) * 9;
        let size = Vec2::broadcast(radius * 2 / 9 - 2);

        let alt = gen.get(center).map(|sample| sample.alt).unwrap_or(0.0) as i32;

        let mut vol = TownVol::generate_from(
            size,
            |pos| {
                let wpos = center + (pos - size / 2) * CELL_SIZE + CELL_SIZE / 2;
                let rel_alt = gen.get(wpos).map(|sample| sample.alt).unwrap_or(0.0) as i32
                    + CELL_HEIGHT / 2
                    - alt;

                let col = TownColumn {
                    ground: rel_alt.div_euclid(CELL_HEIGHT),
                    kind: None,
                };

                (col.ground, col)
            },
            |(col, pos)| {
                if pos.z >= col.ground {
                    TownCell::empty()
                } else {
                    TownCell::from(CellKind::Rock)
                }
            },
        );

        // Generation passes
        vol.setup(rng);
        vol.gen_roads(rng, 30);
        //vol.gen_parks(rng, 8);
        vol.emplace_columns();
        let houses = vol.gen_houses(rng, 60);
        vol.gen_walls();
        vol.resolve_modules(rng);
        vol.cull_unused();

        Some(Self {
            center: Vec3::new(center.x, center.y, alt),
            radius,
            vol,
            houses,
        })
    }

    pub fn center(&self) -> Vec3<i32> {
        self.center
    }

    pub fn radius(&self) -> i32 {
        self.radius
    }
}

impl TownVol {
    fn floodfill(
        &self,
        mut opens: HashSet<Vec2<i32>>,
        mut f: impl FnMut(Vec2<i32>, &TownColumn) -> bool,
    ) -> HashSet<Vec2<i32>> {
        let mut closed = HashSet::new();

        while opens.len() > 0 {
            let mut new_opens = HashSet::new();

            for open in opens.iter() {
                for i in -1..2 {
                    for j in -1..2 {
                        let pos = *open + Vec2::new(i, j);

                        if let Some(col) = self.col(pos) {
                            if !closed.contains(&pos) && !opens.contains(&pos) && f(pos, col) {
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

    fn setup(&mut self, rng: &mut impl Rng) {
        // Place a single road tile at first
        let root_road = self
            .size()
            .map(|sz| (sz / 8) * 2 + rng.gen_range(0, sz / 4) * 2);
        self.set_col_kind(root_road, Some(ColumnKind::Road));
    }

    fn gen_roads(&mut self, rng: &mut impl Rng, n: usize) {
        const ATTEMPTS: usize = 5;

        let mut junctions = HashSet::new();
        junctions.insert(self.choose_column(rng, |_, col| col.is_road()).unwrap());

        for road in 0..n {
            for _ in 0..ATTEMPTS {
                let start = *junctions.iter().choose(rng).unwrap();
                //let start = self.choose_column(rng, |pos, col| pos.map(|e| e % 2 == 0).reduce_and() && col.is_road()).unwrap();
                let dir = util::gen_dir(rng);

                // If the direction we want to paint a path in is obstructed, abandon this attempt
                if self
                    .col(start + dir)
                    .map(|col| !col.is_empty())
                    .unwrap_or(true)
                {
                    continue;
                }

                // How long should this road be?
                let len = rng.gen_range(1, 10) * 2 + 1;

                // Paint the road until we hit an obstacle
                let success = (1..len)
                    .map(|i| start + dir * i)
                    .try_for_each(|pos| {
                        if self.col(pos).map(|col| col.is_empty()).unwrap_or(false) {
                            self.set_col_kind(pos, Some(ColumnKind::Road));
                            Ok(())
                        } else {
                            junctions.insert(pos);
                            Err(())
                        }
                    })
                    .is_ok();

                if success {
                    junctions.insert(start + dir * (len - 1));
                }

                break;
            }
        }
    }

    fn gen_parks(&mut self, rng: &mut impl Rng, n: usize) {
        const ATTEMPTS: usize = 5;

        for _ in 0..n {
            for _ in 0..ATTEMPTS {
                let start = self
                    .choose_column(rng, |pos, col| {
                        col.is_empty()
                            && (0..4).any(|i| {
                                self.col(pos + util::dir(i))
                                    .map(|col| col.is_road())
                                    .unwrap_or(false)
                            })
                    })
                    .unwrap();

                let mut energy = 50;
                let mut park = self.floodfill([start].iter().copied().collect(), |_, col| {
                    if col.is_empty() && energy > 0 {
                        energy -= 1;
                        true
                    } else {
                        false
                    }
                });

                if park.len() < 4 {
                    continue;
                }

                for cell in park {
                    self.set_col_kind(cell, Some(ColumnKind::Internal));
                    let col = self.col(cell).unwrap();
                    let ground = col.ground;
                    for z in 0..2 {
                        self.set(Vec3::new(cell.x, cell.y, ground + z), CellKind::Park.into());
                    }
                }

                break;
            }
        }
    }

    fn gen_walls(&mut self) {
        let mut outer = HashSet::new();
        for i in 0..self.size().x {
            outer.insert(Vec2::new(i, 0));
            outer.insert(Vec2::new(i, self.size().y - 1));
        }
        for j in 0..self.size().y {
            outer.insert(Vec2::new(0, j));
            outer.insert(Vec2::new(self.size().x - 1, j));
        }

        let mut outer = self.floodfill(outer, |_, col| col.is_empty());

        let mut walls = HashSet::new();
        let inner = self.floodfill([self.size() / 2].iter().copied().collect(), |pos, _| {
            if outer.contains(&pos) {
                walls.insert(pos);
                false
            } else {
                true
            }
        });

        while let Some(wall) = walls
            .iter()
            .filter(|pos| {
                (0..4)
                    .filter(|i| walls.contains(&(**pos + util::dir(*i))))
                    .count()
                    < 2
            })
            .next()
        {
            let wall = *wall;
            walls.remove(&wall);
        }

        for wall in walls.iter() {
            let col = self.col(*wall).unwrap();
            let ground = col.ground;
            for z in -1..2 {
                self.set(Vec3::new(wall.x, wall.y, ground + z), CellKind::Wall.into());
            }
        }
    }

    fn emplace_columns(&mut self) {
        for i in 0..self.size().x {
            for j in 0..self.size().y {
                let col = self.col(Vec2::new(i, j)).unwrap();
                let ground = col.ground;

                match col.kind {
                    None => {}
                    Some(ColumnKind::Internal) => {}
                    Some(ColumnKind::External) => {}
                    Some(ColumnKind::Road) => {
                        for z in -1..2 {
                            self.set(Vec3::new(i, j, ground + z), CellKind::Road.into());
                        }
                    }
                    _ => unimplemented!(),
                }
            }
        }
    }

    fn gen_houses(&mut self, rng: &mut impl Rng, n: usize) -> Vec<House> {
        const ATTEMPTS: usize = 20;

        let mut houses = Vec::new();
        for _ in 0..n {
            for _ in 0..ATTEMPTS {
                let entrance = {
                    let start = self.choose_cell(rng, |_, cell| cell.is_road()).unwrap();
                    let dir = Vec3::from(util::gen_dir(rng));

                    if self
                        .get(start + dir)
                        .map(|col| !col.is_empty())
                        .unwrap_or(true)
                        || self
                            .get(start + dir - Vec3::unit_z())
                            .map(|col| !col.is_foundation())
                            .unwrap_or(true)
                    {
                        continue;
                    } else {
                        start + dir
                    }
                };

                let mut cells: HashSet<_> = Some(entrance).into_iter().collect();

                let mut energy = 1000;
                while energy > 0 {
                    energy -= 1;

                    let parent = *cells.iter().choose(rng).unwrap();
                    let dir = util::UNITS_3D
                        .choose_weighted(rng, |pos| 1 + pos.z.max(0))
                        .unwrap();

                    if self
                        .get(parent + dir)
                        .map(|cell| cell.is_empty())
                        .unwrap_or(false)
                        && self
                            .get(parent + dir - Vec3::unit_z())
                            .map(|cell| {
                                cell.is_foundation()
                                    || cells.contains(&(parent + dir - Vec3::unit_z()))
                            })
                            .unwrap_or(false)
                    {
                        cells.insert(parent + dir);
                        energy -= 10;
                    }
                }

                // Remove cells that are too isolated
                loop {
                    let cells_copy = cells.clone();

                    let mut any_removed = false;
                    cells.retain(|pos| {
                        let neighbour_count = (0..6)
                            .filter(|i| {
                                let neighbour = pos + util::dir_3d(*i);
                                cells_copy.contains(&neighbour)
                            })
                            .count();

                        if neighbour_count < 3 {
                            any_removed = true;
                            false
                        } else {
                            true
                        }
                    });

                    if !any_removed {
                        break;
                    }
                }

                // Get rid of houses that are too small
                if cells.len() < 6 {
                    continue;
                }

                for cell in cells {
                    self.set(cell, CellKind::House(houses.len()).into());
                    self.set_col_kind(Vec2::from(cell), Some(ColumnKind::Internal));
                }

                houses.push(House {
                    color: Rgb::new(rng.gen(), rng.gen(), rng.gen()),
                });
            }
        }

        houses
    }

    fn cull_unused(&mut self) {
        for x in 0..self.size().x {
            for y in 0..self.size().y {
                for z in self.col_range(Vec2::new(x, y)).unwrap() {
                    let pos = Vec3::new(x, y, z);

                    // Remove foundations that don't have anything on top of them
                    if self.get(pos).unwrap().is_foundation()
                        && self
                            .get(pos + Vec3::unit_z())
                            .map(Vox::is_empty)
                            .unwrap_or(true)
                    {
                        self.set(pos, TownCell::empty());
                    }
                }
            }
        }
    }

    fn resolve_modules(&mut self, rng: &mut impl Rng) {
        fn classify(cell: &TownCell, this_cell: &TownCell) -> ModuleKind {
            match (&cell.kind, &this_cell.kind) {
                (CellKind::House(a), CellKind::House(b)) if a == b => ModuleKind::This,
                (CellKind::Wall, CellKind::Wall) => ModuleKind::This,
                _ => ModuleKind::That,
            }
        }

        for x in 0..self.size().x {
            for y in 0..self.size().y {
                for z in self.col_range(Vec2::new(x, y)).unwrap() {
                    let pos = Vec3::new(x, y, z);
                    let this_cell = if let Ok(this_cell) = self.get(pos) {
                        this_cell
                    } else {
                        continue;
                    };

                    let mut signature = [ModuleKind::That; 6];
                    for i in 0..6 {
                        signature[i] = self
                            .get(pos + util::dir_3d(i))
                            .map(|cell| classify(cell, this_cell))
                            .unwrap_or(ModuleKind::That);
                    }

                    let module_list = if let Some(modules) = modules_from_kind(&this_cell.kind) {
                        modules
                    } else {
                        continue;
                    };

                    let module = module_list
                        .iter()
                        .enumerate()
                        .filter_map(|(i, module)| {
                            let perms = [[0, 1, 2, 3], [3, 0, 1, 2], [2, 3, 0, 1], [1, 2, 3, 0]];

                            let mut rotated_signature = [ModuleKind::That; 6];
                            for (dir, perm) in perms.iter().enumerate() {
                                rotated_signature[perm[0]] = signature[0];
                                rotated_signature[perm[1]] = signature[1];
                                rotated_signature[perm[2]] = signature[2];
                                rotated_signature[perm[3]] = signature[3];
                                rotated_signature[4] = signature[4];
                                rotated_signature[5] = signature[5];

                                if &module.1[0..6] == &rotated_signature[0..6] {
                                    return Some(Module { vol_idx: i, dir });
                                }
                            }

                            None
                        })
                        .choose(rng);

                    if let Some(module) = module {
                        let kind = this_cell.kind.clone();
                        self.set(
                            pos,
                            TownCell {
                                kind,
                                module: Some(module),
                            },
                        );
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum ModuleKind {
    This,
    That,
}

fn module(name: &str, sig: [ModuleKind; 6]) -> (Arc<Structure>, [ModuleKind; 6]) {
    (
        assets::load(&format!("world.module.{}", name)).unwrap(),
        sig,
    )
}

fn modules_from_kind(kind: &CellKind) -> Option<&'static [(Arc<Structure>, [ModuleKind; 6])]> {
    match kind {
        CellKind::House(_) => Some(&HOUSE_MODULES),
        CellKind::Wall => Some(&WALL_MODULES),
        _ => None,
    }
}

lazy_static! {
    pub static ref HOUSE_MODULES: Vec<(Arc<Structure>, [ModuleKind; 6])> = {
        use ModuleKind::*;
        vec![
            module("human.floor_ground", [This, This, This, This, This, That]),
            module("human.stair_ground", [This, This, This, This, This, That]),
            module("human.corner_ground", [This, This, That, That, This, That]),
            module("human.wall_ground", [This, This, This, That, This, That]),
            module("human.door_ground", [This, This, This, That, This, That]),
            module("human.window_ground", [This, This, This, That, This, That]),
            module("human.floor_roof", [This, This, This, This, That, This]),
            module("human.corner_roof", [This, This, That, That, That, This]),
            module("human.chimney_roof", [This, This, That, That, That, This]),
            module("human.wall_roof", [This, This, This, That, That, This]),
            module("human.floor_upstairs", [This, This, This, This, This, This]),
            module(
                "human.balcony_upstairs",
                [This, This, This, This, This, This],
            ),
            module(
                "human.corner_upstairs",
                [This, This, That, That, This, This],
            ),
            module("human.wall_upstairs", [This, This, This, That, This, This]),
            module(
                "human.window_upstairs",
                [This, This, This, That, This, This],
            ),
        ]
    };
    pub static ref WALL_MODULES: Vec<(Arc<Structure>, [ModuleKind; 6])> = {
        use ModuleKind::*;
        vec![
            module("wall.edge_ground", [This, That, This, That, This, That]),
            module("wall.edge_mid", [This, That, This, That, This, This]),
            module("wall.edge_top", [This, That, This, That, That, This]),
            module("wall.corner_ground", [This, This, That, That, This, That]),
            module("wall.corner_mid", [This, This, That, That, This, This]),
            module("wall.corner_top", [This, This, That, That, That, This]),
            module("wall.end_top", [That, This, That, That, That, This]),
            module("wall.single_top", [That, That, That, That, That, This]),
        ]
    };
}

/*
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

        let radius = 200;

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
                let road_len = 2 + rng.gen_range(1, 5) * 2 + 1;

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
        for _ in 0..radius.pow(2) / 2000 {
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

        for _ in 0..radius.pow(2) / 1000 {
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
            //variate_walls();
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
*/
