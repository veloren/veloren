use crate::vol::{
    BaseVol, IntoPosIterator, IntoVolIterator, RasterableVol, ReadVol, VolSize, Vox, WriteVol,
};
use serde::{Deserialize, Serialize};
use std::{iter::Iterator, marker::PhantomData};
use vek::*;

#[derive(Debug)]
pub enum ChunkError {
    OutOfBounds,
}

/// The volume is spatially subdivided into groups of `4*4*4` blocks. Since a
/// `Chunk` is of total size `32*32*16`, this implies that there are `8*8*4`
/// groups. (These numbers are generic in the actual code such that there are
/// always `256` groups. I.e. the group size is chosen depending on the desired
/// total size of the `Chunk`.)
///
/// There's a single vector `self.vox` which consecutively stores these groups.
/// Each group might or might not be contained in `self.vox`. A group that is
/// not contained represents that the full group consists only of `self.default`
/// voxels. This saves a lot of memory because oftentimes a `Chunk` consists of
/// either a lot of air or a lot of stone.
///
/// To track whether a group is contained in `self.vox`, there's an index buffer
/// `self.indices : [u8; 256]`. It contains for each group
///
/// * (a) the order in which it has been inserted into `self.vox`, if the group
///   is contained in `self.vox` or
/// * (b) 255, otherwise. That case represents that the whole group consists
///   only of `self.default` voxels.
///
/// (Note that 255 is a valid insertion order for case (a) only if `self.vox` is
/// full and then no other group has the index 255. Therefore there's no
/// ambiguity.)
///
/// ## Rationale:
///
/// The index buffer should be small because:
///
/// * Small size increases the probability that it will always be in cache.
/// * The index buffer is allocated for every `Chunk` and an almost empty
///   `Chunk` shall not consume too much memory.
///
/// The number of 256 groups is particularly nice because it means that the
/// index buffer can consist of `u8`s. This keeps the space requirement for the
/// index buffer as low as 4 cache lines.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk<V: Vox, S: VolSize, M> {
    indices: Vec<u8>, /* TODO (haslersn): Box<[u8; S::SIZE.x * S::SIZE.y * S::SIZE.z]>, this is
                       * however not possible in Rust yet */
    vox: Vec<V>,
    default: V,
    meta: M,
    phantom: PhantomData<S>,
}

impl<V: Vox, S: VolSize, M> Chunk<V, S, M> {
    const GROUP_COUNT: Vec3<u32> = Vec3::new(
        S::SIZE.x / Self::GROUP_SIZE.x,
        S::SIZE.y / Self::GROUP_SIZE.y,
        S::SIZE.z / Self::GROUP_SIZE.z,
    );
    /// `GROUP_COUNT_TOTAL` is always `256`, except if `VOLUME < 256`
    const GROUP_COUNT_TOTAL: u32 = Self::VOLUME / Self::GROUP_VOLUME;
    const GROUP_LONG_SIDE_LEN: u32 = 1 << ((Self::GROUP_VOLUME * 4 - 1).count_ones() / 3);
    const GROUP_SIZE: Vec3<u32> = Vec3::new(
        Self::GROUP_LONG_SIDE_LEN,
        Self::GROUP_LONG_SIDE_LEN,
        Self::GROUP_VOLUME / (Self::GROUP_LONG_SIDE_LEN * Self::GROUP_LONG_SIDE_LEN),
    );
    const GROUP_VOLUME: u32 = [Self::VOLUME / 256, 1][(Self::VOLUME < 256) as usize];
    const VOLUME: u32 = (S::SIZE.x * S::SIZE.y * S::SIZE.z) as u32;

    /// Creates a new `Chunk` with the provided dimensions and all voxels filled
    /// with duplicates of the provided voxel.
    pub fn filled(default: V, meta: M) -> Self {
        // TODO (haslersn): Alter into compile time assertions
        //
        // An extent is valid if it fulfils the following conditions.
        //
        // 1. In each direction, the extent is a power of two.
        // 2. In each direction, the group size is in [1, 256].
        // 3. In each direction, the group count is in [1, 256].
        //
        // Rationales:
        //
        // 1. We have code in the implementation that assumes it. In particular,
        //    code using `.count_ones()`.
        // 2. The maximum group size is `256x256x256`, because there's code that
        //    stores group relative indices as `u8`.
        // 3. There's code that stores group indices as `u8`.
        debug_assert!(S::SIZE.x.is_power_of_two());
        debug_assert!(S::SIZE.y.is_power_of_two());
        debug_assert!(S::SIZE.z.is_power_of_two());
        debug_assert!(0 < Self::GROUP_SIZE.x);
        debug_assert!(0 < Self::GROUP_SIZE.y);
        debug_assert!(0 < Self::GROUP_SIZE.z);
        debug_assert!(Self::GROUP_SIZE.x <= 256);
        debug_assert!(Self::GROUP_SIZE.y <= 256);
        debug_assert!(Self::GROUP_SIZE.z <= 256);
        debug_assert!(0 < Self::GROUP_COUNT.x);
        debug_assert!(0 < Self::GROUP_COUNT.y);
        debug_assert!(0 < Self::GROUP_COUNT.z);
        debug_assert!(Self::GROUP_COUNT.x <= 256);
        debug_assert!(Self::GROUP_COUNT.y <= 256);
        debug_assert!(Self::GROUP_COUNT.z <= 256);

        Self {
            indices: vec![255; Self::GROUP_COUNT_TOTAL as usize],
            vox: Vec::new(),
            default,
            meta,
            phantom: PhantomData,
        }
    }

