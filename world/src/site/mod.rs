mod settlement;

// Reexports
pub use self::settlement::Settlement;

use crate::{
    block::ZCache,
    column::ColumnSample,
    util::{Grid, Sampler},
};
use common::{
    terrain::Block,
    vol::{BaseVol, WriteVol},
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

    pub fn apply_to(
        &self,
        wpos2d: Vec2<i32>,
        zcaches: &Grid<Option<ZCache>>,
        vol: &mut (impl BaseVol<Vox = Block> + WriteVol),
    ) {
        match self {
            Site::Settlement(settlement) => settlement.apply_to(wpos2d, zcaches, vol),
        }
    }
}

impl From<Settlement> for Site {
    fn from(settlement: Settlement) -> Self { Site::Settlement(Arc::new(settlement)) }
}
