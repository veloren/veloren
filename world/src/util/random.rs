use super::Sampler;
use vek::*;

pub struct RandomField {
    seed: u32,
}

impl RandomField {
    pub const fn new(seed: u32) -> Self {
        Self { seed }
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
        a = a ^ pos.x;
        a = a ^ (a >> 4);
        a = a.wrapping_mul(0x27d4eb2d);
        a = a ^ (a >> 15);
        a = a ^ pos.y;
        a = (a ^ 61) ^ (a >> 16);
        a = a.wrapping_add(a << 3);
        a = a ^ (a >> 4);
        a = a ^ pos.z;
        a = a.wrapping_mul(0x27d4eb2d);
        a = a ^ (a >> 15);
        a
    }
}

pub struct RandomPerm {
    seed: u32,
}

impl RandomPerm {
    pub const fn new(seed: u32) -> Self {
        Self { seed }
    }
}

impl Sampler<'static> for RandomPerm {
    type Index = u32;
    type Sample = u32;

    fn get(&self, perm: Self::Index) -> Self::Sample {
        let a = self
            .seed
            .wrapping_mul(3471)
            .wrapping_add(perm)
            .wrapping_add(0x3BE7172B)
            .wrapping_mul(perm)
            .wrapping_add(0x172A3BE1);
        let b = a.wrapping_mul(a);
        b ^ (a >> 17) ^ b >> 15
    }
}
