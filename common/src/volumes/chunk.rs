use crate::{
    vol::{
        BaseVol, IntoPosIterator, IntoVolIterator, RasterableVol, ReadVol, SizedVol, VolSize, Vox,
        WriteVol,
    },
    volumes::morton::{morton_to_xyz, xyz_to_morton, MortonIter},
};
use serde_derive::{Deserialize, Serialize};
use std::iter::Iterator;
use std::marker::PhantomData;
use vek::*;

#[derive(Debug)]
pub enum ChunkError {
    OutOfBounds,
}

#[derive(Clone)]
pub struct ChunkPos<S: VolSize> {
    morton: u32,
    phantom: PhantomData<S>,
}

/// A `Chunk` is a volume with dimensions known at compile-time. The voxels are
/// ordered by a morton curve (see https://en.wikipedia.org/wiki/Z-order_curve ).
/// In sparsely populated chunks (i.e. where most of the voxels are some default
/// voxel like air) we of course don't want to store the default voxels.
/// Therefore, we also use an index buffer.
///
/// * V = Voxel
/// * S = Size (replace when const generics are a thing)
/// * M = Metadata
///
/// The voxels of a `Chunk` are conceptually ordered by morton order and then
/// subdivided into 256 groups along the order. The constant `BLOCK_GROUP_SIZE`
/// contains the number of voxels per group. The field `vox : Vec<V>` contains
/// concatenated groups, i.e. its `len()` is invariantly divisable by
/// `BLOCK_GROUP_SIZE` and every (aligned) sequence of `BLOCK_GROUP_SIZE` voxels
/// in `self.vox` is a group.
///
/// Furthermore there's an index buffer `indices : [u8; 256]` which contains for
/// each group
///
/// * (a) the order in which it has been inserted into `Chunk::vox`,
///   if the group is contained in `Chunk::vox` or
/// * (b) 255, otherwise. That case represents that the whole group consists
///   only of `self.default` voxels.
///
/// (Note that 255 is a valid insertion order for case (a) only if `self.vox` is
/// full and then no other group has the index 255. Therefore there's no
/// ambiguity.)
///
/// Concerning morton order:
///
/// * (1) The index buffer `Chunk::indices` unconditionally exists for every
///   chunk and is sorted by morton order.
/// * (2) The groups in `Chunk::vox` are not sorted by morton order, but rather
///   by their insertion order in order to prevent insertions in the middle of
///   a big vector.
/// * (3) The voxels inside a group in `Chunk::vox` are sorted by
///   morton order.
///
/// Rationale:
///
/// We hope that sorting indices and voxels by morton order provides cache
/// friendliness for local access patterns. Note that, as mentioned in (2),
/// `self.vox` is not fully sorted by morton order. Only individual groups
/// therein are. This is because otherwise most insertions would be expensive.
/// (As it is, still insertions that trigger a reallocation of `Chunk::vox` are
/// expensive.) As a future optimization, we could possibly provide an
/// `self.optimize()` method to sort the groups by morton order (and update the
/// index buffer accordingly). One could then clone a `Chunk`, run mentioned
/// method in a background thread and afterwards, if the original `Chunk` wasn't
/// altered in the meantime, replace it by its optimized version.
///
/// The number of groups is 256 such that the index buffer can consist of `u8`s.
/// This keeps the space requirement for the index buffer low and hence an empty
/// or almost empty `Chunk` doesn't consume too much memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk<V: Vox, S: VolSize, M> {
    indices: Vec<u8>, // TODO (haslersn): Box<[u8; S::SIZE.x * S::SIZE.y * S::SIZE.z]>, this is however not possible in Rust yet
    vox: Vec<V>,
    default: V,
    meta: M,
    phantom: PhantomData<S>,
}

