use super::Generator;
use crate::util::Sampler;
use common::terrain::{Block, BlockKind};
use vek::*;

#[derive(Clone)]
pub struct TownState;

pub struct TownGen;

impl<'a> Sampler<'a> for TownGen {
    type Index = (&'a TownState, Vec3<i32>);
    type Sample = Option<Block>;

    fn get(&self, (town, pos): Self::Index) -> Self::Sample {
        if pos.z < 150 {
            Some(Block::new(BlockKind::Normal, Rgb::broadcast(255)))
        } else {
            None
        }
    }
}

impl<'a> Generator<'a, TownState> for TownGen {}
