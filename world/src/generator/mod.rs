mod town;

// Reexports
pub use self::town::{TownGen, TownState};

use crate::{column::ColumnSample, util::Sampler};
use common::terrain::Block;
use vek::*;

#[derive(Copy, Clone, Debug)]
pub struct SpawnRules {
    pub trees: bool,
    pub cliffs: bool,
}

impl Default for SpawnRules {
    fn default() -> Self {
        Self {
            trees: true,
            cliffs: true,
        }
    }
}

impl SpawnRules {
    pub fn and(self, other: Self) -> Self {
        Self {
            trees: self.trees && other.trees,
            cliffs: self.cliffs && other.cliffs,
        }
    }
}

pub trait Generator<'a, T: 'a>:
    Sampler<'a, Index = (&'a T, Vec3<i32>, &'a ColumnSample<'a>, f32), Sample = Option<Block>>
{
    fn get_z_limits(&self, state: &'a T, wpos: Vec2<i32>, sample: &ColumnSample) -> (f32, f32);
    fn spawn_rules(&self, town: &'a TownState, wpos: Vec2<i32>) -> SpawnRules;
}
