use crate::util::{Sampler, StructureGen2d};
use common::{
    astar::Astar,
    path::Path,
    spiral::Spiral2d,
    terrain::{Block, BlockKind},
};
use hashbrown::{HashMap, HashSet};
use rand::prelude::*;
use std::{collections::VecDeque, f32, marker::PhantomData};
use vek::*;

pub fn gradient(line: [Vec2<f32>; 2]) -> f32 {
    let r = (line[0].y - line[1].y) / (line[0].x - line[1].x);
    if r.is_nan() { 100000.0 } else { r }
}

pub fn intersect(a: [Vec2<f32>; 2], b: [Vec2<f32>; 2]) -> Option<Vec2<f32>> {
    let ma = gradient(a);
    let mb = gradient(b);

    let ca = a[0].y - ma * a[0].x;
    let cb = b[0].y - mb * b[0].x;

    if (ma - mb).abs() < 0.0001 || (ca - cb).abs() < 0.0001 {
        None
    } else {
        let x = (cb - ca) / (ma - mb);
        let y = ma * x + ca;

        Some(Vec2::new(x, y))
    }
}

pub fn dist_to_line(line: [Vec2<f32>; 2], p: Vec2<f32>) -> f32 {
    let lsq = line[0].distance_squared(line[1]);

    if lsq == 0.0 {
        line[0].distance(p)
    } else {
        let t = ((p - line[0]).dot(line[1] - line[0]) / lsq)
            .max(0.0)
            .min(1.0);
        p.distance(line[0] + (line[1] - line[0]) * t)
    }
}

pub fn center_of(p: [Vec2<f32>; 3]) -> Vec2<f32> {
    let ma = -1.0 / gradient([p[0], p[1]]);
    let mb = -1.0 / gradient([p[1], p[2]]);

    let pa = (p[0] + p[1]) * 0.5;
    let pb = (p[1] + p[2]) * 0.5;

    let ca = pa.y - ma * pa.x;
    let cb = pb.y - mb * pb.x;

    let x = (cb - ca) / (ma - mb);
    let y = ma * x + ca;

    Vec2::new(x, y)
}

const AREA_SIZE: u32 = 64;

fn to_tile(e: i32) -> i32 { ((e as f32).div_euclid(AREA_SIZE as f32)).floor() as i32 }

pub enum StructureKind {
    House,
}

pub struct Structure {
    kind: StructureKind,
    bounds: Aabr<i32>,
}

pub struct Settlement {
    origin: Vec2<i32>,
    land: Land,
    farms: Store<Farm>,
    structures: Vec<Structure>,
    town: Option<Town>,
}

pub struct Town {
    base_tile: Vec2<i32>,
}

pub struct Farm {
    base_tile: Vec2<i32>,
}

impl Settlement {
    pub fn generate(wpos: Vec2<i32>, rng: &mut impl Rng) -> Self {
        let mut this = Self {
            origin: wpos,
            land: Land::new(rng),
            farms: Store::default(),
            structures: Vec::new(),
            town: None,
        };

        //this.place_river(rng);

        this.place_farms(rng);
        this.place_town(rng);
        this.place_paths(rng);

        this
    }

    pub fn place_river(&mut self, rng: &mut impl Rng) {
        let river_dir = Vec2::new(rng.gen::<f32>() - 0.5, rng.gen::<f32>() - 0.5).normalized();
        let radius = 500.0 + rng.gen::<f32>().powf(2.0) * 1000.0;
        let river = self.land.new_plot(Plot::Water);
        let river_offs = Vec2::new(rng.gen_range(-3, 4), rng.gen_range(-3, 4));

        for x in (0..100).map(|e| e as f32 / 100.0) {
            let theta0 = x as f32 * f32::consts::PI * 2.0;
            let theta1 = (x + 0.01) as f32 * f32::consts::PI * 2.0;

            let pos0 = (river_dir * radius + Vec2::new(theta0.sin(), theta0.cos()) * radius)
                .map(|e| e.floor() as i32)
                .map(to_tile)
                + river_offs;
            let pos1 = (river_dir * radius + Vec2::new(theta1.sin(), theta1.cos()) * radius)
                .map(|e| e.floor() as i32)
                .map(to_tile)
                + river_offs;

            if pos0.magnitude_squared() > 15i32.pow(2) {
                continue;
            }

            if let Some(path) = self.land.find_path(pos0, pos1, |_, _| 1.0) {
                for pos in path.iter().copied() {
                    self.land.set(pos, river);
                }
            }
        }
    }

