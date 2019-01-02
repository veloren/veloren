// Library
use vek::*;

pub trait BaseVol {
    type Vox;
    type Err;
}

pub trait SizedVol: BaseVol {
    const SIZE: Vec3<u32>;
}

pub trait ReadVol: BaseVol {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, Self::Err>;
}

pub trait WriteVol: BaseVol {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), Self::Err>;
}

// Utility traits

pub trait VolSize {
    const SIZE: Vec3<u32>;
}
