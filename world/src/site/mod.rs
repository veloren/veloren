mod block_mask;
mod castle;
mod dungeon;
pub mod economy;
pub mod namegen;
mod settlement;

// Reexports
pub use self::{
    block_mask::BlockMask, castle::Castle, dungeon::Dungeon, economy::Economy,
    settlement::Settlement,
};

use crate::{
    column::ColumnSample,
    IndexRef,
    site2,
    Canvas,
};
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
    Refactor(site2::Site),
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

    pub fn refactor(s: site2::Site) -> Self {
        Self {
            kind: SiteKind::Refactor(s),
            economy: Economy::default(),
        }
    }

    pub fn radius(&self) -> f32 {
        match &self.kind {
            SiteKind::Settlement(s) => s.radius(),
            SiteKind::Dungeon(d) => d.radius(),
            SiteKind::Castle(c) => c.radius(),
            SiteKind::Refactor(s) => s.radius(),
        }
    }

    pub fn get_origin(&self) -> Vec2<i32> {
        match &self.kind {
            SiteKind::Settlement(s) => s.get_origin(),
            SiteKind::Dungeon(d) => d.get_origin(),
            SiteKind::Castle(c) => c.get_origin(),
            SiteKind::Refactor(s) => s.origin,
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        match &self.kind {
            SiteKind::Settlement(s) => s.spawn_rules(wpos),
            SiteKind::Dungeon(d) => d.spawn_rules(wpos),
            SiteKind::Castle(c) => c.spawn_rules(wpos),
            SiteKind::Refactor(s) => s.spawn_rules(wpos),
        }
    }

    pub fn name(&self) -> &str {
        match &self.kind {
            SiteKind::Settlement(s) => s.name(),
            SiteKind::Dungeon(d) => d.name(),
            SiteKind::Castle(c) => c.name(),
            SiteKind::Refactor(s) => "Experimental",
        }
    }

    pub fn apply_to<'a>(
        &'a self,
        canvas: &mut Canvas,
        dynamic_rng: &mut impl Rng,
    ) {
        let info = canvas.info();
        let get_col = |wpos| info.col(wpos + info.wpos);
        match &self.kind {
            SiteKind::Settlement(s) => s.apply_to(canvas.index, canvas.wpos, get_col, canvas.chunk),
            SiteKind::Dungeon(d) => d.apply_to(canvas.index, canvas.wpos, get_col, canvas.chunk),
            SiteKind::Castle(c) => c.apply_to(canvas.index, canvas.wpos, get_col, canvas.chunk),
            SiteKind::Refactor(s) => {
                s.render(canvas, dynamic_rng);
            },
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
            SiteKind::Refactor(_) => {},
        }
    }
}
