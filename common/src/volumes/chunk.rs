// Standard
use std::marker::PhantomData;

// Library
use vek::*;

// Local
use crate::vol::{
    BaseVol,
    SizedVol,
    ReadVol,
    WriteVol,
    VolSize,
};

pub enum ChunkErr {
    OutOfBounds,
}

// V = Voxel
// S = Size (replace when const generics are a thing)
// M = Metadata
pub struct Chunk<V, S: VolSize, M> {
    vox: Vec<V>,
    meta: M,
    phantom: PhantomData<S>,
}

impl<V, S: VolSize, M> Chunk<V, S, M> {
    #[inline(always)]
    fn idx_for(pos: Vec3<i32>) -> Option<usize> {
        if
            pos.map(|e| e >= 0).reduce_and() &&
            pos.map2(S::SIZE, |e, lim| e < lim as i32).reduce_and()
        {
            Some((
                pos.x * S::SIZE.y as i32 * S::SIZE.z as i32 +
                pos.y * S::SIZE.z as i32 +
                pos.z
            ) as usize)
        } else {
            None
        }
    }
}

impl<V, S: VolSize, M> BaseVol for Chunk<V, S, M> {
    type Vox = V;
    type Err = ChunkErr;
}

impl<V, S: VolSize, M> SizedVol for Chunk<V, S, M> {
    const SIZE: Vec3<u32> = Vec3 { x: 32, y: 32, z: 32 };
}

impl<V, S: VolSize, M> ReadVol for Chunk<V, S, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V, ChunkErr> {
        Self::idx_for(pos)
            .and_then(|idx| self.vox.get(idx))
            .ok_or(ChunkErr::OutOfBounds)
    }
}

impl<V, S: VolSize, M> WriteVol for Chunk<V, S, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), ChunkErr> {
        Self::idx_for(pos)
            .and_then(|idx| self.vox.get_mut(idx))
            .map(|old_vox| *old_vox = vox)
            .ok_or(ChunkErr::OutOfBounds)
    }
}

impl<V: Clone, S: VolSize, M> Chunk<V, S, M> {
    pub fn filled(vox: V, meta: M) -> Self {
        Self {
            vox: vec![vox; S::SIZE.product() as usize],
            meta,
            phantom: PhantomData,
        }
    }

    pub fn metadata(&self) -> &M {
        &self.meta
    }

    pub fn metadata_mut(&mut self) -> &mut M {
        &mut self.meta
    }
}
