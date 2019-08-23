mod town;

// Reexports
pub use self::town::TownGen;

use crate::util::Sampler;
use common::terrain::Block;
use vek::*;

pub trait Generator<'a, T: 'a>: Sampler<'a, Index = (&'a T, Vec3<i32>), Sample = Block> {}