    /// Get a reference to the internal metadata.
    pub fn metadata(&self) -> &M { &self.meta }

    /// Get a mutable reference to the internal metadata.
    pub fn metadata_mut(&mut self) -> &mut M { &mut self.meta }

    #[inline(always)]
    fn grp_idx(pos: Vec3<i32>) -> u32 {
        let grp_pos = pos.map2(Self::GROUP_SIZE, |e, s| e as u32 / s);
        (grp_pos.z * (Self::GROUP_COUNT.y * Self::GROUP_COUNT.x))
            + (grp_pos.y * Self::GROUP_COUNT.x)
            + (grp_pos.x)
    }

    #[inline(always)]
    fn rel_idx(pos: Vec3<i32>) -> u32 {
        let rel_pos = pos.map2(Self::GROUP_SIZE, |e, s| e as u32 % s);
        (rel_pos.z * (Self::GROUP_SIZE.y * Self::GROUP_SIZE.x))
            + (rel_pos.y * Self::GROUP_SIZE.x)
            + (rel_pos.x)
    }

    #[inline(always)]
    fn idx_unchecked(&self, pos: Vec3<i32>) -> Option<usize> {
        let grp_idx = Self::grp_idx(pos);
        let rel_idx = Self::rel_idx(pos);
        let base = self.indices[grp_idx as usize];
        let num_groups = self.vox.len() as u32 / Self::GROUP_VOLUME;
        if base as u32 >= num_groups {
            None
        } else {
            Some((base as u32 * Self::GROUP_VOLUME + rel_idx) as usize)
        }
    }

    #[inline(always)]
    fn force_idx_unchecked(&mut self, pos: Vec3<i32>) -> usize {
        let grp_idx = Self::grp_idx(pos);
        let rel_idx = Self::rel_idx(pos);
        let base = &mut self.indices[grp_idx as usize];
        let num_groups = self.vox.len() as u32 / Self::GROUP_VOLUME;
        if *base as u32 >= num_groups {
            *base = num_groups as u8;
            self.vox
                .extend(std::iter::repeat(self.default.clone()).take(Self::GROUP_VOLUME as usize));
        }
        (*base as u32 * Self::GROUP_VOLUME + rel_idx) as usize
    }

    #[inline(always)]
    fn get_unchecked(&self, pos: Vec3<i32>) -> &V {
        match self.idx_unchecked(pos) {
            Some(idx) => &self.vox[idx],
            None => &self.default,
        }
    }

    #[inline(always)]
    fn set_unchecked(&mut self, pos: Vec3<i32>, vox: V) {
        if vox != self.default {
            let idx = self.force_idx_unchecked(pos);
            self.vox[idx] = vox;
        } else if let Some(idx) = self.idx_unchecked(pos) {
            self.vox[idx] = vox;
        }
    }
}

impl<V: Vox, S: VolSize, M> BaseVol for Chunk<V, S, M> {
    type Error = ChunkError;
    type Vox = V;
}

impl<V: Vox, S: VolSize, M> RasterableVol for Chunk<V, S, M> {
    const SIZE: Vec3<u32> = S::SIZE;
}

impl<V: Vox, S: VolSize, M> ReadVol for Chunk<V, S, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, Self::Error> {
        if !pos
            .map2(S::SIZE, |e, s| 0 <= e && e < s as i32)
            .reduce_and()
        {
            Err(Self::Error::OutOfBounds)
        } else {
            Ok(self.get_unchecked(pos))
        }
    }
}

