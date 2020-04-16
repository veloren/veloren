use crate::{
    column::ColumnSample,
    sim::{SimChunk, WorldSim},
    util::{attempt, Grid, RandomField, Sampler, StructureGen2d},
    site::BlockMask,
};
use super::SpawnRules;
use common::{
    astar::Astar,
    path::Path,
    spiral::Spiral2d,
    terrain::{Block, BlockKind, TerrainChunkSize},
    vol::{BaseVol, RectSizedVol, RectVolSize, ReadVol, WriteVol, Vox},
    store::{Id, Store},
    generation::{ChunkSupplement, EntityInfo, EntityKind},
};
use hashbrown::{HashMap, HashSet};
use rand::prelude::*;
use std::{collections::VecDeque, f32};
use vek::*;

impl WorldSim {
    fn can_host_dungeon(&self, pos: Vec2<i32>) -> bool {
        self
            .get(pos)
            .map(|chunk| {
                !chunk.near_cliffs && !chunk.river.is_river() && !chunk.river.is_lake()
            })
            .unwrap_or(false)
        && self
            .get_gradient_approx(pos)
            .map(|grad| grad > 0.25 && grad < 1.5)
            .unwrap_or(false)
    }
}

pub struct Dungeon {
    origin: Vec2<i32>,
    alt: i32,
    noise: RandomField,
    floors: Vec<Floor>,
}

pub struct GenCtx<'a, R: Rng> {
    sim: Option<&'a WorldSim>,
    rng: &'a mut R,
}

impl Dungeon {
    pub fn generate(wpos: Vec2<i32>, sim: Option<&WorldSim>, rng: &mut impl Rng) -> Self {
        let mut ctx = GenCtx { sim, rng };
        let mut this = Self {
            origin: wpos,
            alt: ctx.sim.and_then(|sim| sim.get_alt_approx(wpos)).unwrap_or(0.0) as i32 + 6,
            noise: RandomField::new(ctx.rng.gen()),
            floors: (0..6)
                .scan(Vec2::zero(), |stair_tile, level| {
                    let (floor, st) = Floor::generate(&mut ctx, *stair_tile, level);
                    *stair_tile = st;
                    Some(floor)
                })
                .collect(),
        };

        this
    }

    pub fn radius(&self) -> f32 { 1200.0 }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            ..SpawnRules::default()
        }
    }

    pub fn apply_to<'a>(
        &'a self,
        wpos2d: Vec2<i32>,
        mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
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
                let surface_z = col_sample.riverless_alt.floor() as i32;

                let tile_pos = rpos.map(|e| e.div_euclid(TILE_SIZE));
                let tile_center = tile_pos * TILE_SIZE + TILE_SIZE / 2;

                let mut z = self.alt;
                for floor in &self.floors {
                    let mut sampler = floor.col_sampler(rpos);

                    z -= floor.total_depth();
                    for rz in 0..floor.total_depth() {
                        if let Some(block) = sampler(rz).finish() {
                            vol.set(Vec3::new(offs.x, offs.y, z + rz), block);
                        }
                    }
                }
            }
        }
    }

    pub fn apply_supplement<'a>(
        &'a self,
        wpos2d: Vec2<i32>,
        mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        supplement: &mut ChunkSupplement,
    ) {
        let rpos = wpos2d - self.origin;
        let area = Aabr {
            min: rpos,
            max: rpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
        };

        let mut z = self.alt;
        for floor in &self.floors {
            z -= floor.total_depth();
            let origin = Vec3::new(self.origin.x, self.origin.y, z);
            floor.apply_supplement(area, origin, supplement);
        }
    }
}

const CARDINALS: [Vec2<i32>; 4] = [
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
];

const DIRS: [Vec2<i32>; 8] = [
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
    Vec2::new(1, 1),
    Vec2::new(1, -1),
    Vec2::new(-1, 1),
    Vec2::new(-1, -1),
];

const TILE_SIZE: i32 = 13;

#[derive(Clone)]
pub enum Tile {
    UpStair,
    DownStair,
    Room,
    Tunnel,
    Solid,
}

impl Tile {
    fn is_passable(&self) -> bool {
        match self {
            Tile::UpStair => true,
            Tile::DownStair => true,
            Tile::Room => true,
            Tile::Tunnel => true,
            _ => false,
        }
    }
}

pub struct Floor {
    tile_offset: Vec2<i32>,
    tiles: Grid<Tile>,
    rooms: Vec<Rect<i32, i32>>,
    solid_depth: i32,
    hollow_depth: i32,
    stair_tile: Vec2<i32>,
}

