use super::{RandomField, Sampler};
use crate::block::BlockGen;
use rayon::prelude::*;
use vek::*;

pub struct StructureGen2d {
    freq: u32,
    spread: u32,
    x_field: RandomField,
    y_field: RandomField,
    seed_field: RandomField,
}

pub type StructureField = (Vec2<i32>, u32);

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

    #[inline]
    fn sample_to_index_internal(freq: i32, pos: Vec2<i32>) -> Vec2<i32> {
        pos.map(|e| e.div_euclid(freq))
    }

    #[inline]
    pub fn sample_to_index(&self, pos: Vec2<i32>) -> Vec2<i32> {
        Self::sample_to_index_internal(self.freq as i32, pos)
    }

    #[inline]
    fn freq_offset(freq: i32) -> i32 {
        freq / 2
    }

    #[inline]
    fn spread_mul(spread: u32) -> u32 {
        spread * 2
    }

    #[inline]
    fn index_to_sample_internal(
        freq: i32,
        freq_offset: i32,
        spread: i32,
        spread_mul: u32,
        x_field: RandomField,
        y_field: RandomField,
        seed_field: RandomField,
        index: Vec2<i32>,
    ) -> StructureField {
        let center = index * freq + freq_offset;
        let pos = Vec3::from(center);
        (
            center
                + Vec2::new(
                    (x_field.get(pos) % spread_mul) as i32 - spread,
                    (y_field.get(pos) % spread_mul) as i32 - spread,
                ),
            seed_field.get(pos),
        )
    }

    /// Note: Generates all possible closest samples for elements in the range of min to max,
    /// *exclusive.*
    pub fn par_iter(
        &self,
        min: Vec2<i32>,
        max: Vec2<i32>,
    ) -> impl ParallelIterator<Item = StructureField> {
        let freq = self.freq;
        let spread = self.spread;
        let spread_mul = Self::spread_mul(spread);
        assert!(spread * 2 == spread_mul);
        assert!(spread_mul <= freq);
        let spread = spread as i32;
        let freq = freq as i32;
        let freq_offset = Self::freq_offset(freq);
        assert!(freq_offset * 2 == freq);

        let min_index = Self::sample_to_index_internal(freq, min) - 1;
        let max_index = Self::sample_to_index_internal(freq, max) + 1;
        assert!(min_index.x < max_index.x);
        // NOTE: xlen > 0
        let xlen = (max_index.x - min_index.x) as u32;
        assert!(min_index.y < max_index.y);
        // NOTE: ylen > 0
        let ylen = (max_index.y - min_index.y) as u32;
        // NOTE: Cannot fail, since every product of u32s fits in a u64.
        let len = ylen as u64 * xlen as u64;
        // NOTE: since iteration is *exclusive* for the initial range, it's fine that we don't go
        // up to the maximum value.
        // NOTE: we convert to usize first, and then iterate, because we want to make sure we get
        // a properly indexed parallel iterator that can deal with the whole range at once.
        let x_field = self.x_field;
        let y_field = self.y_field;
        let seed_field = self.seed_field;
        (0..len).into_par_iter().map(move |xy| {
            let index = min_index + Vec2::new((xy % xlen as u64) as i32, (xy / xlen as u64) as i32);
            Self::index_to_sample_internal(
                freq,
                freq_offset,
                spread,
                spread_mul,
                x_field,
                y_field,
                seed_field,
                index,
            )
        })
    }
}

impl Sampler<'static> for StructureGen2d {
    type Index = Vec2<i32>;
    type Sample = [StructureField; 9];

    fn get(&self, sample_pos: Self::Index) -> Self::Sample {
        let mut samples = [(Vec2::zero(), 0); 9];

        let freq = self.freq;
        let spread = self.spread;
        let spread_mul = Self::spread_mul(spread);
        let spread = spread as i32;
        let freq = freq as i32;
        let freq_offset = Self::freq_offset(freq);

        let sample_closest = Self::sample_to_index_internal(freq, sample_pos);

        for i in 0..3 {
            for j in 0..3 {
                let index = sample_closest + Vec2::new(i as i32, j as i32) - 1;
                let sample = Self::index_to_sample_internal(
                    freq,
                    freq_offset,
                    spread,
                    spread_mul,
                    self.x_field,
                    self.y_field,
                    self.seed_field,
                    index,
                );
                samples[i * 3 + j] = sample;
            }
        }

        samples
    }
}
