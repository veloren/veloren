use crate::vol::{BaseVol, ReadVol, SizedVol, VolSize, Vox, WriteVol};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;
use vek::*;

#[derive(Debug)]
pub enum ChunkErr {
    OutOfBounds,
}

/// A volume with dimensions known at compile-time.
// V = Voxel
// S = Size (replace when const generics are a thing)
// M = Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk<V: Vox, S: VolSize, M> {
    vox: Vec<V>,
    meta: M,
    phantom: PhantomData<S>,
}

impl<V: Vox, S: VolSize, M> Chunk<V, S, M> {
    /// Used to transform a voxel position in the volume into its corresponding index
    /// in the voxel array.
    #[inline(always)]
    fn idx_for(pos: Vec3<i32>) -> Option<usize> {
        if pos.map(|e| e >= 0).reduce_and()
            && pos.map2(S::SIZE, |e, lim| e < lim as i32).reduce_and()
        {
            Some(Self::idx_for_unchecked(pos))
        } else {
            None
        }
    }

    /// Used to transform a voxel position in the volume into its corresponding index
    /// in the voxel array.
    #[inline(always)]
    fn idx_for_unchecked(pos: Vec3<i32>) -> usize {
        (pos.x * S::SIZE.y as i32 * S::SIZE.z as i32 + pos.y * S::SIZE.z as i32 + pos.z) as usize
    }
}

impl<V: Vox, S: VolSize, M> BaseVol for Chunk<V, S, M> {
    type Vox = V;
    type Err = ChunkErr;
}

impl<V: Vox, S: VolSize, M> SizedVol for Chunk<V, S, M> {
    #[inline(always)]
    fn get_size(&self) -> Vec3<u32> {
        S::SIZE
    }
}

impl<V: Vox, S: VolSize, M> ReadVol for Chunk<V, S, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V, ChunkErr> {
        Self::idx_for(pos)
            .and_then(|idx| self.vox.get(idx))
            .ok_or(ChunkErr::OutOfBounds)
    }
}

impl<V: Vox, S: VolSize, M> WriteVol for Chunk<V, S, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), ChunkErr> {
        Self::idx_for(pos)
            .and_then(|idx| self.vox.get_mut(idx))
            .map(|old_vox| *old_vox = vox)
            .ok_or(ChunkErr::OutOfBounds)
    }
}

impl<V: Vox + Clone, S: VolSize, M> Chunk<V, S, M> {
    /// Create a new `Chunk` with the provided dimensions and all voxels filled with duplicates of
    /// the provided voxel.
    pub fn filled(vox: V, meta: M) -> Self {
        Self {
            vox: vec![vox; S::SIZE.product() as usize],
            meta,
            phantom: PhantomData,
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
