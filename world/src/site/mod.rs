mod settlement;

// Reexports
pub use self::settlement::Settlement;

use crate::{
    column::ColumnSample,
    util::{Grid, Sampler},
};
use common::{
    terrain::Block,
    vol::{BaseVol, RectSizedVol, WriteVol},
};
use std::sync::Arc;
use vek::*;

#[derive(Clone)]
pub enum Site {
    Settlement(Arc<Settlement>),
}

impl Site {
    pub fn radius(&self) -> f32 {
        match self {
            Site::Settlement(settlement) => settlement.radius(),
        }
    }

    pub fn get_surface(&self, wpos: Vec2<i32>) -> Option<Block> {
        match self {
            Site::Settlement(settlement) => settlement.get_surface(wpos),
        }
    }

    pub fn apply_to<'a>(
        &'a self,
        wpos2d: Vec2<i32>,
        get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + WriteVol),
    ) {
        match self {
            Site::Settlement(settlement) => settlement.apply_to(wpos2d, get_column, vol),
        }
    }
}

impl From<Settlement> for Site {
    fn from(settlement: Settlement) -> Self { Site::Settlement(Arc::new(settlement)) }
}