const FLOOR_SIZE: Vec2<i32> = Vec2::new(18, 18);

impl Floor {
    pub fn generate(ctx: &mut GenCtx<impl Rng>, stair_tile: Vec2<i32>, level: i32) -> (Self, Vec2<i32>) {
        let new_stair_tile = std::iter::from_fn(|| Some(FLOOR_SIZE.map(|sz| ctx.rng.gen_range(-sz / 2 + 1, sz / 2))))
            .filter(|pos| *pos != stair_tile)
            .take(8)
            .max_by_key(|pos| (*pos - stair_tile).map(|e| e.abs()).sum())
            .unwrap();

        let tile_offset = -FLOOR_SIZE / 2;
        let mut this = Floor {
            tile_offset,
            tiles: Grid::new(FLOOR_SIZE, Tile::Solid),
            rooms: Vec::new(),
            solid_depth: if level == 0 {
                80
            } else {
                13 * 2
            },
            hollow_depth: 13,
            stair_tile: new_stair_tile - tile_offset,
        };
        this.create_rooms(ctx, 10);
        // Create routes between all rooms
        let rooms = this.rooms.clone();
        for a in rooms.iter() {
            for b in rooms.iter() {
                this.create_route(ctx, a.center(), b.center(), true);
            }
        }
        this.create_route(ctx, stair_tile - tile_offset, new_stair_tile - tile_offset, false);

        this.tiles.set(stair_tile - tile_offset, Tile::UpStair);
        this.tiles.set(new_stair_tile - tile_offset, Tile::DownStair);

        (this, new_stair_tile)
    }

    fn create_rooms(&mut self, ctx: &mut GenCtx<impl Rng>, n: usize) {
        let dim_limits = (3, 8);

        for _ in 0..n {
            let room = match attempt(30, || {
                let sz = Vec2::<i32>::zero().map(|_| ctx.rng.gen_range(dim_limits.0, dim_limits.1));
                let pos = FLOOR_SIZE.map2(sz, |floor_sz, room_sz| ctx.rng.gen_range(0, floor_sz + 1 - room_sz));
                let room = Rect::from((pos, Extent2::from(sz)));
                let room_border = Rect::from((pos - 1, Extent2::from(sz) + 2)); // The room, but with some personal space

                // Ensure no overlap
                if self.rooms
                    .iter()
                    .any(|r| r.collides_with_rect(room_border) || r.contains_point(self.stair_tile))
                {
                    return None;
                }

                Some(room)
            }) {
                Some(room) => room,
                None => return,
            };

            self.rooms.push(room);

            for x in 0..room.extent().w {
                for y in 0..room.extent().h {
                    self.tiles.set(room.position() + Vec2::new(x, y), Tile::Room);
                }
            }
        }
    }

    fn create_route(&mut self, ctx: &mut GenCtx<impl Rng>, a: Vec2<i32>, b: Vec2<i32>, optimise_longest: bool) {
        let sim = &ctx.sim;
        let heuristic = move |l: &Vec2<i32>| if optimise_longest {
            (l.distance_squared(b) as f32).sqrt()
        } else {
            100.0 - (l.distance_squared(b) as f32).sqrt()
        };
        let neighbors = |l: &Vec2<i32>| {
            let l = *l;
            CARDINALS
                .iter()
                .map(move |dir| l + dir)
                .filter(|pos| self.tiles.get(*pos).is_some())
        };
        let transition = |a: &Vec2<i32>, b: &Vec2<i32>| match self.tiles.get(*b) {
            Some(Tile::Room) | Some(Tile::Tunnel) => 1.0,
            Some(Tile::Solid) => 25.0,
            Some(Tile::UpStair) | Some(Tile::DownStair) => 0.0,
            _ => 100000.0,
        };
        let satisfied = |l: &Vec2<i32>| *l == b;
        let mut astar = Astar::new(20000, a, heuristic);
        let path = astar
            .poll(FLOOR_SIZE.product() as usize + 1, heuristic, neighbors, transition, satisfied)
            .into_path()
            .expect("No route between locations - this shouldn't be able to happen");

        for pos in path.iter() {
            if let Some(tile @ Tile::Solid) = self.tiles.get_mut(*pos) {
                *tile = Tile::Tunnel;
            }
        }
    }

