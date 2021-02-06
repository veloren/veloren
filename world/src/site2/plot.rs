use crate::util::DHashSet;
use vek::*;

pub struct Plot {
    kind: PlotKind,
    root_tile: Vec2<i32>,
    tiles: DHashSet<Vec2<i32>>,
}

impl Plot {
    pub fn find_bounds(&self) -> Aabr<i32> {
        self.tiles
            .iter()
            .fold(Aabr::new_empty(self.root_tile), |b, t| {
                b.expanded_to_contain_point(*t)
            })
    }
}

pub enum PlotKind {
    Field,
    House,
}
