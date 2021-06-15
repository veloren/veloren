mod block_mask;
mod castle;
pub mod economy;
pub mod namegen;
mod settlement;
mod tree;

// Reexports
pub use self::{
    block_mask::BlockMask, castle::Castle, economy::Economy,
    settlement::Settlement, tree::Tree,
};

use crate::{column::ColumnSample, site2, Canvas};
use common::generation::ChunkSupplement;
use rand::Rng;
use serde::Deserialize;
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    pub castle: castle::Colors,
    pub dungeon: site2::plot::dungeon::Colors,
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
    Dungeon(site2::Site),
    Castle(Castle),
    Refactor(site2::Site),
    Tree(tree::Tree),
}

impl Site {
    pub fn settlement(s: Settlement) -> Self {
        Self {
            kind: SiteKind::Settlement(s),
            economy: Economy::default(),
        }
    }

    pub fn dungeon(d: site2::Site) -> Self {
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

    pub fn tree(t: tree::Tree) -> Self {
        Self {
            kind: SiteKind::Tree(t),
            economy: Economy::default(),
        }
    }

    pub fn radius(&self) -> f32 {
        match &self.kind {
            SiteKind::Settlement(s) => s.radius(),
            SiteKind::Dungeon(d) => d.radius(),
            SiteKind::Castle(c) => c.radius(),
            SiteKind::Refactor(s) => s.radius(),
            SiteKind::Tree(t) => t.radius(),
        }
    }

    pub fn get_origin(&self) -> Vec2<i32> {
        match &self.kind {
            SiteKind::Settlement(s) => s.get_origin(),
            SiteKind::Dungeon(d) => d.origin,
            SiteKind::Castle(c) => c.get_origin(),
            SiteKind::Refactor(s) => s.origin,
            SiteKind::Tree(t) => t.origin,
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        match &self.kind {
            SiteKind::Settlement(s) => s.spawn_rules(wpos),
            SiteKind::Dungeon(d) => d.spawn_rules(wpos),
            SiteKind::Castle(c) => c.spawn_rules(wpos),
            SiteKind::Refactor(s) => s.spawn_rules(wpos),
            SiteKind::Tree(t) => t.spawn_rules(wpos),
        }
    }

    pub fn name(&self) -> &str {
        match &self.kind {
            SiteKind::Settlement(s) => s.name(),
            SiteKind::Dungeon(d) => d.name(),
            SiteKind::Castle(c) => c.name(),
            SiteKind::Refactor(s) => s.name(),
            SiteKind::Tree(_) => "Giant Tree",
        }
    }

    pub fn apply_to<'a>(&'a self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        let info = canvas.info();
        let get_col = |wpos| info.col(wpos + info.wpos);
        match &self.kind {
            SiteKind::Settlement(s) => s.apply_to(canvas.index, canvas.wpos, get_col, canvas.chunk),
            SiteKind::Dungeon(d) => d.render(canvas, dynamic_rng),
            SiteKind::Castle(c) => c.apply_to(canvas.index, canvas.wpos, get_col, canvas.chunk),
            SiteKind::Refactor(s) => s.render(canvas, dynamic_rng),
            SiteKind::Tree(t) => t.render(canvas, dynamic_rng),
        }
    }

    pub fn apply_supplement<'a>(
        &'a self,
        // NOTE: Used only for dynamic elements like chests and entities!
        dynamic_rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        supplement: &mut ChunkSupplement,
        site_id: common::trade::SiteId,
    ) {
        match &self.kind {
            SiteKind::Settlement(s) => {
                let economy = common::trade::SiteInformation {
                    id: site_id,
                    unconsumed_stock: self
                        .economy
                        .unconsumed_stock
                        .iter()
                        .map(|(g, a)| (g.into(), *a))
                        .collect(),
                };
                s.apply_supplement(dynamic_rng, wpos2d, get_column, supplement, economy)
            },
            SiteKind::Dungeon(d) => {}, //d.apply_supplement(dynamic_rng, wpos2d, get_column, supplement),
            SiteKind::Castle(c) => c.apply_supplement(dynamic_rng, wpos2d, get_column, supplement),
            SiteKind::Refactor(_) => {},
            SiteKind::Tree(_) => {},
        }
    }

    pub fn do_economic_simulation(&self) -> bool { matches!(self.kind, SiteKind::Settlement(_)) }
}
