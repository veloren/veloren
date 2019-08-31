use super::{RandomField, Sampler};
use std::f32;
use vek::*;

pub struct FastNoise {
    noise: RandomField,
}

impl FastNoise {
    pub const fn new(seed: u32) -> Self {
        Self {
            noise: RandomField::new(seed),
        }
    }

    fn noise_at(&self, pos: Vec3<i32>) -> f32 {
        (self.noise.get(pos) % 4096) as f32 / 4096.0
    }
}

impl Sampler<'static> for FastNoise {
    type Index = Vec3<f64>;
    type Sample = f32;

    fn get(&self, pos: Self::Index) -> Self::Sample {
        let near_pos = pos.map(|e| e.floor() as i32);

        let v000 = self.noise_at(near_pos + Vec3::new(0, 0, 0));
        let v100 = self.noise_at(near_pos + Vec3::new(1, 0, 0));
        let v010 = self.noise_at(near_pos + Vec3::new(0, 1, 0));
        let v110 = self.noise_at(near_pos + Vec3::new(1, 1, 0));
        let v001 = self.noise_at(near_pos + Vec3::new(0, 0, 1));
        let v101 = self.noise_at(near_pos + Vec3::new(1, 0, 1));
        let v011 = self.noise_at(near_pos + Vec3::new(0, 1, 1));
        let v111 = self.noise_at(near_pos + Vec3::new(1, 1, 1));

        let factor = pos.map(|e| 0.5 - (e.fract() as f32 * f32::consts::PI).cos() * 0.5);

        let x00 = Lerp::lerp(v000, v100, factor.x);
        let x10 = Lerp::lerp(v010, v110, factor.x);
        let x01 = Lerp::lerp(v001, v101, factor.x);
        let x11 = Lerp::lerp(v011, v111, factor.x);

        let y0 = Lerp::lerp(x00, x10, factor.y);
        let y1 = Lerp::lerp(x01, x11, factor.y);

        Lerp::lerp(y0, y1, factor.z) * 2.0 - 1.0
    }
}
