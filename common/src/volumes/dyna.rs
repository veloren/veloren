use crate::vol::{BaseVol, DefaultVolIterator, IntoVolIterator, ReadVol, SizedVol, Vox, WriteVol};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;
use vek::*;

#[derive(Debug, Clone)]
pub enum DynaError {
    OutOfBounds,
}

#[derive(Clone)]
pub struct DynaPos {
    idx: u32,
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
    fn to_pos_unchecked(&self, pos: Vec3<i32>) -> DynaPos {
        DynaPos {
            idx: (pos.x * self.sz.y as i32 * self.sz.z as i32 + pos.y * self.sz.z as i32 + pos.z)
                as u32,
        }
    }
}

impl<V: Vox, M> BaseVol for Dyna<V, M> {
    type Vox = V;
    type Error = DynaError;
    type Pos = DynaPos;

    fn to_pos(&self, pos: Vec3<i32>) -> Result<Self::Pos, Self::Error> {
        if pos.map(|e| e >= 0).reduce_and()
            && pos.map2(self.sz, |e, lim| e < lim as i32).reduce_and()
        {
            Ok(self.to_pos_unchecked(pos))
        } else {
            Err(Self::Error::OutOfBounds)
        }
    }

    fn to_vec3(&self, pos: Self::Pos) -> Vec3<i32> {
        let mut idx = pos.idx;
        let z = idx % self.sz.z;
        idx /= self.sz.z;
        let y = idx % self.sz.y;
        idx /= self.sz.y;
        let x = idx;
        Vec3::new(x, y, z).map(|e| e as i32)
    }
}

impl<V: Vox, M> SizedVol for Dyna<V, M> {
    #[inline(always)]
    fn lower_bound(&self) -> Vec3<i32> {
        Vec3::zero()
    }

    #[inline(always)]
    fn upper_bound(&self) -> Vec3<i32> {
        self.sz.map(|e| e as i32)
    }
}

impl<V: Vox, M> ReadVol for Dyna<V, M> {
    #[inline(always)]
    fn get_pos(&self, pos: Self::Pos) -> &V {
        &self.vox[pos.idx as usize]
    }
}

impl<V: Vox, M> WriteVol for Dyna<V, M> {
    #[inline(always)]
    fn set_pos(&mut self, pos: Self::Pos, vox: Self::Vox) {
        self.vox[pos.idx as usize] = vox;
    }
}

impl<'a, V: Vox, M> IntoVolIterator<'a> for &'a Dyna<V, M> {
    type IntoIter = DefaultVolIterator<'a, Dyna<V, M>>;

    fn vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter::new(self, lower_bound, upper_bound)
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
