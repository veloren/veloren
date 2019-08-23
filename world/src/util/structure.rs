use super::{RandomField, Sampler};
use vek::*;

pub struct StructureGen2d {
    freq: u32,
    spread: u32,
    x_field: RandomField,
    y_field: RandomField,
    seed_field: RandomField,
}

impl StructureGen2d {
    pub fn new(seed: u32, freq: u32, spread: u32) -> Self {
        Self {
            freq,
            spread,
            x_field: RandomField::new(seed + 0),
            y_field: RandomField::new(seed + 1),
            seed_field: RandomField::new(seed + 2),
        }
    }
}

impl Sampler<'static> for StructureGen2d {
    type Index = Vec2<i32>;
    type Sample = [(Vec2<i32>, u32); 9];

    fn get(&self, sample_pos: Self::Index) -> Self::Sample {
        let mut samples = [(Vec2::zero(), 0); 9];

        let sample_closest = sample_pos.map(|e| e - e.rem_euclid(self.freq as i32));

        for i in 0..3 {
            for j in 0..3 {
                let center = sample_closest
                    + Vec2::new(i, j).map(|e| e as i32 - 1) * self.freq as i32
                    + self.freq as i32 / 2;
                samples[i * 3 + j] = (
                    center
                        + Vec2::new(
                            (self.x_field.get(Vec3::from(center)) % (self.spread * 2)) as i32
                                - self.spread as i32,
                            (self.y_field.get(Vec3::from(center)) % (self.spread * 2)) as i32
                                - self.spread as i32,
                        ),
                    self.seed_field.get(Vec3::from(center)),
                );
            }
        }

        samples
    }
}
