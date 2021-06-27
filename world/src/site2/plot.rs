mod castle;
pub mod dungeon;
mod house;

pub use self::{castle::Castle, dungeon::Dungeon, house::House};

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

    pub fn kind(&self) -> &PlotKind { &self.kind }
}

pub enum PlotKind {
    House(House),
    Plaza,
    Castle(Castle),
    Road(Path<Vec2<i32>>),
    Dungeon(Dungeon),
}
