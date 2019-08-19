use crate::vol::{BaseVol, ReadVol, SizedVol, Vox, WriteVol};
use serde_derive::{Deserialize, Serialize};
use vek::*;

#[derive(Debug, Clone)]
pub enum DynaErr {
    OutOfBounds,
}

/// A volume with dimensions known only at the creation of the object.
// V = Voxel
// S = Size (replace when const generics are a thing)
// M = Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dyna<V: Vox, M> {
    vox: Vec<V>,
    meta: M,
    sz: Vec3<u32>,
}

impl<V: Vox, M> Dyna<V, M> {
    /// Used to transform a voxel position in the volume into its corresponding index
    /// in the voxel array.
    #[inline(always)]
    fn idx_for(sz: Vec3<u32>, pos: Vec3<i32>) -> Option<usize> {
        if pos.map(|e| e >= 0).reduce_and() && pos.map2(sz, |e, lim| e < lim as i32).reduce_and() {
            Some(Self::idx_for_unchecked(sz, pos))
        } else {
            None
        }
    }

    /// Used to transform a voxel position in the volume into its corresponding index
    /// in the voxel array.
    #[inline(always)]
    fn idx_for_unchecked(sz: Vec3<u32>, pos: Vec3<i32>) -> usize {
        (pos.x * sz.y as i32 * sz.z as i32 + pos.y * sz.z as i32 + pos.z) as usize
    }
}

impl<V: Vox, M> BaseVol for Dyna<V, M> {
    type Vox = V;
    type Err = DynaErr;
}

impl<V: Vox, M> SizedVol for Dyna<V, M> {
    #[inline(always)]
    fn get_size(&self) -> Vec3<u32> {
        self.sz
    }
}

impl<V: Vox, M> ReadVol for Dyna<V, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V, DynaErr> {
        Self::idx_for(self.sz, pos)
            .and_then(|idx| self.vox.get(idx))
            .ok_or(DynaErr::OutOfBounds)
    }
}

impl<V: Vox, M> WriteVol for Dyna<V, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), DynaErr> {
        Self::idx_for(self.sz, pos)
            .and_then(|idx| self.vox.get_mut(idx))
            .map(|old_vox| *old_vox = vox)
            .ok_or(DynaErr::OutOfBounds)
    }
}

impl<V: Vox + Clone, M> Dyna<V, M> {
    /// Create a new `Dyna` with the provided dimensions and all voxels filled with duplicates of
    /// the provided voxel.
    pub fn filled(sz: Vec3<u32>, vox: V, meta: M) -> Self {
        Self {
            vox: vec![vox; sz.product() as usize],
            meta,
            sz,
        }
    }

    /// Get a reference to the internal metadata.
    pub fn metadata(&self) -> &M {
        &self.meta
    }

    /// Get a mutable reference to the internal metadata.
    pub fn metadata_mut(&mut self) -> &mut M {
        &mut self.meta
    }
}