    pub fn apply_supplement(&self, area: Aabr<i32>, origin: Vec3<i32>, supplement: &mut ChunkSupplement) {
        let align = |e: i32| e.div_euclid(TILE_SIZE) + if e.rem_euclid(TILE_SIZE) > TILE_SIZE / 2 {
            1
        } else {
            0
        };
        let aligned_area = Aabr {
            min: area.min.map(align) + self.tile_offset,
            max: area.max.map(align) + self.tile_offset,
        };

        for x in aligned_area.min.x..aligned_area.max.x {
            for y in aligned_area.min.y..aligned_area.max.y {
                let tile_pos = Vec2::new(x, y);
                if let Some(Tile::Room) = self.tiles.get(tile_pos) {
                    if tile_pos.x % 4 != 0 || tile_pos.y % 4 != 0 { continue; } // This is so bad
                    supplement.add_entity(EntityInfo {
                        pos: (origin + Vec3::from(self.tile_offset + tile_pos) * TILE_SIZE + TILE_SIZE / 2).map(|e| e as f32),
                        kind: EntityKind::Boss,
                    });
                }
            }
        }
    }

    pub fn total_depth(&self) -> i32 {
        self.solid_depth + self.hollow_depth
    }

    pub fn nearest_wall(&self, rpos: Vec2<i32>) -> Option<Vec2<i32>> {
        let tile_pos = rpos.map(|e| e.div_euclid(TILE_SIZE));
        let tile_center = tile_pos * TILE_SIZE + TILE_SIZE / 2;

        DIRS
            .iter()
            .map(|dir| tile_pos + *dir)
            .filter(|other_tile_pos| self.tiles.get(*other_tile_pos).filter(|tile| tile.is_passable()).is_none())
            .map(|other_tile_pos| rpos.clamped(
                other_tile_pos * TILE_SIZE,
                (other_tile_pos + 1) * TILE_SIZE - 1,
            ))
            .min_by_key(|nearest| rpos.distance_squared(*nearest))
    }

    pub fn col_sampler(&self, pos: Vec2<i32>) -> impl FnMut(i32) -> BlockMask + '_ {
        let rpos = pos - self.tile_offset * TILE_SIZE;
        let tile_pos = rpos.map(|e| e.div_euclid(TILE_SIZE));
        let tile_center = tile_pos * TILE_SIZE + TILE_SIZE / 2;
        let rtile_pos = rpos - tile_center;

        let empty = BlockMask::new(Block::empty(), 1);

        let make_staircase = move |pos: Vec3<i32>, radius: f32, inner_radius: f32, stretch: f32| {
            let stone = BlockMask::new(Block::new(BlockKind::Normal, Rgb::new(150, 150, 175)), 5);

            if (pos.xy().magnitude_squared() as f32) < inner_radius.powf(2.0) {
                stone
            } else if (pos.xy().magnitude_squared() as f32) < radius.powf(2.0) {
                if ((pos.x as f32).atan2(pos.y as f32) / (f32::consts::PI * 2.0) * stretch + pos.z as f32).rem_euclid(stretch) < 1.5 {
                    stone
                } else {
                    empty
                }
            } else {
                BlockMask::nothing()
            }
        };

        let tunnel_dist = self.nearest_wall(rpos).map(|nearest| 1.0 - (nearest.distance_squared(rpos) as f32).sqrt() / TILE_SIZE as f32).unwrap_or(0.0);

        move |z| {
            match self.tiles.get(tile_pos) {
                Some(Tile::Solid) => BlockMask::nothing(),
                Some(Tile::Tunnel) => {
                    if (z as f32) < 8.0 - 8.0 * tunnel_dist.powf(4.0) {
                        empty
                    } else {
                        BlockMask::nothing()
                    }
                },
                Some(Tile::Room) | Some(Tile::DownStair) if z as f32 >= self.hollow_depth as f32 - 13.0 * tunnel_dist.powf(4.0) => BlockMask::nothing(),
                Some(Tile::Room) => empty,
                Some(Tile::DownStair) => make_staircase(Vec3::new(rtile_pos.x, rtile_pos.y, z), 0.0, 0.5, 9.0).resolve_with(empty),
                Some(Tile::UpStair) => {
                    let mut block = make_staircase(Vec3::new(rtile_pos.x, rtile_pos.y, z), TILE_SIZE as f32 / 2.0, 0.5, 9.0);
                    if z < self.hollow_depth {
                        block = block.resolve_with(empty);
                    }
                    block
                },
                None => BlockMask::nothing(),
            }
        }
    }
}
