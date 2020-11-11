use super::{seed_expan, Sampler};
use vek::*;

#[derive(Clone, Copy)]
pub struct RandomField {
    seed: u32,
}

impl RandomField {
    pub const fn new(seed: u32) -> Self { Self { seed } }

    pub fn chance(&self, pos: Vec3<i32>, chance: f32) -> bool {
        (self.get(pos) % (1 << 16)) as f32 / ((1 << 16) as f32) < chance
    }
}

impl Sampler<'static> for RandomField {
    type Index = Vec3<i32>;
    type Sample = u32;

    fn get(&self, pos: Self::Index) -> Self::Sample {
        let pos = pos.map(|e| u32::from_le_bytes(e.to_le_bytes()));

        let mut a = self.seed;
        a = (a ^ 61) ^ (a >> 16);
        a = a.wrapping_add(a << 3);
        a ^= pos.x;
        a ^= a >> 4;
        a = a.wrapping_mul(0x27d4eb2d);
        a ^= a >> 15;
        a ^= pos.y;
        a = (a ^ 61) ^ (a >> 16);
        a = a.wrapping_add(a << 3);
        a ^= a >> 4;
        a ^= pos.z;
        a = a.wrapping_mul(0x27d4eb2d);
        a ^= a >> 15;
        a
    }
}

pub struct RandomPerm {
    seed: u32,
}

impl RandomPerm {
    pub const fn new(seed: u32) -> Self { Self { seed } }

    pub fn chance(&self, perm: u32, chance: f32) -> bool {
        (self.get(perm) % (1 << 16)) as f32 / ((1 << 16) as f32) < chance
    }
}

impl Sampler<'static> for RandomPerm {
    type Index = u32;
    type Sample = u32;

    fn get(&self, perm: Self::Index) -> Self::Sample {
        seed_expan::diffuse_mult(&[self.seed, perm])
    }
}
