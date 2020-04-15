use crate::{
    column::ColumnSample,
    sim::{SimChunk, WorldSim},
    util::{Grid, RandomField, Sampler, StructureGen2d},
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
            alt: ctx.sim.and_then(|sim| sim.get_alt_approx(wpos)).unwrap_or(0.0) as i32,
            noise: RandomField::new(ctx.rng.gen()),
            floors: (0..6)
                .scan(Vec2::zero(), |stair_tile, _| {
                    let (floor, st) = Floor::generate(&mut ctx, *stair_tile);
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

                let make_staircase = |pos: Vec3<i32>, radius: f32, inner_radius: f32, stretch| {
                    if (pos.xy().magnitude_squared() as f32) < radius.powf(2.0) {
                        if ((pos.x as f32).atan2(pos.y as f32) / (f32::consts::PI * 2.0) * stretch + pos.z as f32) % stretch < 3.0
                            || (pos.xy().magnitude_squared() as f32) < inner_radius.powf(2.0)
                        {
                            BlockMask::new(Block::new(BlockKind::Normal, Rgb::new(150, 150, 175)), 5)
                        } else {
                            BlockMask::new(Block::empty(), 1)
                        }
                    } else {
                        BlockMask::nothing()
                    }
                };

                let tile_pos = rpos.map(|e| e.div_euclid(TILE_SIZE));
                let tile_center = tile_pos * TILE_SIZE + TILE_SIZE / 2;

                let mut z = self.alt + 20;
                for floor in &self.floors {
                    match floor.sample(tile_pos) {
                        Some(Tile::DownStair) | Some(Tile::Empty) => {
                            z -= floor.solid_depth;
                            for _ in 0..floor.hollow_depth {
                                vol.set(Vec3::new(offs.x, offs.y, z), Block::empty());
                                z -= 1;
                            }
                        },
                        Some(Tile::UpStair) => {
                            for i in 0..floor.solid_depth + floor.hollow_depth {
                                let rtile_pos = rpos - tile_center;
                                let mut block = make_staircase(Vec3::new(rtile_pos.x, rtile_pos.y, z), TILE_SIZE as f32 / 2.0, 1.5, 13.0);
                                if i >= floor.solid_depth {
                                    block = block.resolve_with(BlockMask::new(Block::empty(), 1));
                                }
                                if let Some(block) = block.finish() {
                                    vol.set(Vec3::new(offs.x, offs.y, z), block);
                                }
                                z -= 1;
                            }
                        },
                        None => z -= floor.solid_depth + floor.hollow_depth,
                    }
                }
            }
        }
    }
}

const CARDINALS: [Vec2<i32>; 4] = [
    Vec2::new(0, 1),
    Vec2::new(1, 0),
    Vec2::new(0, -1),
    Vec2::new(-1, 0),
];

const TILE_SIZE: i32 = 17;

pub enum Tile {
    UpStair,
    DownStair,
    Empty,
}

pub struct Floor {
    tile_offset: Vec2<i32>,
    tiles: Grid<Tile>,
    solid_depth: i32,
    hollow_depth: i32,
}

impl Floor {
    pub fn generate(ctx: &mut GenCtx<impl Rng>, stair_tile: Vec2<i32>) -> (Self, Vec2<i32>) {
        let new_stair_tile = std::iter::from_fn(|| Some(FLOOR_SIZE.map(|sz| ctx.rng.gen_range(-sz / 2 + 1, sz / 2))))
            .find(|pos| *pos != stair_tile)
            .unwrap();

        const FLOOR_SIZE: Vec2<i32> = Vec2::new(12, 12);
        let tile_offset = -FLOOR_SIZE / 2;
        let this = Floor {
            tile_offset,
            tiles: Grid::populate_from(FLOOR_SIZE, |pos| {
                let tile_pos = tile_offset + pos;
                if tile_pos == stair_tile {
                    Tile::UpStair
                } else if tile_pos == new_stair_tile {
                    Tile::DownStair
                } else {
                    Tile::Empty
                }
            }),
            solid_depth: 13 * 3,
            hollow_depth: 13,
        };

        (this, new_stair_tile)
    }

    pub fn sample(&self, tile_pos: Vec2<i32>) -> Option<&Tile> {
        self.tiles.get(tile_pos - self.tile_offset)
    }
}
