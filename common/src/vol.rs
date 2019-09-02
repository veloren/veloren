use crate::ray::Ray;
use std::fmt::Debug;
use vek::*;

/// Used to specify a volume's compile-time size. This exists as a substitute until const generics
/// are implemented.
pub trait VolSize: Clone {
    const SIZE: Vec3<u32>;
}

pub trait RectVolSize: Clone {
    const RECT_SIZE: Vec2<u32>;
}

/// A voxel.
pub trait Vox: Sized + Clone + PartialEq {
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
    type Error: Debug;
    type Pos: Clone;

    fn to_pos(&self, pos: Vec3<i32>) -> Result<Self::Pos, Self::Error>;
    fn to_vec3(&self, pos: Self::Pos) -> Vec3<i32>;
}

/// Implementing `BaseVol` for any `&'a BaseVol` makes it possible to implement
/// `IntoVolIterator` for references.
impl<'a, T: BaseVol> BaseVol for &'a T {
    type Vox = T::Vox;
    type Error = T::Error;
    type Pos = T::Pos;

    fn to_pos(&self, pos: Vec3<i32>) -> Result<Self::Pos, Self::Error> {
        <T as BaseVol>::to_pos(self, pos)
    }

    fn to_vec3(&self, pos: Self::Pos) -> Vec3<i32> {
        <T as BaseVol>::to_vec3(self, pos)
    }
}

// Utility types

/// A volume that is a cuboid.
pub trait SizedVol: BaseVol {
    /// Returns the (inclusive) lower bound of the volume.
    fn lower_bound(&self) -> Vec3<i32>;

    /// Returns the (exclusive) upper bound of the volume.
    fn upper_bound(&self) -> Vec3<i32>;

    /// Returns the size of the volume.
    fn size(&self) -> Vec3<u32> {
        (self.upper_bound() - self.lower_bound()).map(|e| e as u32)
    }
}

/// A volume that is compile-time sized and has its lower bound at `(0, 0, 0)`.
/// The name `RasterableVol` was chosen because such a volume can be used with
/// `VolGrid3d`.
pub trait RasterableVol: BaseVol {
    const SIZE: Vec3<u32>;
}

impl<V: RasterableVol> SizedVol for V {
    fn lower_bound(&self) -> Vec3<i32> {
        Vec3::zero()
    }

    fn upper_bound(&self) -> Vec3<i32> {
        V::SIZE.map(|e| e as i32)
    }
}

/// A volume whose cross section with the XY-plane is a rectangle.
pub trait RectSizedVol: BaseVol {
    fn lower_bound_xy(&self) -> Vec2<i32>;

    fn upper_bound_xy(&self) -> Vec2<i32>;

    fn get_size_xy(&self) -> Vec2<u32> {
        (self.upper_bound_xy() - self.lower_bound_xy()).map(|e| e as u32)
    }
}

/// A volume that is compile-time sized in x and y direction and has its lower
/// bound at `(0, 0, z)`. In z direction there's no restriction on the lower
/// or upper bound. The name `RectRasterableVol` was chosen because such a
/// volume can be used with `VolGrid2d`.
pub trait RectRasterableVol: BaseVol {
    const RECT_SIZE: Vec2<u32>;
}

impl<V: RectRasterableVol> RectSizedVol for V {
    fn lower_bound_xy(&self) -> Vec2<i32> {
        Vec2::zero()
    }

    fn upper_bound_xy(&self) -> Vec2<i32> {
        V::RECT_SIZE.map(|e| e as i32)
    }
}

/// A volume that provides read access to its voxel data.
pub trait ReadVol: BaseVol {
    /// Get a reference to the voxel at the provided position in the volume.
    fn get_pos<'a>(&'a self, pos: Self::Pos) -> &'a Self::Vox;

    fn get<'a>(&'a self, pos: Vec3<i32>) -> Result<&'a Self::Vox, Self::Error> {
        self.to_pos(pos).map(|pos| self.get_pos(pos))
    }

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
///
/// TODO (haslersn): Do we still need this now that we have `IntoVolIterator`?
pub trait SampleVol<I>: BaseVol {
    type Sample: BaseVol + ReadVol;
    /// Take a sample of the volume by cloning voxels within the provided range.
    ///
    /// Note that value and accessibility of voxels outside the bounds of the sample is
    /// implementation-defined and should not be used.
    ///
    /// Note that the resultant volume has a coordinate space relative to the sample, not the
    /// original volume.
    fn sample(&self, range: I) -> Result<Self::Sample, Self::Error>;
}

/// A volume that provides write access to its voxel data.
pub trait WriteVol: BaseVol {
    /// Write the voxel at the provided position in the volume.
    fn set_pos(&mut self, pos: Self::Pos, vox: Self::Vox);

    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), Self::Error> {
        self.to_pos(pos).map(|pos| self.set_pos(pos, vox))
    }
}

/// A volume (usually rather a reference to a volume) that is convertible into
/// an iterator to a cuboid subsection of the volume.
pub trait IntoVolIterator<'a>: BaseVol
where
    Self::Vox: 'a,
{
    type IntoIter: Iterator<Item = (Self::Pos, &'a Self::Vox)>;

    fn vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter;
}

pub trait IntoPosIterator: BaseVol {
    type IntoIter: Iterator<Item = Self::Pos>;

    fn pos_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter;
}

// Helpers

/// A volume (usually rather a reference to a volume) that is convertible into
/// an iterator.
pub trait IntoFullVolIterator<'a>: BaseVol
where
    Self::Vox: 'a,
{
    type IntoIter: Iterator<Item = (Self::Pos, &'a Self::Vox)>;

    fn full_vol_iter(self) -> Self::IntoIter;
}

/// For any `&'a SizedVol: IntoVolIterator` we implement `IntoFullVolIterator`.
/// Unfortunately we can't just implement `IntoIterator` in this generic way
/// because it's defined in another crate. That's actually the only reason why
/// the trait `IntoFullVolIterator` exists.
impl<'a, T: 'a + SizedVol> IntoFullVolIterator<'a> for &'a T
where
    Self: IntoVolIterator<'a>,
{
    type IntoIter = <Self as IntoVolIterator<'a>>::IntoIter;

    fn full_vol_iter(self) -> Self::IntoIter {
        self.vol_iter(self.lower_bound(), self.upper_bound())
    }
}

pub trait IntoFullPosIterator: BaseVol {
    type IntoIter: Iterator<Item = Self::Pos>;

    fn full_pos_iter(self) -> Self::IntoIter;
}

impl<'a, T: 'a + SizedVol> IntoFullPosIterator for &'a T
where
    Self: IntoPosIterator,
{
    type IntoIter = <Self as IntoPosIterator>::IntoIter;

    fn full_pos_iter(self) -> Self::IntoIter {
        self.pos_iter(self.lower_bound(), self.upper_bound())
    }
}

// Defaults

/// Convenience iterator type that can be used to quickly implement
/// `IntoVolIterator`.
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
    type Item = (T::Pos, &'a T::Vox);

    fn next(&mut self) -> Option<(T::Pos, &'a T::Vox)> {
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
            if let Ok(pos) = self.vol.to_pos(self.current) {
                return Some((pos.clone(), self.vol.get_pos(pos)));
            }
        }
    }
}
