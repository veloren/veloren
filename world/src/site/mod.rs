mod block_mask;
mod castle;
pub mod economy;
pub mod namegen;
pub mod settlement;
mod tree;

// Reexports
pub use self::{
    block_mask::BlockMask, castle::Castle, economy::Economy, settlement::Settlement, tree::Tree,
};

pub use common::terrain::site::{DungeonKindMeta, SettlementKindMeta, SiteKindMeta};

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
    pub max_warp: f32,
    pub paths: bool,
    pub waypoints: bool,
}

impl SpawnRules {
    #[must_use]
    pub fn combine(self, other: Self) -> Self {
        // Should be commutative
        Self {
            trees: self.trees && other.trees,
            max_warp: self.max_warp.min(other.max_warp),
            paths: self.paths && other.paths,
            waypoints: self.waypoints && other.waypoints,
        }
    }
}

impl Default for SpawnRules {
    fn default() -> Self {
        Self {
            trees: true,
            max_warp: 1.0,
            paths: true,
            waypoints: true,
        }
    }
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
    CliffTown(site2::Site),
    SavannahPit(site2::Site),
    Tree(Tree),
    DesertCity(site2::Site),
    ChapelSite(site2::Site),
    GiantTree(site2::Site),
    Gnarling(site2::Site),
    Bridge(site2::Site),
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

