mod building;

use crate::{
    column::ColumnSample,
    sim::{SimChunk, WorldSim},
    util::{Grid, RandomField, Sampler, StructureGen2d},
};
use self::building::HouseBuilding;
use super::SpawnRules;
use common::{
    astar::Astar,
    path::Path,
    spiral::Spiral2d,
    terrain::{Block, BlockKind},
    vol::{BaseVol, RectSizedVol, WriteVol, Vox},
    store::{Id, Store},
};
use hashbrown::{HashMap, HashSet};
use rand::prelude::*;
use std::{collections::VecDeque, f32};
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

impl SimChunk {
    fn can_host_settlement(&self) -> bool {
        !self.near_cliffs && !self.river.is_river() && !self.river.is_lake()
    }
}

const AREA_SIZE: u32 = 32;

fn to_tile(e: i32) -> i32 { ((e as f32).div_euclid(AREA_SIZE as f32)).floor() as i32 }

pub enum StructureKind {
    House(HouseBuilding),
}

pub struct Structure {
    kind: StructureKind,
}

impl Structure {
    pub fn bounds_2d(&self) -> Aabr<i32> {
        match &self.kind {
            StructureKind::House(house) => house.bounds_2d(),
        }
    }
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

pub struct GenCtx<'a, R: Rng> {
    sim: Option<&'a WorldSim>,
    rng: &'a mut R,
}

impl Settlement {
    pub fn generate(wpos: Vec2<i32>, sim: Option<&WorldSim>, rng: &mut impl Rng) -> Self {
        let mut ctx = GenCtx { sim, rng };
        let mut this = Self {
            origin: wpos,
            land: Land::new(ctx.rng),
            farms: Store::default(),
            structures: Vec::new(),
            town: None,
        };

        if let Some(sim) = ctx.sim {
            this.designate_from_world(sim, ctx.rng);
        }

        //this.place_river(rng);

        this.place_farms(&mut ctx);
        this.place_town(&mut ctx);
        this.place_paths(ctx.rng);

        this
    }

    /// Designate hazardous terrain based on world data
    pub fn designate_from_world(&mut self, sim: &WorldSim, rng: &mut impl Rng) {
        let tile_radius = self.radius() as i32 / AREA_SIZE as i32;
        let hazard = self.land.new_plot(Plot::Hazard);
        Spiral2d::new()
            .take_while(|tile| tile.map(|e| e.abs()).reduce_max() < tile_radius)
            .for_each(|tile| {
                let wpos = self.origin + tile * AREA_SIZE as i32;

                if (0..4)
                    .map(|x| (0..4).map(move |y| Vec2::new(x, y)))
                    .flatten()
                    .any(|offs| {
                        sim.get_wpos(wpos + offs * AREA_SIZE as i32 / 2)
                            .map(|chunk| !chunk.can_host_settlement())
                            .unwrap_or(true)
                    })
                {
                    self.land.set(tile, hazard);
                }
            })
    }

    /// Testing only
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
        const PATH_COUNT: usize = 6;

