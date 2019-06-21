use super::Sampler;
use vek::*;

pub struct RandomField {
    seed: u32,
}

impl RandomField {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }
}

impl Sampler for RandomField {
    type Index = Vec3<i32>;
    type Sample = u32;

    fn get(&self, pos: Self::Index) -> Self::Sample {
        let pos = pos.map(|e| (e * 13 + (1 << 31)) as u32);

        let next = self.seed.wrapping_mul(0x168E3D1F).wrapping_add(0xDEADBEAD);
        let next = next
            .wrapping_mul(133227)
            .wrapping_add(pos.x);
        let next = next.rotate_left(13).wrapping_add(318912) ^ 0x42133742;
        let next = next
            .wrapping_mul(938219)
            .wrapping_add(pos.y);
        let next = next.rotate_left(13).wrapping_add(318912) ^ 0x23341753;
        let next = next
            .wrapping_mul(938219)
            .wrapping_add(pos.z);
        let next = next.wrapping_add(313322) ^ 0xDEADBEEF;
        let next = next.wrapping_sub(929009) ^ 0xFF329DE3;
        let next = next.wrapping_add(422671) ^ 0x42892942;
        next.rotate_left(13)
    }
}
