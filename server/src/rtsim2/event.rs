use rtsim2::Event;
use common::terrain::Block;
use vek::*;

#[derive(Clone)]
pub struct OnBlockChange {
    pub wpos: Vec3<i32>,
    pub old: Block,
    pub new: Block,
}

impl Event for OnBlockChange {}
