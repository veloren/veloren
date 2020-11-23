mod block_mask;
mod castle;
mod dungeon;
pub mod economy;
mod namegen;
mod settlement;

// Reexports
pub use self::{
    block_mask::BlockMask, castle::Castle, dungeon::Dungeon, economy::Economy,
    settlement::Settlement,
};

use crate::{column::ColumnSample, IndexRef};
use common::{
    generation::ChunkSupplement,
    terrain::Block,
    vol::{BaseVol, ReadVol, RectSizedVol, WriteVol},
};
use rand::Rng;
use serde::Deserialize;
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    pub castle: castle::Colors,
    pub dungeon: dungeon::Colors,
    pub settlement: settlement::Colors,
}

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
    Castle(Castle),
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

    pub fn castle(c: Castle) -> Self {
        Self {
            kind: SiteKind::Castle(c),
            economy: Economy::default(),
        }
    }

    pub fn radius(&self) -> f32 {
        match &self.kind {
            SiteKind::Settlement(s) => s.radius(),
            SiteKind::Dungeon(d) => d.radius(),
            SiteKind::Castle(c) => c.radius(),
        }
    }

    pub fn get_origin(&self) -> Vec2<i32> {
        match &self.kind {
            SiteKind::Settlement(s) => s.get_origin(),
            SiteKind::Dungeon(d) => d.get_origin(),
            SiteKind::Castle(c) => c.get_origin(),
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        match &self.kind {
            SiteKind::Settlement(s) => s.spawn_rules(wpos),
            SiteKind::Dungeon(d) => d.spawn_rules(wpos),
            SiteKind::Castle(c) => c.spawn_rules(wpos),
        }
    }

    pub fn name(&self) -> &str {
        match &self.kind {
            SiteKind::Settlement(s) => s.name(),
            SiteKind::Dungeon(d) => d.name(),
            SiteKind::Castle(c) => c.name(),
        }
    }

    pub fn apply_to<'a>(
        &'a self,
        index: IndexRef,
        wpos2d: Vec2<i32>,
        get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    ) {
        match &self.kind {
            SiteKind::Settlement(s) => s.apply_to(index, wpos2d, get_column, vol),
            SiteKind::Dungeon(d) => d.apply_to(index, wpos2d, get_column, vol),
            SiteKind::Castle(c) => c.apply_to(index, wpos2d, get_column, vol),
        }
    }

    pub fn apply_supplement<'a>(
        &'a self,
        // NOTE: Used only for dynamic elements like chests and entities!
        dynamic_rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        supplement: &mut ChunkSupplement,
    ) {
        match &self.kind {
            SiteKind::Settlement(s) => {
                s.apply_supplement(dynamic_rng, wpos2d, get_column, supplement)
            },
            SiteKind::Dungeon(d) => d.apply_supplement(dynamic_rng, wpos2d, get_column, supplement),
            SiteKind::Castle(c) => c.apply_supplement(dynamic_rng, wpos2d, get_column, supplement),
        }
    }
}
