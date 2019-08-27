use crate::ray::Ray;
use std::fmt::Debug;
use vek::*;

/// A voxel.
pub trait Vox: Sized + Clone {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;

    fn or(self, other: Self) -> Self {
        if self.is_empty() {
            other
        } else {
            self
        }
    }
}

/// A volume that contains voxel data.
pub trait BaseVol {
    type Vox: Vox;
    type Err: Debug;
}

impl<'a, T: BaseVol> BaseVol for &'a T {
    type Vox = T::Vox;
    type Err = T::Err;
}

// Utility types

/// A volume that has a finite size.
pub trait SizedVol: BaseVol {
    /// Returns the (exclusive) upper bound of the volume.
    fn lower_bound(&self) -> Vec3<i32>;

    /// Returns the (inclusive) lower bound of the volume.
    fn upper_bound(&self) -> Vec3<i32>;

    /// Returns the size of the volume.
    fn get_size(&self) -> Vec3<u32> {
        (self.upper_bound() - self.lower_bound()).map(|e| e as u32)
    }
}

/// A volume that provides read access to its voxel data.
pub trait ReadVol: BaseVol {
    /// Get a reference to the voxel at the provided position in the volume.
    fn get<'a>(&'a self, pos: Vec3<i32>) -> Result<&'a Self::Vox, Self::Err>;

    fn ray<'a>(
        &'a self,
        from: Vec3<f32>,
        to: Vec3<f32>,
    ) -> Ray<'a, Self, fn(&Self::Vox) -> bool, fn(Vec3<i32>)>
    where
        Self: Sized,
    {
        Ray::new(self, from, to, |vox| !vox.is_empty())
    }
}

/// A volume that provides the ability to sample (i.e., clone a section of) its voxel data.
pub trait SampleVol<I>: BaseVol {
    type Sample: BaseVol + ReadVol;
    /// Take a sample of the volume by cloning voxels within the provided range.
    ///
    /// Note that value and accessibility of voxels outside the bounds of the sample is
    /// implementation-defined and should not be used.
    ///
    /// Note that the resultant volume has a coordinate space relative to the sample, not the
    /// original volume.
    fn sample(&self, range: I) -> Result<Self::Sample, Self::Err>;
}

/// A volume that provides write access to its voxel data.
pub trait WriteVol: BaseVol {
    /// Set the voxel at the provided position in the volume to the provided value.
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), Self::Err>;
}

/// A volume that shall be iterable.
pub trait IntoVolIterator<'a>: BaseVol
where
    Self::Vox: 'a,
{
    type IntoIter: Iterator<Item = (Vec3<i32>, &'a Self::Vox)>;

    fn into_vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter;
}

pub trait IntoFullVolIterator<'a>: BaseVol
where
    Self::Vox: 'a,
{
    type IntoIter: Iterator<Item = (Vec3<i32>, &'a Self::Vox)>;

    fn into_iter(self) -> Self::IntoIter;
}

impl<'a, T: 'a + SizedVol> IntoFullVolIterator<'a> for &'a T
where
    Self: IntoVolIterator<'a>,
{
    type IntoIter = <Self as IntoVolIterator<'a>>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vol_iter(self.lower_bound(), self.upper_bound())
    }
}

// Defaults

/// Iterator type for the default implementation of `IterateVol`
pub struct DefaultVolIterator<'a, T: ReadVol> {
    vol: &'a T,
    current: Vec3<i32>,
    begin: Vec2<i32>,
    end: Vec3<i32>,
}

impl<'a, T: ReadVol> DefaultVolIterator<'a, T> {
    pub fn new(vol: &'a T, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self {
        Self {
            vol,
            current: lower_bound,
            begin: From::from(lower_bound),
            end: upper_bound,
        }
    }
}

impl<'a, T: ReadVol> Iterator for DefaultVolIterator<'a, T> {
    type Item = (Vec3<i32>, &'a T::Vox);

    fn next(&mut self) -> Option<(Vec3<i32>, &'a T::Vox)> {
        loop {
            self.current.x += (self.current.x < self.end.x) as i32;
            if self.current.x == self.end.x {
                self.current.x = self.begin.x;
                self.current.y += (self.current.y < self.end.y) as i32;
                if self.current.y == self.end.y {
                    self.current.y = self.begin.y;
                    self.current.z += (self.current.z < self.end.z) as i32;
                    if self.current.z == self.end.z {
                        return None;
                    }
                }
            }
            if let Ok(vox) = self.vol.get(self.current) {
                return Some((self.current, vox));
            }
        }
    }
}

// WIP

/// Used to specify a volume's compile-time size. This exists as a substitute until const generics
/// are implemented.
pub trait VolSize: Clone {
    const SIZE: Vec3<u32>;
}
