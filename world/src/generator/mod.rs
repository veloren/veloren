mod town;

// Reexports
pub use self::town::{TownGen, TownState};

use crate::{column::ColumnSample, util::Sampler};
use common::terrain::Block;
use vek::*;

pub trait Generator<'a, T: 'a>:
    Sampler<'a, Index = (&'a T, Vec3<i32>, &'a ColumnSample<'a>, f32), Sample = Option<Block>>
{
    fn get_z_limits(&self, state: &'a T, wpos: Vec2<i32>, sample: &ColumnSample) -> (f32, f32);
}