impl<V: Vox, S: VolSize, M> Chunk<V, S, M> {
    const BLOCK_COUNT: usize = (S::SIZE.x * S::SIZE.y * S::SIZE.z) as usize;
    const BLOCK_GROUP_SIZE: usize =
        [Self::BLOCK_COUNT / 256, 1][(Self::BLOCK_COUNT < 256) as usize];
    /// `BLOCK_GROUP_COUNT` is always `256`, except if `BLOCK_COUNT < 256`
    const BLOCK_GROUP_COUNT: usize = Self::BLOCK_COUNT / Self::BLOCK_GROUP_SIZE;

    /// Creates a new `Chunk` with the provided dimensions and all voxels filled
    /// with duplicates of the provided voxel.
    pub fn filled(default: V, meta: M) -> Self {
        // TODO (haslersn): Alter into compile time assertions
        //
        // An extent is valid if it fulfils the following conditions.
        //
        // 1. In each direction, the extent is a power of two.
        // 2. In `x` and `y` direction the extent is in [1, 2048]. In `z`
        //    direction the extent is in [1, 1024].
        // 3. `x = y = z` or `x = 2y = 2z` or `x = y = 2z`
        //
        // Rationales:
        //
        // 1. The morton curve needs powers of two.
        // 2. The `stretch_unchecked()` implementation only works for up to 11
        //    bits.
        // 3. The morton curve extends in the order `x, y, z, x, y, z, ...`
        debug_assert!(0 < S::SIZE.x);
        debug_assert!(0 < S::SIZE.y);
        debug_assert!(0 < S::SIZE.z);
        debug_assert!(S::SIZE.x <= 64);
        debug_assert!(S::SIZE.y <= 64);
        debug_assert!(S::SIZE.z <= 64);
        debug_assert!(S::SIZE.x & (S::SIZE.x - 1) == 0);
        debug_assert!(S::SIZE.y & (S::SIZE.y - 1) == 0);
        debug_assert!(S::SIZE.z & (S::SIZE.z - 1) == 0);
        debug_assert!(S::SIZE.x >= S::SIZE.y);
        debug_assert!(S::SIZE.y >= S::SIZE.z);
        debug_assert!(2 * S::SIZE.z >= S::SIZE.x);

        Self {
            indices: vec![255; Self::BLOCK_GROUP_COUNT],
            vox: Vec::new(),
            default,
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

    fn idx_from_morton_unchecked(&self, morton: u32) -> Option<usize> {
        let base = self.indices[morton as usize / Self::BLOCK_GROUP_SIZE];
        let num_groups = self.vox.len() / Self::BLOCK_GROUP_SIZE;
        if base as usize >= num_groups {
            None
        } else {
            Some(
                base as usize * Self::BLOCK_GROUP_SIZE + (morton as usize % Self::BLOCK_GROUP_SIZE),
            )
        }
    }

    fn force_idx_from_morton_unchecked(&mut self, morton: u32) -> usize {
        let base = &mut self.indices[morton as usize / Self::BLOCK_GROUP_SIZE];
        let num_groups = self.vox.len() / Self::BLOCK_GROUP_SIZE;
        if *base as usize >= num_groups {
            *base = num_groups as u8;
            self.vox
                .extend(std::iter::repeat(self.default.clone()).take(Self::BLOCK_GROUP_SIZE));
        }
        *base as usize * Self::BLOCK_GROUP_SIZE + (morton as usize % Self::BLOCK_GROUP_SIZE)
    }

    fn get_from_morton_unchecked(&self, morton: u32) -> &V {
        match self.idx_from_morton_unchecked(morton) {
            Some(idx) => &self.vox[idx],
            None => &self.default,
        }
    }

    fn get_from_morton(&self, morton: u32) -> Result<&V, ChunkError> {
        if morton as usize >= Self::BLOCK_COUNT {
            Err(ChunkError::OutOfBounds)
        } else {
            Ok(self.get_from_morton_unchecked(morton))
        }
    }

    fn set_from_morton_unchecked(&mut self, morton: u32, vox: V) {
        if vox != self.default {
            let idx = self.force_idx_from_morton_unchecked(morton);
            self.vox[idx] = vox;
        } else if let Some(idx) = self.idx_from_morton_unchecked(morton) {
            self.vox[idx] = vox;
        }
    }

    fn set_from_morton(&mut self, morton: u32, vox: V) -> Result<(), ChunkError> {
        if morton as usize >= Self::BLOCK_COUNT {
            Err(ChunkError::OutOfBounds)
        } else {
            Ok(self.set_from_morton_unchecked(morton, vox))
        }
    }

    pub fn to_pos(pos: Vec3<i32>) -> Result<ChunkPos<S>, ChunkError> {
        if !pos
            .map2(S::SIZE, |e, s| 0 <= e && e < s as i32)
            .reduce_and()
        {
            Err(ChunkError::OutOfBounds)
        } else {
            Ok(ChunkPos::<S> {
                morton: xyz_to_morton(pos),
                phantom: PhantomData,
            })
        }
    }

    pub fn to_vec3(pos: ChunkPos<S>) -> Vec3<i32> {
        morton_to_xyz(pos.morton)
    }
}

impl<V: Vox, S: VolSize, M> BaseVol for Chunk<V, S, M> {
    type Vox = V;
    type Error = ChunkError;
    type Pos = ChunkPos<S>;

    fn to_pos(&self, pos: Vec3<i32>) -> Result<Self::Pos, Self::Error> {
        Self::to_pos(pos)
    }

    fn to_vec3(&self, pos: Self::Pos) -> Vec3<i32> {
        Self::to_vec3(pos)
    }
}

impl<V: Vox, S: VolSize, M> RasterableVol for Chunk<V, S, M> {
    const SIZE: Vec3<u32> = S::SIZE;
}

impl<V: Vox, S: VolSize, M> ReadVol for Chunk<V, S, M> {
    #[inline(always)]
    fn get_pos(&self, pos: Self::Pos) -> &Self::Vox {
        self.get_from_morton_unchecked(pos.morton)
    }
}

impl<V: Vox, S: VolSize, M> WriteVol for Chunk<V, S, M> {
    #[inline(always)]
    fn set_pos(&mut self, pos: Self::Pos, vox: Self::Vox) {
        self.set_from_morton_unchecked(pos.morton, vox);
    }
}

pub struct ChunkPosIter<S: VolSize> {
    morton_iter: MortonIter,
    phantom: PhantomData<S>,
}

impl<S: VolSize> ChunkPosIter<S> {
    pub fn new(lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self {
        let lb = Vec3::partial_max(lower_bound, Vec3::zero());
        let ub = Vec3::partial_min(upper_bound, S::SIZE.map(|e| e as i32));
        Self {
            morton_iter: MortonIter::new(lb, ub),
            phantom: PhantomData,
        }
    }
}

impl<S: VolSize> Iterator for ChunkPosIter<S> {
    type Item = ChunkPos<S>;

    fn next(&mut self) -> Option<Self::Item> {
        self.morton_iter.next().map(|morton| ChunkPos::<S> {
            morton,
            phantom: PhantomData,
        })
    }
}

pub struct ChunkVolIter<'a, V: 'a + Vox, S: VolSize, M> {
    chunk: &'a Chunk<V, S, M>,
    iter: ChunkPosIter<S>,
}

impl<'a, V: 'a + Vox, S: VolSize, M> Iterator for ChunkVolIter<'a, V, S, M> {
    type Item = (ChunkPos<S>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|pos| (pos.clone(), self.chunk.get_pos(pos)))
    }
}

impl<'a, V: Vox, S: VolSize, M> IntoPosIterator for &'a Chunk<V, S, M> {
    type IntoIter = ChunkPosIter<S>;

    fn pos_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter::new(lower_bound, upper_bound)
    }
}

impl<'a, V: Vox, S: VolSize, M> IntoVolIterator<'a> for &'a Chunk<V, S, M> {
    type IntoIter = ChunkVolIter<'a, V, S, M>;

    fn vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter {
            chunk: self,
            iter: self.pos_iter(lower_bound, upper_bound),
        }
    }
}
