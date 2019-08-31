use super::{RandomPerm, Sampler};
use vek::*;

const UNIT_CHOICES: [(Vec2<i32>, Vec2<i32>); 8] = [
    (Vec2 { x: 1, y: 0 }, Vec2 { x: 0, y: 1 }),
    (Vec2 { x: 1, y: 0 }, Vec2 { x: 0, y: -1 }),
    (Vec2 { x: -1, y: 0 }, Vec2 { x: 0, y: 1 }),
    (Vec2 { x: -1, y: 0 }, Vec2 { x: 0, y: -1 }),
    (Vec2 { x: 0, y: 1 }, Vec2 { x: 1, y: 0 }),
    (Vec2 { x: 0, y: 1 }, Vec2 { x: -1, y: 0 }),
    (Vec2 { x: 0, y: -1 }, Vec2 { x: 1, y: 0 }),
    (Vec2 { x: 0, y: -1 }, Vec2 { x: -1, y: 0 }),
];

pub struct UnitChooser {
    perm: RandomPerm,
}

impl UnitChooser {
    pub const fn new(seed: u32) -> Self {
        Self {
            perm: RandomPerm::new(seed),
        }
    }
}

impl Sampler<'static> for UnitChooser {
    type Index = u32;
    type Sample = (Vec2<i32>, Vec2<i32>);

    fn get(&self, perm: Self::Index) -> Self::Sample {
        UNIT_CHOICES[self.perm.get(perm) as usize % UNIT_CHOICES.len()]
    }
}
