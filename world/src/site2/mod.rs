mod tile;
mod plot;

use vek::*;
use common::store::{Store, Id};
use crate::util::Grid;
use self::{
    tile::TileGrid,
    plot::Plot,
};

pub struct Site {
    grid: TileGrid,
    plot: Store<Plot>,
}