    pub fn place_paths(&mut self, rng: &mut impl Rng) {
        let mut dir = Vec2::zero();
        for _ in 0..6 {
            dir = (Vec2::new(rng.gen::<f32>() - 0.5, rng.gen::<f32>() - 0.5) * 2.0 - dir)
                .try_normalized()
                .unwrap_or(Vec2::zero());
            let origin = dir.map(|e| (e * 20.0) as i32);
            let origin = self
                .land
                .find_tile_near(origin, |plot| match plot {
                    Some(&Plot::Field { .. }) => true,
                    _ => false,
                })
                .unwrap();

            if let Some(path) = self.town.as_ref().and_then(|town| {
                self.land
                    .find_path(origin, town.base_tile, |from, to| match (from, to) {
                        (_, Some(b)) if self.land.plot(b.plot) == &Plot::Dirt => 0.0,
                        (_, Some(b)) if self.land.plot(b.plot) == &Plot::Water => 20.0,
                        (Some(a), Some(b)) if a.contains(WayKind::Wall) => {
                            if b.contains(WayKind::Wall) {
                                1000.0
                            } else {
                                10.0
                            }
                        },
                        (Some(_), Some(_)) => 1.0,
                        _ => 1000.0,
                    })
            }) {
                let path = path.iter().copied().collect::<Vec<_>>();
                self.land.write_path(&path, WayKind::Path, |_| true, true);
            }
        }
    }

    pub fn place_town(&mut self, rng: &mut impl Rng) {
        let mut origin = Vec2::new(rng.gen_range(-2, 3), rng.gen_range(-2, 3));

        let town = self.land.new_plot(Plot::Town);
        for i in 0..6 {
            if let Some(base_tile) = self.land.find_tile_near(origin, |plot| match plot {
                Some(Plot::Field { .. }) => true,
                Some(Plot::Dirt) => true,
                _ => false,
            }) {
                self.land.set(base_tile, town);

                if i == 0 {
                    /*
                    for dir in CARDINALS.iter() {
                        self.land.set(base_tile + *dir, town);
                    }
                    */

                    self.town = Some(Town { base_tile });
                    origin = base_tile;
                }
            }
        }

        // Boundary wall
        let spokes = CARDINALS
            .iter()
            .filter_map(|dir| {
                self.land.find_tile_dir(origin, *dir, |plot| match plot {
                    Some(Plot::Town) => false,
                    _ => true,
                })
            })
            .collect::<Vec<_>>();
        let mut wall_path = Vec::new();
        for i in 0..spokes.len() {
            self.land
                .find_path(spokes[i], spokes[(i + 1) % spokes.len()], |_, to| match to
                    .map(|to| self.land.plot(to.plot))
                {
                    Some(Plot::Town) => 1000.0,
                    _ => 1.0,
                })
                .map(|path| wall_path.extend(path.iter().copied()));
        }
        let grass = self.land.new_plot(Plot::Grass);
        let buildable = |plot: &Plot| match plot {
            Plot::Water => false,
            _ => true,
        };
        for pos in wall_path.iter() {
            if self.land.tile_at(*pos).is_none() {
                self.land.set(*pos, grass);
            }
            if self.land.plot_at(*pos).copied().filter(buildable).is_some() {
                self.land
                    .tile_at_mut(*pos)
                    .map(|tile| tile.tower = Some(Tower::Wall));
            }
        }
        wall_path.push(wall_path[0]);
        self.land
            .write_path(&wall_path, WayKind::Wall, buildable, true);
    }

