use super::{seed_expan, Sampler};
use vek::*;

#[derive(Clone, Copy)]
pub struct RandomField {
    seed: u32,
}

impl RandomField {
    pub const fn new(seed: u32) -> Self { Self { seed } }
}

impl Sampler<'static> for RandomField {
    type Index = Vec3<i32>;
    type Sample = u32;

    fn get(&self, pos: Self::Index) -> Self::Sample {
        let pos = pos.map(|e| u32::from_le_bytes(e.to_le_bytes()));
        seed_expan::diffuse_mult(&[self.seed, pos.x, pos.y, pos.z])
    }
}

pub struct RandomPerm {
    seed: u32,
}

impl RandomPerm {
    pub const fn new(seed: u32) -> Self { Self { seed } }
}

impl Sampler<'static> for RandomPerm {
    type Index = u32;
    type Sample = u32;

    fn get(&self, perm: Self::Index) -> Self::Sample {
        seed_expan::diffuse_mult(&[self.seed, perm])
    }
}
