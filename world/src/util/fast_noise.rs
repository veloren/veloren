use super::{RandomField, Sampler};
use std::{f32, ops::Add};
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

        let factor = pos.map(|e| {
            let f = e.fract().add(1.0).fract() as f32;
            f.powf(2.0) * (3.0 - 2.0 * f)
        });

        let x00 = v000 + factor.x * (v100 - v000);
        let x10 = v010 + factor.x * (v110 - v010);
        let x01 = v001 + factor.x * (v101 - v001);
        let x11 = v011 + factor.x * (v111 - v011);

        let y0 = x00 + factor.y * (x10 - x00);
        let y1 = x01 + factor.y * (x11 - x01);

        (y0 + factor.z * (y1 - y0)) * 2.0 - 1.0
    }
}