    pub fn place_farms(&mut self, rng: &mut impl Rng) {
        for _ in 0..6 {
            if let Some(base_tile) = self
                .land
                .find_tile_near(Vec2::zero(), |plot| plot.is_none())
            {
                // Farm
                let farmhouse = self.land.new_plot(Plot::Dirt);
                self.land.set(base_tile, farmhouse);

                // Farmhouses
                for _ in 0..rng.gen_range(1, 4) {
                    let house_pos = base_tile.map(|e| e * AREA_SIZE as i32 + AREA_SIZE as i32 / 2)
                        + Vec2::new(rng.gen_range(-16, 16), rng.gen_range(-16, 16));

                    self.structures.push(Structure {
                        kind: StructureKind::House,
                        bounds: Aabr {
                            min: house_pos - Vec2::new(rng.gen_range(4, 6), rng.gen_range(4, 6)),
                            max: house_pos + Vec2::new(rng.gen_range(4, 6), rng.gen_range(4, 6)),
                        },
                    });
                }

                // Fields
                let farmland = self.farms.insert(Farm { base_tile });
                for _ in 0..5 {
                    self.place_field(farmland, base_tile, rng);
                }
            }
        }
    }

    pub fn place_field(
        &mut self,
        farm: Id<Farm>,
        origin: Vec2<i32>,
        rng: &mut impl Rng,
    ) -> Option<Id<Plot>> {
        let max_size = 7;

        if let Some(center) = self.land.find_tile_near(origin, |plot| plot.is_none()) {
            let field = self.land.new_plot(Plot::Field {
                farm,
                seed: rng.gen(),
            });
            let tiles = self
                .land
                .grow_from(center, rng.gen_range(1, max_size), rng, |plot| {
                    plot.is_none()
                });
            for pos in tiles.into_iter() {
                self.land.set(pos, field);
            }
            Some(field)
        } else {
            None
        }
    }

    pub fn get_surface(&self, wpos: Vec2<i32>) -> Option<Block> {
        self.get_color((wpos - self.origin).map(|e| e as f32))
            .map(|col| Block::new(BlockKind::Normal, col))
    }

