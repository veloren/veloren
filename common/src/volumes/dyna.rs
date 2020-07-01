use crate::vol::{
    BaseVol, DefaultPosIterator, DefaultVolIterator, IntoPosIterator, IntoVolIterator, ReadVol,
    SizedVol, Vox, WriteVol,
};
use serde_derive::{Deserialize, Serialize};
use vek::*;

#[derive(Debug, Clone)]
pub enum DynaError {
    OutOfBounds,
}

/// A volume with dimensions known only at the creation of the object.
// V = Voxel
// S = Size (replace when const generics are a thing)
// M = Metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct Dyna<V: Vox, M, A: Access = ColumnAccess> {
    vox: Vec<V>,
    meta: M,
    pub sz: Vec3<u32>,
    _phantom: std::marker::PhantomData<A>,
}

impl<V: Vox, M: Clone, A: Access> Clone for Dyna<V, M, A> {
    fn clone(&self) -> Self {
        Self {
            vox: self.vox.clone(),
            meta: self.meta.clone(),
            sz: self.sz,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<V: Vox, M, A: Access> Dyna<V, M, A> {
    /// Used to transform a voxel position in the volume into its corresponding
    /// index in the voxel array.
    #[inline(always)]
    fn idx_for(sz: Vec3<u32>, pos: Vec3<i32>) -> Option<usize> {
        if pos.map(|e| e >= 0).reduce_and() && pos.map2(sz, |e, lim| e < lim as i32).reduce_and() {
            Some(A::idx(pos, sz))
        } else {
            None
        }
    }
}

impl<V: Vox, M, A: Access> BaseVol for Dyna<V, M, A> {
    type Error = DynaError;
    type Vox = V;
}

impl<V: Vox, M, A: Access> SizedVol for Dyna<V, M, A> {
    #[inline(always)]
    fn lower_bound(&self) -> Vec3<i32> { Vec3::zero() }

    #[inline(always)]
    fn upper_bound(&self) -> Vec3<i32> { self.sz.map(|e| e as i32) }
}

impl<V: Vox, M, A: Access> ReadVol for Dyna<V, M, A> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V, DynaError> {
        Self::idx_for(self.sz, pos)
            .and_then(|idx| self.vox.get(idx))
            .ok_or(DynaError::OutOfBounds)
    }
}

impl<V: Vox, M, A: Access> WriteVol for Dyna<V, M, A> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), DynaError> {
        Self::idx_for(self.sz, pos)
            .and_then(|idx| self.vox.get_mut(idx))
            .map(|old_vox| *old_vox = vox)
            .ok_or(DynaError::OutOfBounds)
    }
}

impl<'a, V: Vox, M, A: Access> IntoPosIterator for &'a Dyna<V, M, A> {
    type IntoIter = DefaultPosIterator;

    fn pos_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter::new(lower_bound, upper_bound)
    }
}

impl<'a, V: Vox, M, A: Access> IntoVolIterator<'a> for &'a Dyna<V, M, A> {
    type IntoIter = DefaultVolIterator<'a, Dyna<V, M, A>>;

    fn vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter::new(self, lower_bound, upper_bound)
    }
}

impl<V: Vox + Clone, M, A: Access> Dyna<V, M, A> {
    /// Create a new `Dyna` with the provided dimensions and all voxels filled
    /// with duplicates of the provided voxel.
    pub fn filled(sz: Vec3<u32>, vox: V, meta: M) -> Self {
        Self {
            vox: vec![vox; sz.product() as usize],
            meta,
            sz,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Get a reference to the internal metadata.
    pub fn metadata(&self) -> &M { &self.meta }

    /// Get a mutable reference to the internal metadata.
    pub fn metadata_mut(&mut self) -> &mut M { &mut self.meta }
}

pub trait Access {
    fn idx(pos: Vec3<i32>, sz: Vec3<u32>) -> usize;
}

pub struct ColumnAccess;

impl Access for ColumnAccess {
    fn idx(pos: Vec3<i32>, sz: Vec3<u32>) -> usize {
        (pos.x * sz.y as i32 * sz.z as i32 + pos.y * sz.z as i32 + pos.z) as usize
    }
}
