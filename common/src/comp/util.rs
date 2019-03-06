// Library
use specs::{Component, NullStorage};
use vek::*;

// Pos

#[derive(Copy, Clone, Debug, Default)]
pub struct New;

impl Component for New {
    type Storage = NullStorage<Self>;
}
