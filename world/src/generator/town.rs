use super::Generator;
use crate::util::Sampler;
use common::terrain::Block;
use vek::*;

pub struct TownState;

pub struct TownGen;

impl<'a> Sampler<'a> for TownGen {
    type Index = (&'a TownState, Vec3<i32>);
    type Sample = Block;

    fn get(&self, (town, pos): Self::Index) -> Self::Sample {
        unimplemented!()
    }
}

impl<'a> Generator<'a, TownState> for TownGen {}
