use crate::ray::Ray;
use std::fmt::Debug;
use vek::*;

/// A voxel.
pub trait Vox: Sized {
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

// Utility types

pub struct VoxPosIter {
    pos: Vec3<u32>,
    sz: Vec3<u32>,
}

impl Iterator for VoxPosIter {
    type Item = Vec3<i32>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut old_pos = self.pos;

        if old_pos.z == self.sz.z {
            old_pos.z = 0;
            old_pos.y += 1;
            if old_pos.y == self.sz.y {
                old_pos.y = 0;
                old_pos.x += 1;
                if old_pos.x == self.sz.x {
                    return None;
                }
            }
        }

        self.pos = old_pos + Vec3::unit_z();

        Some(old_pos.map(|e| e as i32))
    }
}

/// A volume that has a finite size.
pub trait SizedVol: BaseVol {
    /// Get the size of the volume.
    fn get_size(&self) -> Vec3<u32>;

    /// Iterate through all potential voxel positions in this volume.
    fn iter_positions(&self) -> VoxPosIter {
        VoxPosIter {
            pos: Vec3::zero(),
            sz: self.get_size(),
        }
    }
}

/// A volume that provides read access to its voxel data.
pub trait ReadVol: BaseVol {
    /// Get a reference to the voxel at the provided position in the volume.
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, Self::Err>;

    unsafe fn get_unchecked(&self, pos: Vec3<i32>) -> &Self::Vox {
        self.get(pos).unwrap()
    }

    fn ray(
        &self,
        from: Vec3<f32>,
        to: Vec3<f32>,
    ) -> Ray<Self, fn(&Self::Vox) -> bool, fn(Vec3<i32>)>
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

// Utility traits

/// Used to specify a volume's compile-time size. This exists as a substitute until const generics
/// are implemented.
pub trait VolSize {
    const SIZE: Vec3<u32>;
}