    pub fn get_color(&self, pos: Vec2<f32>) -> Option<Rgb<u8>> {
        let pos = pos.map(|e| e.floor() as i32);

        if let Some(structure) = self
            .structures
            .iter()
            .find(|s| s.bounds.contains_point(pos))
        {
            return Some(match structure.kind {
                StructureKind::House => Rgb::new(200, 80, 50),
            });
        }

        Some(match self.land.get_at_block(pos) {
            Sample::Wilderness => return None,
            Sample::Way(WayKind::Path) => Rgb::new(130, 100, 0),
            Sample::Way(WayKind::Hedge) => Rgb::new(0, 150, 0),
            Sample::Way(WayKind::Wall) => Rgb::new(60, 60, 60),
            Sample::Tower(Tower::Wall) => Rgb::new(50, 50, 50),
            Sample::Plot(Plot::Dirt) => Rgb::new(130, 100, 0),
            Sample::Plot(Plot::Grass) => Rgb::new(100, 200, 0),
            Sample::Plot(Plot::Water) => Rgb::new(100, 150, 250),
            Sample::Plot(Plot::Town) => {
                if pos.map(|e| e.rem_euclid(4) < 2).reduce(|x, y| x ^ y) {
                    Rgb::new(200, 130, 120)
                } else {
                    Rgb::new(160, 150, 120)
                }
            },
            Sample::Plot(Plot::Field { seed, .. }) => {
                let furrow_dirs = [
                    Vec2::new(1, 0),
                    Vec2::new(0, 1),
                    Vec2::new(1, 1),
                    Vec2::new(-1, 1),
                ];
                let furrow_dir = furrow_dirs[*seed as usize % furrow_dirs.len()];
                let furrow = (pos * furrow_dir).sum().rem_euclid(4) < 2;
                Rgb::new(
                    if furrow {
                        120
                    } else {
                        48 + seed.to_le_bytes()[0] % 64
                    },
                    128 + seed.to_le_bytes()[1] % 128,
                    16 + seed.to_le_bytes()[2] % 32,
                )
            },
        })
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Plot {
    Dirt,
    Grass,
    Water,
    Town,
    Field { farm: Id<Farm>, seed: u32 },
}

const CARDINALS: [Vec2<i32>; 4] = [
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
];

#[derive(Copy, Clone, PartialEq)]
pub enum WayKind {
    Path,
    Hedge,
    Wall,
}

impl WayKind {
    pub fn width(&self) -> f32 {
        match self {
            WayKind::Path => 4.0,
            WayKind::Hedge => 1.5,
            WayKind::Wall => 3.5,
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Tower {
    Wall,
}

impl Tower {
    pub fn radius(&self) -> f32 {
        match self {
            Tower::Wall => 8.0,
        }
    }
}

pub struct Tile {
    plot: Id<Plot>,
    ways: [Option<WayKind>; 4],
    tower: Option<Tower>,
}

impl Tile {
    pub fn contains(&self, kind: WayKind) -> bool { self.ways.iter().any(|way| way == &Some(kind)) }
}

pub enum Sample<'a> {
    Wilderness,
    Plot(&'a Plot),
    Way(&'a WayKind),
    Tower(&'a Tower),
}

pub struct Land {
    tiles: HashMap<Vec2<i32>, Tile>,
    plots: Store<Plot>,
    sampler_warp: StructureGen2d,
}

impl Land {
    pub fn new(rng: &mut impl Rng) -> Self {
        Self {
            tiles: HashMap::new(),
            plots: Store::default(),
            sampler_warp: StructureGen2d::new(rng.gen(), AREA_SIZE, AREA_SIZE * 2 / 5),
        }
    }

    pub fn get_at_block(&self, pos: Vec2<i32>) -> Sample {
        let neighbors = self.sampler_warp.get(pos);
        let closest = neighbors
            .iter()
            .min_by_key(|(center, _)| center.distance_squared(pos))
            .unwrap()
            .0;

        let center_tile = self.tile_at(neighbors[4].0.map(to_tile));

        if let Some(tower) = center_tile.and_then(|tile| tile.tower.as_ref()) {
            if (neighbors[4].0.distance_squared(pos) as f32) < tower.radius().powf(2.0) {
                return Sample::Tower(tower);
            }
        }

        for (i, dir) in CARDINALS.iter().enumerate() {
            let map = [1, 5, 7, 3];
            let line = [
                neighbors[4].0.map(|e| e as f32),
                neighbors[map[i]].0.map(|e| e as f32),
            ];
            if let Some(way) = center_tile.and_then(|tile| tile.ways[i].as_ref()) {
                if dist_to_line(line, pos.map(|e| e as f32)) < way.width() {
                    return Sample::Way(way);
                }
            }
        }

        let plot = self.plot_at(closest.map(to_tile));

        plot.map(|plot| Sample::Plot(plot))
            .unwrap_or(Sample::Wilderness)
    }

    pub fn tile_at(&self, pos: Vec2<i32>) -> Option<&Tile> { self.tiles.get(&pos) }

    pub fn tile_at_mut(&mut self, pos: Vec2<i32>) -> Option<&mut Tile> { self.tiles.get_mut(&pos) }

    pub fn plot(&self, id: Id<Plot>) -> &Plot { self.plots.get(id) }

    pub fn plot_at(&self, pos: Vec2<i32>) -> Option<&Plot> {
        self.tiles.get(&pos).map(|tile| self.plots.get(tile.plot))
    }

    pub fn plot_at_mut(&mut self, pos: Vec2<i32>) -> Option<&mut Plot> {
        self.tiles
            .get(&pos)
            .map(|tile| tile.plot)
            .map(move |plot| self.plots.get_mut(plot))
    }

    pub fn set(&mut self, pos: Vec2<i32>, plot: Id<Plot>) {
        self.tiles.insert(pos, Tile {
            plot,
            ways: [None; 4],
            tower: None,
        });
    }

    fn find_tile_near(
        &self,
        origin: Vec2<i32>,
        mut match_fn: impl FnMut(Option<&Plot>) -> bool,
    ) -> Option<Vec2<i32>> {
        Spiral2d::new()
            .map(|pos| origin + pos)
            .find(|pos| match_fn(self.plot_at(*pos)))
    }

    fn find_tile_dir(
        &self,
        origin: Vec2<i32>,
        dir: Vec2<i32>,
        mut match_fn: impl FnMut(Option<&Plot>) -> bool,
    ) -> Option<Vec2<i32>> {
        (0..)
            .map(|i| origin + dir * i)
            .find(|pos| match_fn(self.plot_at(*pos)))
    }

    fn find_path(
        &self,
        origin: Vec2<i32>,
        dest: Vec2<i32>,
        mut path_cost_fn: impl FnMut(Option<&Tile>, Option<&Tile>) -> f32,
    ) -> Option<Path<Vec2<i32>>> {
        let heuristic = |pos: &Vec2<i32>| pos.distance_squared(dest) as f32;
        let neighbors = |pos: &Vec2<i32>| {
            let pos = *pos;
            CARDINALS.iter().map(move |dir| pos + *dir)
        };
        let transition =
            |from: &Vec2<i32>, to: &Vec2<i32>| path_cost_fn(self.tile_at(*from), self.tile_at(*to));
        let satisfied = |pos: &Vec2<i32>| *pos == dest;

        Astar::new(250, origin, heuristic)
            .poll(250, heuristic, neighbors, transition, satisfied)
            .into_path()
    }

    fn grow_from(
        &self,
        start: Vec2<i32>,
        max_size: usize,
        rng: &mut impl Rng,
        mut match_fn: impl FnMut(Option<&Plot>) -> bool,
    ) -> HashSet<Vec2<i32>> {
        let mut open = VecDeque::new();
        open.push_back(start);
        let mut closed = HashSet::new();

        while open.len() + closed.len() < max_size {
            let next_pos = if let Some(next_pos) = open.pop_front() {
                closed.insert(next_pos);
                next_pos
            } else {
                break;
            };

            let dirs = [
                Vec2::new(1, 0),
                Vec2::new(-1, 0),
                Vec2::new(0, 1),
                Vec2::new(0, -1),
            ];

            for dir in dirs.iter() {
                let neighbor = next_pos + dir;
                if !closed.contains(&neighbor) && match_fn(self.plot_at(neighbor)) {
                    open.push_back(neighbor);
                }
            }
        }

        closed.into_iter().chain(open.into_iter()).collect()
    }

    fn write_path(
        &mut self,
        tiles: &[Vec2<i32>],
        kind: WayKind,
        mut permit_fn: impl FnMut(&Plot) -> bool,
        overwrite: bool,
    ) {
        for tiles in tiles.windows(2) {
            let dir = tiles[1] - tiles[0];
            let idx = if dir.y > 0 {
                1
            } else if dir.x > 0 {
                2
            } else if dir.y < 0 {
                3
            } else if dir.x < 0 {
                0
            } else {
                continue;
            };
            let mut plots = &self.plots;
            self.tiles
                .get_mut(&tiles[1])
                .filter(|tile| permit_fn(plots.get(tile.plot)))
                .map(|tile| {
                    if overwrite || tile.ways[(idx + 2) % 4].is_none() {
                        tile.ways[(idx + 2) % 4] = Some(kind);
                    }
                });
            self.tiles
                .get_mut(&tiles[0])
                .filter(|tile| permit_fn(plots.get(tile.plot)))
                .map(|tile| {
                    if overwrite || tile.ways[idx].is_none() {
                        tile.ways[idx] = Some(kind);
                    }
                });
        }
    }

    pub fn new_plot(&mut self, plot: Plot) -> Id<Plot> { self.plots.insert(plot) }
}

#[derive(Hash)]
pub struct Id<T>(usize, PhantomData<T>);

impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self { Self(self.0, PhantomData) }
}
impl<T> Eq for Id<T> {}
impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

pub struct Store<T> {
    items: Vec<T>,
}

impl<T> Default for Store<T> {
    fn default() -> Self { Self { items: Vec::new() } }
}

impl<T> Store<T> {
    pub fn get(&self, id: Id<T>) -> &T { self.items.get(id.0).unwrap() }

    pub fn get_mut(&mut self, id: Id<T>) -> &mut T { self.items.get_mut(id.0).unwrap() }

    pub fn iter(&self) -> impl Iterator<Item = &T> { self.items.iter() }

    pub fn insert(&mut self, item: T) -> Id<T> {
        let id = Id(self.items.len(), PhantomData);
        self.items.push(item);
        id
    }
}
