use super::*;
use common::spiral::Spiral2d;
use std::ops::Range;

pub const TILE_SIZE: u32 = 7;
pub const ZONE_SIZE: u32 = 16;
pub const ZONE_RADIUS: u32 = 16;
pub const TILE_RADIUS: u32 = ZONE_SIZE * ZONE_RADIUS;
pub const MAX_BLOCK_RADIUS: u32 = TILE_SIZE * TILE_RADIUS;

pub struct TileGrid {
    zones: Grid<Option<Grid<Option<Tile>>>>,
}

impl Default for TileGrid {
    fn default() -> Self {
        Self {
            zones: Grid::populate_from(Vec2::broadcast(ZONE_RADIUS as i32 * 2 + 1), |_| None),
        }
    }
}

impl TileGrid {
    pub fn get(&self, tpos: Vec2<i32>) -> &Tile {
        static EMPTY: Tile = Tile::empty();

        let tpos = tpos + TILE_RADIUS as i32;
        self.zones
            .get(tpos.map(|e| e.div_euclid(ZONE_SIZE as i32)))
            .and_then(|zone| {
                zone.as_ref()?
                    .get(tpos.map(|e| e.rem_euclid(ZONE_SIZE as i32)))
            })
            .and_then(|tile| tile.as_ref())
            .unwrap_or(&EMPTY)
    }

    pub fn get_mut(&mut self, tpos: Vec2<i32>) -> Option<&mut Tile> {
        let tpos = tpos + TILE_RADIUS as i32;
        self.zones.get_mut(tpos.map(|e| e.div_euclid(ZONE_SIZE as i32))).and_then(|zone| {
            zone.get_or_insert_with(|| {
                Grid::populate_from(Vec2::broadcast(ZONE_SIZE as i32), |_| None)
            })
            .get_mut(tpos.map(|e| e.rem_euclid(ZONE_SIZE as i32)))
            .map(|tile| tile.get_or_insert_with(|| Tile::empty()))
        })
    }

    pub fn set(&mut self, tpos: Vec2<i32>, tile: Tile) -> Option<Tile> {
        self.get_mut(tpos).map(|t| std::mem::replace(t, tile))
    }

    pub fn find_near<R>(&self, tpos: Vec2<i32>, f: impl Fn(Vec2<i32>, &Tile) -> Option<R>) -> Option<(R, Vec2<i32>)> {
        const MAX_SEARCH_RADIUS_BLOCKS: u32 = 70;
        const MAX_SEARCH_CELLS: u32 = ((MAX_SEARCH_RADIUS_BLOCKS / TILE_SIZE) * 2 + 1).pow(2);
        Spiral2d::new()
            .take(MAX_SEARCH_CELLS as usize)
            .map(|r| tpos + r)
            .find_map(|tpos| (&f)(tpos, self.get(tpos)).zip(Some(tpos)))
    }

    pub fn grow_aabr(&self, center: Vec2<i32>, area_range: Range<u32>, min_dims: Extent2<u32>) -> Result<Aabr<i32>, Aabr<i32>> {
        let mut aabr = Aabr { min: center, max: center + 1 };

        if !self.get(center).is_empty() {
            return Err(aabr);
        };

        let mut last_growth = 0;
        for i in 0.. {
            if i - last_growth >= 4 {
                break;
            } else if aabr.size().product() + if i % 2 == 0 { aabr.size().h } else { aabr.size().w } > area_range.end as i32 {
                break;
            } else {
                match i % 4 {
                    0 if (aabr.min.y..aabr.max.y + 1).all(|y| self.get(Vec2::new(aabr.max.x, y)).is_empty()) => {
                        aabr.max.x += 1;
                        last_growth = i;
                    },
                    1 if (aabr.min.x..aabr.max.x + 1).all(|x| self.get(Vec2::new(x, aabr.max.y)).is_empty()) => {
                        aabr.max.y += 1;
                        last_growth = i;
                    },
                    2 if (aabr.min.y..aabr.max.y + 1).all(|y| self.get(Vec2::new(aabr.min.x - 1, y)).is_empty()) => {
                        aabr.min.x -= 1;
                        last_growth = i;
                    },
                    3 if (aabr.min.x..aabr.max.x + 1).all(|x| self.get(Vec2::new(x, aabr.min.y - 1)).is_empty()) => {
                        aabr.min.y -= 1;
                        last_growth = i;
                    },
                    _ => {},
                }
            }
        }

        if aabr.size().product() as u32 >= area_range.start
            && aabr.size().w as u32 >= min_dims.w
            && aabr.size().h as u32 >= min_dims.h
        {
            Ok(aabr)
        } else {
            Err(aabr)
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum TileKind {
    Empty,
    Field,
    Road,
    Building { levels: u32 },
    Castle,
    Wall,
}

#[derive(Clone)]
pub struct Tile {
    pub(crate) kind: TileKind,
    pub(crate) plot: Option<Id<Plot>>,
}

impl Tile {
    pub const fn empty() -> Self {
        Self {
            kind: TileKind::Empty,
            plot: None,
        }
    }

    /// Create a tile that is not associated with any plot.
    pub const fn free(kind: TileKind) -> Self {
        Self {
            kind,
            plot: None,
        }
    }

    pub fn is_empty(&self) -> bool { self.kind == TileKind::Empty }

    pub fn is_obstacle(&self) -> bool {
        matches!(
            self.kind,
            TileKind::Building { .. }
            | TileKind::Castle
            | TileKind::Wall
        )
    }
}
