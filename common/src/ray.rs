use crate::vol::{ReadVol, Vox};
use vek::*;

pub trait RayUntil<V: Vox> = FnMut(&V) -> bool;

pub struct Ray<'a, V: ReadVol, F: RayUntil<V::Vox>> {
    vol: &'a V,
    from: Vec3<f32>,
    to: Vec3<f32>,
    until: F,
    max_iter: usize,
}

impl<'a, V: ReadVol, F: RayUntil<V::Vox>> Ray<'a, V, F> {
    pub fn new(vol: &'a V, from: Vec3<f32>, to: Vec3<f32>, until: F) -> Self {
        Self {
            vol,
            from,
            to,
            until,
            max_iter: 100,
        }
    }

    pub fn until(self, f: F) -> Ray<'a, V, F> {
        Ray { until: f, ..self }
    }

    pub fn max_iter(mut self, max_iter: usize) -> Self {
        self.max_iter = max_iter;
        self
    }

    pub fn cast(mut self) -> (f32, Result<Option<&'a V::Vox>, V::Err>) {
        // TODO: Fully test this!

        const PLANCK: f32 = 0.001;

        let mut dist = 0.0;
        let dir = (self.to - self.from).normalized();

        let mut pos = self.from;
        let mut ipos = pos.map(|e| e as i32);

        for _ in 0..self.max_iter {
            pos = self.from + dir * dist;
            ipos = pos.map(|e| e as i32);

            match self.vol.get(ipos).map(|vox| (vox, (self.until)(vox))) {
                Ok((vox, true)) => return (dist, Ok(Some(vox))),
                Ok((_, false)) => {}
                Err(err) => return (dist, Err(err)),
            }

            let deltas =
                (dir.map(|e| if e < 0.0 { 0.0 } else { 1.0 }) - pos.map(|e| e.fract())) / dir;

            dist += deltas.reduce(f32::min).max(PLANCK);
        }

        (dist, Ok(None))
    }
}
