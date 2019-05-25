use vek::*;

pub struct StructureGen2d {
    seed: u32,
    freq: u32,
    spread: u32,
}

impl StructureGen2d {
    pub fn new(seed: u32, freq: u32, spread: u32) -> Self {
        Self {
            seed,
            freq,
            spread,
        }
    }

    fn random(&self, seed: u32, pos: Vec2<i32>) -> u32 {
        let pos = pos.map(|e| (e * 13 + (1 << 31)) as u32);

        let next = (self.seed + seed).wrapping_mul(0x168E3D1F).wrapping_add(0xDEADBEAD);
        let next = next.rotate_left(13).wrapping_mul(133227).wrapping_add(pos.x);
        let next = next.rotate_left(13).wrapping_mul(318912) ^ 0x42133742;
        let next = next.rotate_left(13).wrapping_mul(938219).wrapping_add(pos.y);
        let next = next.rotate_left(13).wrapping_mul(313322) ^ 0xDEADBEEF;
        let next = next.rotate_left(13).wrapping_mul(929009) ^ 0xFF329DE3;
        let next = next.rotate_left(13).wrapping_mul(422671) ^ 0x42892942;
        next
    }

    pub fn sample(&self, sample_pos: Vec2<i32>) -> [(Vec2<i32>, u32); 9] {
        let mut samples = [(Vec2::zero(), 0); 9];

        let sample_closest = sample_pos.map(|e| e - e.rem_euclid(self.freq as i32));

        for i in 0..3 {
            for j in 0..3 {
                let center = sample_closest
                    + Vec2::new(i, j).map(|e| e as i32 - 1) * self.freq as i32
                    + self.freq as i32 / 2;
                samples[i * 3 + j] = (center + Vec2::new(
                    (self.random(1, center) % (self.spread * 2)) as i32 - self.spread as i32,
                    (self.random(2, center) % (self.spread * 2)) as i32 - self.spread as i32,
                ), self.random(3, center));
            }
        }

        samples
    }
}