    pub fn gnarling(g: site2::Site) -> Self {
        Self {
            kind: SiteKind::Gnarling(g),
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

    pub fn cliff_town(ct: site2::Site) -> Self {
        Self {
            kind: SiteKind::CliffTown(ct),
            economy: Economy::default(),
        }
    }

    pub fn savannah_pit(sp: site2::Site) -> Self {
        Self {
            kind: SiteKind::SavannahPit(sp),
            economy: Economy::default(),
        }
    }

    pub fn desert_city(dc: site2::Site) -> Self {
        Self {
            kind: SiteKind::DesertCity(dc),
            economy: Economy::default(),
        }
    }

    pub fn chapel_site(p: site2::Site) -> Self {
        Self {
            kind: SiteKind::ChapelSite(p),
            economy: Economy::default(),
        }
    }

    pub fn tree(t: Tree) -> Self {
        Self {
            kind: SiteKind::Tree(t),
            economy: Economy::default(),
        }
    }

    pub fn giant_tree(gt: site2::Site) -> Self {
        Self {
            kind: SiteKind::GiantTree(gt),
            economy: Economy::default(),
        }
    }

    pub fn bridge(b: site2::Site) -> Self {
        Self {
            kind: SiteKind::Bridge(b),
            economy: Economy::default(),
        }
    }

    pub fn radius(&self) -> f32 {
        match &self.kind {
            SiteKind::Settlement(s) => s.radius(),
            SiteKind::Dungeon(d) => d.radius(),
            SiteKind::Castle(c) => c.radius(),
            SiteKind::Refactor(s) => s.radius(),
            SiteKind::CliffTown(ct) => ct.radius(),
            SiteKind::SavannahPit(sp) => sp.radius(),
            SiteKind::DesertCity(dc) => dc.radius(),
            SiteKind::ChapelSite(p) => p.radius(),
            SiteKind::Tree(t) => t.radius(),
            SiteKind::GiantTree(gt) => gt.radius(),
            SiteKind::Gnarling(g) => g.radius(),
            SiteKind::Bridge(b) => b.radius(),
        }
    }

    pub fn get_origin(&self) -> Vec2<i32> {
        match &self.kind {
            SiteKind::Settlement(s) => s.get_origin(),
            SiteKind::Dungeon(d) => d.origin,
            SiteKind::Castle(c) => c.get_origin(),
            SiteKind::Refactor(s) => s.origin,
            SiteKind::CliffTown(ct) => ct.origin,
            SiteKind::SavannahPit(sp) => sp.origin,
            SiteKind::DesertCity(dc) => dc.origin,
            SiteKind::ChapelSite(p) => p.origin,
            SiteKind::Tree(t) => t.origin,
            SiteKind::GiantTree(gt) => gt.origin,
            SiteKind::Gnarling(g) => g.origin,
            SiteKind::Bridge(b) => b.origin,
        }
    }

    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        match &self.kind {
            SiteKind::Settlement(s) => s.spawn_rules(wpos),
            SiteKind::Dungeon(d) => d.spawn_rules(wpos),
            SiteKind::Castle(c) => c.spawn_rules(wpos),
            SiteKind::Refactor(s) => s.spawn_rules(wpos),
            SiteKind::CliffTown(ct) => ct.spawn_rules(wpos),
            SiteKind::SavannahPit(sp) => sp.spawn_rules(wpos),
            SiteKind::DesertCity(dc) => dc.spawn_rules(wpos),
            SiteKind::ChapelSite(p) => p.spawn_rules(wpos),
            SiteKind::Tree(t) => t.spawn_rules(wpos),
            SiteKind::GiantTree(gt) => gt.spawn_rules(wpos),
            SiteKind::Gnarling(g) => g.spawn_rules(wpos),
            SiteKind::Bridge(b) => b.spawn_rules(wpos),
        }
    }

    pub fn name(&self) -> &str {
        match &self.kind {
            SiteKind::Settlement(s) => s.name(),
            SiteKind::Dungeon(d) => d.name(),
            SiteKind::Castle(c) => c.name(),
            SiteKind::Refactor(s) => s.name(),
            SiteKind::CliffTown(ct) => ct.name(),
            SiteKind::SavannahPit(sp) => sp.name(),
            SiteKind::DesertCity(dc) => dc.name(),
            SiteKind::ChapelSite(p) => p.name(),
            SiteKind::Tree(_) => "Giant Tree",
            SiteKind::GiantTree(gt) => gt.name(),
            SiteKind::Gnarling(g) => g.name(),
            SiteKind::Bridge(b) => b.name(),
        }
    }

    pub fn trade_information(
        &self,
        site_id: common::trade::SiteId,
    ) -> Option<common::trade::SiteInformation> {
        match &self.kind {
            SiteKind::Settlement(_)
            | SiteKind::Refactor(_)
            | SiteKind::CliffTown(_)
            | SiteKind::SavannahPit(_)
            | SiteKind::DesertCity(_) => Some(common::trade::SiteInformation {
                id: site_id,
                unconsumed_stock: self.economy.get_available_stock(),
            }),
            _ => None,
        }
    }

    pub fn apply_to(&self, canvas: &mut Canvas, dynamic_rng: &mut impl Rng) {
        let info = canvas.info();
        let get_col = |wpos| info.col(wpos + info.wpos);
        match &self.kind {
            SiteKind::Settlement(s) => s.apply_to(canvas.index, canvas.wpos, get_col, canvas.chunk),
            SiteKind::Dungeon(d) => d.render(canvas, dynamic_rng),
            SiteKind::Castle(c) => c.apply_to(canvas.index, canvas.wpos, get_col, canvas.chunk),
            SiteKind::Refactor(s) => s.render(canvas, dynamic_rng),
            SiteKind::CliffTown(ct) => ct.render(canvas, dynamic_rng),
            SiteKind::SavannahPit(sp) => sp.render(canvas, dynamic_rng),
            SiteKind::DesertCity(dc) => dc.render(canvas, dynamic_rng),
            SiteKind::ChapelSite(p) => p.render(canvas, dynamic_rng),
            SiteKind::Tree(t) => t.render(canvas, dynamic_rng),
            SiteKind::GiantTree(gt) => gt.render(canvas, dynamic_rng),
            SiteKind::Gnarling(g) => g.render(canvas, dynamic_rng),
            SiteKind::Bridge(b) => b.render(canvas, dynamic_rng),
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
                let economy = self
                    .trade_information(site_id)
                    .expect("Settlement has no economy");
                s.apply_supplement(dynamic_rng, wpos2d, get_column, supplement, economy)
            },
            SiteKind::Dungeon(d) => d.apply_supplement(dynamic_rng, wpos2d, supplement),
            SiteKind::Castle(c) => c.apply_supplement(dynamic_rng, wpos2d, get_column, supplement),
            SiteKind::Refactor(_) => {},
            SiteKind::CliffTown(_) => {},
            SiteKind::SavannahPit(_) => {},
            SiteKind::DesertCity(_) => {},
            SiteKind::ChapelSite(p) => p.apply_supplement(dynamic_rng, wpos2d, supplement),
            SiteKind::Tree(_) => {},
            SiteKind::GiantTree(gt) => gt.apply_supplement(dynamic_rng, wpos2d, supplement),
            SiteKind::Gnarling(g) => g.apply_supplement(dynamic_rng, wpos2d, supplement),
            SiteKind::Bridge(b) => b.apply_supplement(dynamic_rng, wpos2d, supplement),
        }
    }

    pub fn do_economic_simulation(&self) -> bool {
        matches!(
            self.kind,
            SiteKind::Refactor(_)
                | SiteKind::CliffTown(_)
                | SiteKind::SavannahPit(_)
                | SiteKind::DesertCity(_)
                | SiteKind::Settlement(_)
        )
    }
}

impl SiteKind {
    pub fn convert_to_meta(&self) -> Option<SiteKindMeta> {
        match self {
            SiteKind::Refactor(_) | SiteKind::Settlement(_) => {
                Some(SiteKindMeta::Settlement(SettlementKindMeta::Default))
            },
            SiteKind::CliffTown(_) => Some(SiteKindMeta::Settlement(SettlementKindMeta::CliffTown)),
            SiteKind::SavannahPit(_) => {
                Some(SiteKindMeta::Settlement(SettlementKindMeta::SavannahPit))
            },
            SiteKind::DesertCity(_) => {
                Some(SiteKindMeta::Settlement(SettlementKindMeta::DesertCity))
            },
            SiteKind::Dungeon(_) => Some(SiteKindMeta::Dungeon(DungeonKindMeta::Old)),
            SiteKind::Gnarling(_) => Some(SiteKindMeta::Dungeon(DungeonKindMeta::Gnarling)),
            _ => None,
        }
    }
}
