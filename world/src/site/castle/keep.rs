use vek::*;
use crate::{
    util::{attempt, Grid, RandomField, Sampler, CARDINALS, DIRS},
};

pub struct Keep {
    offset: Vec2<i32>,
    cols: Grid<KeepCol>,
}

const KEEP_CELL_STOREY: i32 = 12;

pub struct KeepCol {
    z_offset: i32,
    storeys: Vec<KeepCell>,
}

enum KeepCell {
    Cube,
}