impl<V: Vox, S: VolSize, M> WriteVol for Chunk<V, S, M> {
    #[inline(always)]
    #[allow(clippy::unit_arg)] // TODO: Pending review in #587
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), Self::Error> {
        if !pos
            .map2(S::SIZE, |e, s| 0 <= e && e < s as i32)
            .reduce_and()
        {
            Err(Self::Error::OutOfBounds)
        } else {
            Ok(self.set_unchecked(pos, vox))
        }
    }
}

pub struct ChunkPosIter<V: Vox, S: VolSize, M> {
    // Store as `u8`s so as to reduce memory footprint.
    lb: Vec3<i32>,
    ub: Vec3<i32>,
    pos: Vec3<i32>,
    phantom: PhantomData<Chunk<V, S, M>>,
}

impl<V: Vox, S: VolSize, M> ChunkPosIter<V, S, M> {
    fn new(lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self {
        // If the range is empty, then we have the special case `ub = lower_bound`.
        let ub = if lower_bound.map2(upper_bound, |l, u| l < u).reduce_and() {
            upper_bound
        } else {
            lower_bound
        };
        Self {
            lb: lower_bound,
            ub,
            pos: lower_bound,
            phantom: PhantomData,
        }
    }
}

impl<V: Vox, S: VolSize, M> Iterator for ChunkPosIter<V, S, M> {
    type Item = Vec3<i32>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos.z >= self.ub.z {
            return None;
        }
        let res = Some(self.pos);

        self.pos.x += 1;
        if self.pos.x != self.ub.x && self.pos.x % Chunk::<V, S, M>::GROUP_SIZE.x as i32 != 0 {
            return res;
        }
        self.pos.x = std::cmp::max(
            self.lb.x,
            (self.pos.x - 1) & !(Chunk::<V, S, M>::GROUP_SIZE.x as i32 - 1),
        );

        self.pos.y += 1;
        if self.pos.y != self.ub.y && self.pos.y % Chunk::<V, S, M>::GROUP_SIZE.y as i32 != 0 {
            return res;
        }
        self.pos.y = std::cmp::max(
            self.lb.y,
            (self.pos.y - 1) & !(Chunk::<V, S, M>::GROUP_SIZE.y as i32 - 1),
        );

        self.pos.z += 1;
        if self.pos.z != self.ub.z && self.pos.z % Chunk::<V, S, M>::GROUP_SIZE.z as i32 != 0 {
            return res;
        }
        self.pos.z = std::cmp::max(
            self.lb.z,
            (self.pos.z - 1) & !(Chunk::<V, S, M>::GROUP_SIZE.z as i32 - 1),
        );

        self.pos.x = (self.pos.x | (Chunk::<V, S, M>::GROUP_SIZE.x as i32 - 1)) + 1;
        if self.pos.x < self.ub.x {
            return res;
        }
        self.pos.x = self.lb.x;

        self.pos.y = (self.pos.y | (Chunk::<V, S, M>::GROUP_SIZE.y as i32 - 1)) + 1;
        if self.pos.y < self.ub.y {
            return res;
        }
        self.pos.y = self.lb.y;

        self.pos.z = (self.pos.z | (Chunk::<V, S, M>::GROUP_SIZE.z as i32 - 1)) + 1;

        res
    }
}

pub struct ChunkVolIter<'a, V: Vox, S: VolSize, M> {
    chunk: &'a Chunk<V, S, M>,
    iter_impl: ChunkPosIter<V, S, M>,
}

impl<'a, V: Vox, S: VolSize, M> Iterator for ChunkVolIter<'a, V, S, M> {
    type Item = (Vec3<i32>, &'a V);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter_impl
            .next()
            .map(|pos| (pos, self.chunk.get_unchecked(pos)))
    }
}

impl<V: Vox, S: VolSize, M> Chunk<V, S, M> {
    /// It's possible to obtain a positional iterator without having a `Chunk`
    /// instance.
    pub fn pos_iter(lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> ChunkPosIter<V, S, M> {
        ChunkPosIter::<V, S, M>::new(lower_bound, upper_bound)
    }
}

impl<'a, V: Vox, S: VolSize, M> IntoPosIterator for &'a Chunk<V, S, M> {
    type IntoIter = ChunkPosIter<V, S, M>;

    fn pos_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Chunk::<V, S, M>::pos_iter(lower_bound, upper_bound)
    }
}

impl<'a, V: Vox, S: VolSize, M> IntoVolIterator<'a> for &'a Chunk<V, S, M> {
    type IntoIter = ChunkVolIter<'a, V, S, M>;

    fn vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        ChunkVolIter::<'a, V, S, M> {
            chunk: self,
            iter_impl: ChunkPosIter::<V, S, M>::new(lower_bound, upper_bound),
        }
    }
}
