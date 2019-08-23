mod town;

// Reexports
pub use self::town::{TownGen, TownState};

use crate::util::Sampler;
use common::terrain::Block;
use vek::*;

pub trait Generator<'a, T: 'a>:
    Sampler<'a, Index = (&'a T, Vec3<i32>), Sample = Option<Block>>
{
}
