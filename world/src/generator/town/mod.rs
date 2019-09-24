mod util;
mod vol;

use super::{Generator, SpawnRules};
use crate::{
    block::block_from_structure,
    column::{ColumnGen, ColumnSample},
    util::Sampler,
    CONFIG,
};
use common::{
    assets,
    terrain::{Block, BlockKind, Structure},
    vol::{ReadVol, Vox, WriteVol},
};
use hashbrown::HashSet;
use lazy_static::lazy_static;
use rand::prelude::*;
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
                CellKind::Well => Some(Block::new(BlockKind::Normal, Rgb::broadcast(0))),
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
        _town: &'a TownState,
        _wpos: Vec2<i32>,
        sample: &ColumnSample,
    ) -> (f32, f32) {
        (sample.alt - 32.0, sample.alt + 75.0)
    }

    fn spawn_rules(&self, town: &'a TownState, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            trees: wpos.distance_squared(town.center.into()) > (town.radius + 32).pow(2),
        }
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
        let radius = rng.gen_range(18, 20) * 9;
        let size = Vec2::broadcast(radius * 2 / 9 - 2);

        if gen.get(center).map(|sample| sample.chaos).unwrap_or(0.0) > 0.35
            || gen.get(center).map(|sample| sample.alt).unwrap_or(0.0) < CONFIG.sea_level + 10.0
        {
            return None;
        }

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
        vol.gen_parks(rng, 3);
        vol.emplace_columns();
        let houses = vol.gen_houses(rng, 50);
        vol.gen_wells(rng, 5);
        vol.gen_walls(rng);
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
        limit: Option<usize>,
        mut opens: HashSet<Vec2<i32>>,
        mut f: impl FnMut(Vec2<i32>, &TownColumn) -> bool,
    ) -> HashSet<Vec2<i32>> {
        let mut closed = HashSet::new();

        while opens.len() > 0 {
            let mut new_opens = HashSet::new();

            'search: for open in opens.iter() {
                for i in -1..2 {
                    for j in -1..2 {
                        let pos = *open + Vec2::new(i, j);

                        if let Some(col) = self.col(pos) {
                            if !closed.contains(&pos) && !opens.contains(&pos) && f(pos, col) {
                                match limit {
                                    Some(limit)
                                        if limit
                                            <= new_opens.len() + closed.len() + opens.len() =>
                                    {
                                        break 'search
                                    }
                                    _ => {
                                        new_opens.insert(pos);
                                    }
                                }
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

        for _road in 0..n {
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

                let park = self.floodfill(Some(16), [start].iter().copied().collect(), |_, col| {
                    col.is_empty()
                });

                if park.len() < 4 {
                    continue;
                }

                for cell in park {
                    self.set_col_kind(cell, Some(ColumnKind::Internal));
                    let col = self.col(cell).unwrap();
                    let ground = col.ground;
                    let _ = self.set(Vec3::new(cell.x, cell.y, ground), CellKind::Park.into());
                }

                break;
            }
        }
    }

    fn gen_walls(&mut self, _rng: &mut impl Rng) {
        let mut outer = HashSet::new();
        for i in 0..self.size().x {
            outer.insert(Vec2::new(i, 0));
            outer.insert(Vec2::new(i, self.size().y - 1));
        }
        for j in 0..self.size().y {
            outer.insert(Vec2::new(0, j));
            outer.insert(Vec2::new(self.size().x - 1, j));
        }

        let outer = self.floodfill(None, outer, |_, col| col.is_empty());

        let mut walls = HashSet::new();
        let _inner = self.floodfill(
            None,
            [self.size() / 2].iter().copied().collect(),
            |pos, _| {
                if outer.contains(&pos) {
                    walls.insert(pos);
                    false
                } else {
                    true
                }
            },
        );

        while let Some(wall) = walls
            .iter()
            .filter(|pos| {
                let lateral_count = (0..4)
                    .filter(|i| walls.contains(&(**pos + util::dir(*i))))
                    .count();
                let max_quadrant_count = (0..4)
                    .map(|i| {
                        let units = util::unit(i);
                        (0..2)
                            .map(|i| (0..2).map(move |j| (i, j)))
                            .flatten()
                            .filter(|(i, j)| walls.contains(&(**pos + units.0 * *i + units.1 * *j)))
                            .count()
                    })
                    .max()
                    .unwrap();

                lateral_count < 2 || (lateral_count == 2 && max_quadrant_count == 4)
            })
            .next()
        {
            let wall = *wall;
            walls.remove(&wall);
        }

        for wall in walls.iter() {
            let col = self.col(*wall).unwrap();
            let ground = col.ground;
            for z in -1..3 {
                let _ = self.set(Vec3::new(wall.x, wall.y, ground + z), CellKind::Wall.into());
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
                    //Some(ColumnKind::External) => {}
                    Some(ColumnKind::Road) => {
                        for z in -1..1 {
                            let _ = self.set(Vec3::new(i, j, ground + z), CellKind::Road.into());
                        }
                    }
                }
            }
        }
    }

    fn gen_wells(&mut self, rng: &mut impl Rng, n: usize) {
        for _ in 0..n {
            if let Some(cell) = self.choose_cell(rng, |_, cell| {
                if let CellKind::Park = cell.kind {
                    true
                } else {
                    false
                }
            }) {
                self.set(cell, CellKind::Well.into());
            }
        }
    }

    fn gen_houses(&mut self, rng: &mut impl Rng, n: usize) -> Vec<House> {
        const ATTEMPTS: usize = 10;

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

                let mut energy = 2300;
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
                        && parent.z + dir.z <= entrance.z + 2
                    // Maximum house height
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
                    let _ = self.set(cell, CellKind::House(houses.len()).into());
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
                for z in self.col_range(Vec2::new(x, y)).unwrap().rev() {
                    let pos = Vec3::new(x, y, z);

                    // Remove foundations that don't have anything on top of them
                    if self.get(pos).unwrap().is_foundation()
                        && self
                            .get(pos + Vec3::unit_z())
                            .map(TownCell::is_space)
                            .unwrap_or(true)
                    {
                        let _ = self.set(pos, TownCell::empty());
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
                        let _ = self.set(
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
        CellKind::Well => Some(&WELL_MODULES),
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
            module(
                "human.window_corner_ground",
                [This, This, That, That, This, That],
            ),
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
            module(
                "human.window_corner_upstairs",
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
    pub static ref WELL_MODULES: Vec<(Arc<Structure>, [ModuleKind; 6])> = {
        use ModuleKind::*;
        vec![module("misc.well", [That; 6])]
    };
}

struct ModuleModel {
    near: u64,
    mask: u64,
    vol: Arc<Structure>,
}

#[derive(Copy, Clone)]
pub enum NearKind {
    This,
    That,
}

impl ModuleModel {
    pub fn generate_list(details: &[(&str, &[([i32; 3], NearKind)])]) -> Vec<Self> {
        unimplemented!()
    }
}