        let mut dir = Vec2::zero();
        for _ in 0..PATH_COUNT {
            dir = (Vec2::new(rng.gen::<f32>() - 0.5, rng.gen::<f32>() - 0.5) * 2.0 - dir)
                .try_normalized()
                .unwrap_or(Vec2::zero());
            let origin = dir.map(|e| (e * 100.0) as i32);
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
                        (_, Some(b)) if self.land.plot(b.plot) == &Plot::Hazard => 50.0,
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
                self.land.write_path(&path, WayKind::Path, |_| true, false);
            }
        }
    }

    pub fn place_town(&mut self, ctx: &mut GenCtx<impl Rng>) {
        const PLOT_COUNT: usize = 2;

        let mut origin = Vec2::new(ctx.rng.gen_range(-2, 3), ctx.rng.gen_range(-2, 3));

        for i in 0..PLOT_COUNT {
            if let Some(base_tile) = self.land.find_tile_near(origin, |plot| match plot {
                Some(Plot::Field { .. }) => true,
                Some(Plot::Dirt) => true,
                _ => false,
            }) {
                self.land
                    .plot_at_mut(base_tile)
                    .map(|plot| *plot = Plot::Town);

                for _ in 0..ctx.rng.gen_range(10, 30) {
                    for _ in 0..10 {
                        let house_pos = base_tile.map(|e| e * AREA_SIZE as i32 + AREA_SIZE as i32 / 2)
                            + Vec2::<i32>::zero().map(|_| ctx.rng.gen_range(-(AREA_SIZE as i32) * 3, AREA_SIZE as i32 * 3));

                        if let Some(Plot::Town) = self.land
                            .plot_at(house_pos.map(|e| e.div_euclid(AREA_SIZE as i32)))
                        {} else {
                            continue;
                        }

                        let structure = Structure {
                            kind: StructureKind::House(HouseBuilding::generate(ctx.rng, Vec3::new(
                                house_pos.x,
                                house_pos.y,
                                ctx.sim
                                    .and_then(|sim| sim.get_alt_approx(self.origin + house_pos))
                                    .unwrap_or(0.0)
                                    .ceil() as i32,
                            ))),
                        };

                        let bounds = structure.bounds_2d();

                        // Check for collision with other structures
                        if self.structures
                            .iter()
                            .any(|s| s.bounds_2d().collides_with_aabr(bounds))
                        {
                            continue;
                        }

                        self.structures.push(structure);
                        break;
                    }
                }

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
                    Some(Plot::Hazard) => 10000.0,
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
        if wall_path.len() > 0 {
            wall_path.push(wall_path[0]);
        }
        self.land
            .write_path(&wall_path, WayKind::Wall, buildable, true);
    }

    pub fn place_farms(&mut self, ctx: &mut GenCtx<impl Rng>) {
        const FARM_COUNT: usize = 6;
        const FIELDS_PER_FARM: usize = 5;

        for _ in 0..FARM_COUNT {
            if let Some(base_tile) = self
                .land
                .find_tile_near(Vec2::zero(), |plot| plot.is_none())
            {
                // Farm
                let farmhouse = self.land.new_plot(Plot::Dirt);
                self.land.set(base_tile, farmhouse);

                // Farmhouses
                // for _ in 0..ctx.rng.gen_range(1, 3) {
                //     let house_pos = base_tile.map(|e| e * AREA_SIZE as i32 + AREA_SIZE as i32 / 2)
                //         + Vec2::new(ctx.rng.gen_range(-16, 16), ctx.rng.gen_range(-16, 16));

                //     self.structures.push(Structure {
                //         kind: StructureKind::House(HouseBuilding::generate(ctx.rng, Vec3::new(
                //             house_pos.x,
                //             house_pos.y,
                //             ctx.sim
                //                 .and_then(|sim| sim.get_alt_approx(self.origin + house_pos))
                //                 .unwrap_or(0.0)
                //                 .ceil() as i32,
                //         ))),
                //     });
                // }

                // Fields
                let farmland = self.farms.insert(Farm { base_tile });
                for _ in 0..FIELDS_PER_FARM {
                    self.place_field(farmland, base_tile, ctx.rng);
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
        const MAX_FIELD_SIZE: usize = 24;

        if let Some(center) = self.land.find_tile_near(origin, |plot| plot.is_none()) {
            let field = self.land.new_plot(Plot::Field {
                farm,
                seed: rng.gen(),
            });
            let tiles =
                self.land
                    .grow_from(center, rng.gen_range(5, MAX_FIELD_SIZE), rng, |plot| {
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

    pub fn radius(&self) -> f32 { 1200.0 }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            trees: self.land.get_at_block(wpos - self.origin).plot.is_none(),
        }
    }

    pub fn apply_to<'a>(
        &'a self,
        wpos2d: Vec2<i32>,
        mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + WriteVol),
    ) {
        let rand_field = RandomField::new(0);

        for y in 0..vol.size_xy().y as i32 {
            for x in 0..vol.size_xy().x as i32 {
                let offs = Vec2::new(x, y);

                let wpos2d = wpos2d + offs;
                let rpos = wpos2d - self.origin;

                // Sample terrain
                let col_sample = if let Some(col_sample) = get_column(offs) {
                    col_sample
                } else {
                    continue;
                };
                let surface_z = col_sample.alt.floor() as i32;

                // Sample settlement
                let sample = self.land.get_at_block(rpos);

                // Ground color
                if let Some(color) = self.get_color(rpos) {
                    for z in -3..5 {
                        vol.set(
                            Vec3::new(offs.x, offs.y, surface_z + z),
                            if z >= 0 { Block::empty() } else { Block::new(BlockKind::Normal, color) },
                        );
                    }
                }

                // Walls
                if let Some((WayKind::Wall, dist)) = sample.way {
                    let color = Lerp::lerp(
                        Rgb::new(130i32, 100, 0),
                        Rgb::new(90, 70, 50),
                        (rand_field.get(wpos2d.into()) % 256) as f32 / 256.0,
                    )
                    .map(|e| (e % 256) as u8);
                    for z in 0..12 {
                        if dist / WayKind::Wall.width()
                            < ((1.0 - z as f32 / 12.0) * 2.0).min(1.0)
                        {
                            vol.set(
                                Vec3::new(offs.x, offs.y, surface_z + z),
                                Block::new(BlockKind::Normal, color),
                            );
                        }
                    }
                }

                // Towers
                if let Some((Tower::Wall, _pos)) = sample.tower {
                    for z in 0..16 {
                        vol.set(
                            Vec3::new(offs.x, offs.y, surface_z + z),
                            Block::new(BlockKind::Normal, Rgb::new(50, 50, 50)),
                        );
                    }
                }

                // Paths
                if let Some((WayKind::Path, dist)) = sample.way {
                    let inset = -1;
                    for z in -3..inset {
                        vol.set(
                            Vec3::new(offs.x, offs.y, surface_z + z),
                            Block::new(BlockKind::Normal, Rgb::new(90, 70, 50)),
                        );
                    }
                    let head_space = (6 - (dist * 0.4).powf(6.0).round() as i32).max(1);
                    for z in inset..inset + head_space {
                        vol.set(
                            Vec3::new(offs.x, offs.y, surface_z + z),
                            Block::empty(),
                        );
                    }
                }
            }
        }

        // Apply structures
        for structure in &self.structures {
            let bounds = structure.bounds_2d();

            // Skip this structure if it's not near this chunk
            if !bounds.collides_with_aabr(Aabr {
                min: wpos2d - self.origin,
                max: wpos2d - self.origin + vol.size_xy().map(|e| e as i32),
            }) {
                continue;
            }

            match &structure.kind {
                StructureKind::House(b) => {
                    let centre = b.bounds_2d().center();
                    let bounds = b.bounds();

                    for x in bounds.min.x..bounds.max.x + 1 {
                        for y in bounds.min.y..bounds.max.y + 1 {
                            for z in bounds.min.z..bounds.max.z + 1 {
                                let rpos = Vec3::new(x, y, z);
                                let wpos = Vec3::from(self.origin) + rpos;
                                let coffs = wpos - Vec3::from(wpos2d);

                                if let Some(block) = b.sample(rpos) {
                                    vol.set(coffs, block);
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    pub fn get_color(&self, pos: Vec2<i32>) -> Option<Rgb<u8>> {
        let sample = self.land.get_at_block(pos);

        match sample.tower {
            Some((Tower::Wall, _)) => return Some(Rgb::new(50, 50, 50)),
            _ => {},
        }

        match sample.way {
            Some((WayKind::Path, _)) => return Some(Rgb::new(90, 70, 50)),
            Some((WayKind::Hedge, _)) => return Some(Rgb::new(0, 150, 0)),
            Some((WayKind::Wall, _)) => return Some(Rgb::new(60, 60, 60)),
            _ => {},
        }

        match sample.plot {
            Some(Plot::Dirt) => return Some(Rgb::new(90, 70, 50)),
            Some(Plot::Grass) => return Some(Rgb::new(100, 200, 0)),
            Some(Plot::Water) => return Some(Rgb::new(100, 150, 250)),
            Some(Plot::Town) => return Some(if pos.map(|e| e.rem_euclid(4) < 2).reduce(|x, y| x ^ y) {
                Rgb::new(200, 130, 120)
            } else {
                Rgb::new(160, 150, 120)
            }),
            Some(Plot::Field { seed, .. }) => {
                let furrow_dirs = [
                    Vec2::new(1, 0),
                    Vec2::new(0, 1),
                    Vec2::new(1, 1),
                    Vec2::new(-1, 1),
                ];
                let furrow_dir = furrow_dirs[*seed as usize % furrow_dirs.len()];
                let furrow = (pos * furrow_dir).sum().rem_euclid(6) < 3;
                return Some(Rgb::new(
                    if furrow {
                        100
                    } else {
                        32 + seed.to_le_bytes()[0] % 64
                    },
                    64 + seed.to_le_bytes()[1] % 128,
                    16 + seed.to_le_bytes()[2] % 32,
                ));
            },
            _ => {},
        }

        None
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Plot {
    Hazard,
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
            WayKind::Wall => 2.5,
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
            Tower::Wall => 6.0,
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

#[derive(Default)]
pub struct Sample<'a> {
    plot: Option<&'a Plot>,
    way: Option<(&'a WayKind, f32)>,
    tower: Option<(&'a Tower, Vec2<i32>)>,
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
        let mut sample = Sample::default();

        let neighbors = self.sampler_warp.get(pos);
        let closest = neighbors
            .iter()
            .min_by_key(|(center, _)| center.distance_squared(pos))
            .unwrap()
            .0;

        let center_tile = self.tile_at(neighbors[4].0.map(to_tile));

        if let Some(tower) = center_tile.and_then(|tile| tile.tower.as_ref()) {
            if (neighbors[4].0.distance_squared(pos) as f32) < tower.radius().powf(2.0) {
                sample.tower = Some((tower, neighbors[4].0));
            }
        }

        for (i, dir) in CARDINALS.iter().enumerate() {
            let map = [1, 5, 7, 3];
            let line = [
                neighbors[4].0.map(|e| e as f32),
                neighbors[map[i]].0.map(|e| e as f32),
            ];
            if let Some(way) = center_tile.and_then(|tile| tile.ways[i].as_ref()) {
                let dist = dist_to_line(line, pos.map(|e| e as f32));
                if dist < way.width() {
                    sample.way = Some((way, dist));
                }
            }
        }

        sample.plot = self.plot_at(closest.map(to_tile));

        sample
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
