use super::*;

const TILE_SIZE: u32 = 7;
const ZONE_SIZE: u32 = 16;
const ZONE_RADIUS: u32 = 16;
const TILE_RADIUS: u32 = ZONE_SIZE * ZONE_RADIUS;
const MAX_BLOCK_RADIUS: u32 = TILE_SIZE * TILE_RADIUS;

pub struct TileGrid {
    zones: Grid<Option<Grid<Option<Tile>>>>,
}

impl TileGrid {
    pub fn new() -> Self {
        Self {
            zones: Grid::populate_from(Vec2::broadcast(ZONE_RADIUS as i32 * 2 + 1), |_| None),
        }
    }

    pub fn get(&self, tpos: Vec2<i32>) -> Option<&Tile> {
        let tpos = tpos + TILE_RADIUS as i32;
        self.zones
            .get(tpos)
            .and_then(|zone| zone.as_ref()?.get(tpos.map(|e| e.rem_euclid(ZONE_SIZE as i32))))
            .and_then(|tile| tile.as_ref())
    }

    pub fn get_mut(&mut self, tpos: Vec2<i32>) -> Option<&mut Tile> {
        let tpos = tpos + TILE_RADIUS as i32;
        self.zones
            .get_mut(tpos)
            .and_then(|zone| zone
                .get_or_insert_with(|| Grid::populate_from(Vec2::broadcast(ZONE_RADIUS as i32 * 2 + 1), |_| None))
                .get_mut(tpos.map(|e| e.rem_euclid(ZONE_SIZE as i32)))
                .map(|tile| tile.get_or_insert_with(|| Tile::empty())))
    }
}

pub struct Tile {
    plot: Option<Id<Plot>>,
}

impl Tile {
    pub fn empty() -> Self {
        Self {
            plot: None,
        }
    }
}
