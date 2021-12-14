mod castle;
pub mod dungeon;
mod giant_tree;
mod gnarling;
mod house;
mod workshop;

pub use self::{
    castle::Castle, dungeon::Dungeon, giant_tree::GiantTree, gnarling::GnarlingFortification,
    house::House, workshop::Workshop,
};

use super::*;
use crate::util::DHashSet;
use common::path::Path;
use vek::*;

pub struct Plot {
    pub(crate) kind: PlotKind,
    pub(crate) root_tile: Vec2<i32>,
    pub(crate) tiles: DHashSet<Vec2<i32>>,
    pub(crate) seed: u32,
}

impl Plot {
    pub fn find_bounds(&self) -> Aabr<i32> {
        self.tiles
            .iter()
            .fold(Aabr::new_empty(self.root_tile), |b, t| {
                b.expanded_to_contain_point(*t)
            })
    }

    pub fn z_range(&self) -> Option<Range<i32>> {
        match &self.kind {
            PlotKind::House(house) => Some(house.z_range()),
            _ => None,
        }
    }

    pub fn kind(&self) -> &PlotKind { &self.kind }

    pub fn root_tile(&self) -> Vec2<i32> { self.root_tile }
}

pub enum PlotKind {
    House(House),
    Workshop(Workshop),
    Plaza,
    Castle(Castle),
    Road(Path<Vec2<i32>>),
    Dungeon(Dungeon),
    Gnarling(GnarlingFortification),
    GiantTree(GiantTree),
}
