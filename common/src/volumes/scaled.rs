use crate::vol::{BaseVol, ReadVol, SizedVol, Vox};
use vek::*;

pub struct Scaled<'a, V> {
    pub inner: &'a V,
    pub scale: Vec3<f32>,
}

impl<'a, V: BaseVol> BaseVol for Scaled<'a, V> {
    type Error = V::Error;
    type Vox = V::Vox;
}

impl<'a, V: ReadVol> ReadVol for Scaled<'a, V> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, Self::Error> {
        let pos = pos.map2(self.scale, |e, scale| (e as f32 / scale).trunc() as i32);
        let search_size = (Vec3::one() / self.scale).map(|e: f32| e.round() as i32);
        let range_iter = |x| {
            std::iter::successors(Some(0), |x| Some(if *x < 0 { -*x } else { -(*x + 1) }))
                .take(x as usize * 2)
        };
        range_iter(search_size.x / 2)
            .map(|i| {
                range_iter(search_size.y / 2)
                    .map(move |j| range_iter(search_size.z / 2).map(move |k| Vec3::new(i, j, k)))
            })
            .flatten()
            .flatten()
            .map(|offs| self.inner.get(pos + offs))
            .find(|vox| vox.as_ref().map(|v| !v.is_empty()).unwrap_or(false))
            .unwrap_or_else(|| self.inner.get(pos))
    }
}

impl<'a, V: SizedVol> SizedVol for Scaled<'a, V> {
    #[inline(always)]
    fn lower_bound(&self) -> Vec3<i32> {
        self.inner
            .lower_bound()
            .map2(self.scale, |e, scale| (e as f32 * scale).floor() as i32)
    }

    #[inline(always)]
    fn upper_bound(&self) -> Vec3<i32> {
        self.inner
            .upper_bound()
            .map2(self.scale, |e, scale| (e as f32 * scale).ceil() as i32 + 1)
    }
}
