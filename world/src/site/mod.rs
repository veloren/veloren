mod block_mask;
mod dungeon;
pub mod economy;
mod settlement;

// Reexports
pub use self::{block_mask::BlockMask, dungeon::Dungeon, economy::Economy, settlement::Settlement};

use crate::column::ColumnSample;
use common::{
    generation::ChunkSupplement,
    terrain::Block,
    vol::{BaseVol, ReadVol, RectSizedVol, Vox, WriteVol},
};
use rand::Rng;
use std::{fmt, sync::Arc};
use vek::*;

pub struct SpawnRules {
    pub trees: bool,
}

impl Default for SpawnRules {
    fn default() -> Self { Self { trees: true } }
}

pub struct Site {
    pub kind: SiteKind,
    pub economy: Economy,
}

pub enum SiteKind {
    Settlement(Settlement),
    Dungeon(Dungeon),
}

impl Site {
    pub fn settlement(s: Settlement) -> Self {
        Self {
            kind: SiteKind::Settlement(s),
            economy: Economy::default(),
        }
    }

    pub fn dungeon(d: Dungeon) -> Self {
        Self {
            kind: SiteKind::Dungeon(d),
            economy: Economy::default(),
        }
    }

    pub fn radius(&self) -> f32 {
        match &self.kind {
            SiteKind::Settlement(settlement) => settlement.radius(),
            SiteKind::Dungeon(dungeon) => dungeon.radius(),
        }
    }

    pub fn get_origin(&self) -> Vec2<i32> {
        match &self.kind {
            SiteKind::Settlement(s) => s.get_origin(),
            SiteKind::Dungeon(d) => d.get_origin(),
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        match &self.kind {
            SiteKind::Settlement(s) => s.spawn_rules(wpos),
            SiteKind::Dungeon(d) => d.spawn_rules(wpos),
        }
    }

    pub fn apply_to<'a>(
        &'a self,
        wpos2d: Vec2<i32>,
        get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    ) {
        match &self.kind {
            SiteKind::Settlement(settlement) => settlement.apply_to(wpos2d, get_column, vol),
            SiteKind::Dungeon(dungeon) => dungeon.apply_to(wpos2d, get_column, vol),
        }
    }

    pub fn apply_supplement<'a>(
        &'a self,
        rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        supplement: &mut ChunkSupplement,
    ) {
        match &self.kind {
            SiteKind::Settlement(settlement) => {
                settlement.apply_supplement(rng, wpos2d, get_column, supplement)
            },
            SiteKind::Dungeon(dungeon) => {
                dungeon.apply_supplement(rng, wpos2d, get_column, supplement)
            },
        }
    }
}
