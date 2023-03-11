use crate::vol::{
    BaseVol, IntoPosIterator, IntoVolIterator, RasterableVol, ReadVol, VolSize, WriteVol,
};
use core::{hash::Hash, iter::Iterator, marker::PhantomData, mem};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
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
pub struct Chunk<V, S: VolSize, M> {
    indices: Vec<u8>, /* TODO (haslersn): Box<[u8; S::SIZE.x * S::SIZE.y * S::SIZE.z]>, this is
                       * however not possible in Rust yet */
    vox: Vec<V>,
    default: V,
    meta: M,
    phantom: PhantomData<S>,
}

impl<V, S: VolSize, M> Chunk<V, S, M> {
    pub const GROUP_COUNT: Vec3<u32> = Vec3::new(
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
    const VOLUME: u32 = S::SIZE.x * S::SIZE.y * S::SIZE.z;

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

    /// Compress this subchunk by frequency.
    pub fn defragment(&mut self)
    where
        V: Clone + Eq + Hash,
    {
        // First, construct a HashMap with max capacity equal to GROUP_COUNT (since each
        // filled group can have at most one slot).
        let mut map: HashMap<_, Vec<_>> = HashMap::with_capacity(Self::GROUP_COUNT_TOTAL as usize);
        let vox = &self.vox;
        let default = &self.default;
        self.indices
            .iter()
            .enumerate()
            .for_each(|(grp_idx, &base)| {
                let start = usize::from(base) * Self::GROUP_VOLUME as usize;
                let end = start + Self::GROUP_VOLUME as usize;
                if let Some(group) = vox.get(start..end) {
                    // Check to see if all blocks in this group are the same.
                    let mut group = group.iter();
                    let first = group.next().expect("GROUP_VOLUME ≥ 1");
                    if group.all(|block| block == first) {
                        // All blocks in the group were the same, so add our position to this entry
                        // in the HashMap.
                        map.entry(first).or_default().push(grp_idx);
                    }
                } else {
                    // This slot is empty (i.e. has the default value).
                    map.entry(default).or_default().push(grp_idx);
                }
            });
        // Now, find the block with max frequency in the HashMap and make that our new
        // default.
        let (new_default, default_groups) = if let Some((new_default, default_groups)) = map
            .into_iter()
            .max_by_key(|(_, default_groups)| default_groups.len())
        {
            (new_default.clone(), default_groups)
        } else {
            // There is no good choice for default group, so leave it as is.
            return;
        };

        // For simplicity, we construct a completely new voxel array rather than
        // attempting in-place updates (TODO: consider changing this).
        let mut new_vox =
            Vec::with_capacity(Self::GROUP_COUNT_TOTAL as usize - default_groups.len());
        let num_groups = self.num_groups();
        self.indices
            .iter_mut()
            .enumerate()
            .for_each(|(grp_idx, base)| {
                if default_groups.contains(&grp_idx) {
                    // Default groups become 255
                    *base = 255;
                } else {
                    // Other groups are allocated in increasing order by group index.
                    // NOTE: Cannot overflow since the current implicit group index can't be at the
                    // end of the vector until at the earliest after the 256th iteration.
                    let old_base = usize::from(mem::replace(
                        base,
                        (new_vox.len() / Self::GROUP_VOLUME as usize) as u8,
                    ));
                    if old_base >= num_groups {
                        // Old default, which (since we reached this branch) is not equal to the new
                        // default, so we have to write out the old default.
                        new_vox
                            .resize(new_vox.len() + Self::GROUP_VOLUME as usize, default.clone());
                    } else {
                        let start = old_base * Self::GROUP_VOLUME as usize;
                        let end = start + Self::GROUP_VOLUME as usize;
                        new_vox.extend_from_slice(&vox[start..end]);
                    }
                }
            });

        // Finally, reset our vox and default values to the new ones.
        self.vox = new_vox;
        self.default = new_default;
    }

    /// Get a reference to the internal metadata.
    pub fn metadata(&self) -> &M { &self.meta }

    /// Get a mutable reference to the internal metadata.
    pub fn metadata_mut(&mut self) -> &mut M { &mut self.meta }

    pub fn num_groups(&self) -> usize { self.vox.len() / Self::GROUP_VOLUME as usize }

    /// Returns `Some(v)` if the block is homogeneous and contains nothing but
    /// voxels of value `v`, and `None` otherwise.  This method is
    /// conservative (it may return None when the chunk is
    /// actually homogeneous) unless called immediately after `defragment`.
    pub fn homogeneous(&self) -> Option<&V> {
        if self.num_groups() == 0 {
            Some(&self.default)
        } else {
            None
        }
    }

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
        let base = u32::from(self.indices[grp_idx as usize]);
        let num_groups = self.vox.len() as u32 / Self::GROUP_VOLUME;
        if base >= num_groups {
            None
        } else {
            Some((base * Self::GROUP_VOLUME + rel_idx) as usize)
        }
    }

    #[inline(always)]
    fn force_idx_unchecked(&mut self, pos: Vec3<i32>) -> usize
    where
        V: Clone,
    {
        let grp_idx = Self::grp_idx(pos);
        let rel_idx = Self::rel_idx(pos);
        let base = &mut self.indices[grp_idx as usize];
        let num_groups = self.vox.len() as u32 / Self::GROUP_VOLUME;
        if u32::from(*base) >= num_groups {
            *base = num_groups as u8;
            self.vox
                .extend(std::iter::repeat(self.default.clone()).take(Self::GROUP_VOLUME as usize));
        }
        (u32::from(*base) * Self::GROUP_VOLUME + rel_idx) as usize
    }

    #[inline(always)]
    fn get_unchecked(&self, pos: Vec3<i32>) -> &V {
        match self.idx_unchecked(pos) {
            Some(idx) => &self.vox[idx],
            None => &self.default,
        }
    }

    #[inline(always)]
    fn set_unchecked(&mut self, pos: Vec3<i32>, vox: V) -> V
    where
        V: Clone + PartialEq,
    {
        if vox != self.default {
            let idx = self.force_idx_unchecked(pos);
            mem::replace(&mut self.vox[idx], vox)
        } else if let Some(idx) = self.idx_unchecked(pos) {
            mem::replace(&mut self.vox[idx], vox)
        } else {
            self.default.clone()
        }
    }
}

impl<V, S: VolSize, M> BaseVol for Chunk<V, S, M> {
    type Error = ChunkError;
    type Vox = V;
}

impl<V, S: VolSize, M> RasterableVol for Chunk<V, S, M> {
    const SIZE: Vec3<u32> = S::SIZE;
}

impl<V, S: VolSize, M> ReadVol for Chunk<V, S, M> {
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

    #[inline(always)]
    fn get_unchecked(&self, pos: Vec3<i32>) -> &Self::Vox { self.get_unchecked(pos) }

    fn for_each_in(&self, mut aabb: Aabb<i32>, mut f: impl FnMut(Vec3<i32>, Self::Vox))
    where
        Self::Vox: Copy,
    {
        aabb.intersect(Aabb {
            min: Vec3::zero(),
            max: S::SIZE.map(|e| e as i32) - 1,
        });
        for z in aabb.min.z..aabb.max.z + 1 {
            for y in aabb.min.y..aabb.max.y + 1 {
                for x in aabb.min.x..aabb.max.x + 1 {
                    f(Vec3::new(x, y, z), *self.get_unchecked(Vec3::new(x, y, z)));
                }
            }
        }
    }
}

impl<V: Clone + PartialEq, S: VolSize, M> WriteVol for Chunk<V, S, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<Self::Vox, Self::Error> {
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

pub struct ChunkPosIter<V, S: VolSize, M> {
    // Store as `u8`s so as to reduce memory footprint.
    lb: Vec3<i32>,
    ub: Vec3<i32>,
    pos: Vec3<i32>,
    phantom: PhantomData<Chunk<V, S, M>>,
}

impl<V, S: VolSize, M> ChunkPosIter<V, S, M> {
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

impl<V, S: VolSize, M> Iterator for ChunkPosIter<V, S, M> {
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

pub struct ChunkVolIter<'a, V, S: VolSize, M> {
    chunk: &'a Chunk<V, S, M>,
    iter_impl: ChunkPosIter<V, S, M>,
}

impl<'a, V, S: VolSize, M> Iterator for ChunkVolIter<'a, V, S, M> {
    type Item = (Vec3<i32>, &'a V);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter_impl
            .next()
            .map(|pos| (pos, self.chunk.get_unchecked(pos)))
    }
}

impl<V, S: VolSize, M> Chunk<V, S, M> {
    /// It's possible to obtain a positional iterator without having a `Chunk`
    /// instance.
    pub fn pos_iter(lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> ChunkPosIter<V, S, M> {
        ChunkPosIter::<V, S, M>::new(lower_bound, upper_bound)
    }
}

impl<'a, V, S: VolSize, M> IntoPosIterator for &'a Chunk<V, S, M> {
    type IntoIter = ChunkPosIter<V, S, M>;

    fn pos_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Chunk::<V, S, M>::pos_iter(lower_bound, upper_bound)
    }
}

impl<'a, V, S: VolSize, M> IntoVolIterator<'a> for &'a Chunk<V, S, M> {
    type IntoIter = ChunkVolIter<'a, V, S, M>;

    fn vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        ChunkVolIter::<'a, V, S, M> {
            chunk: self,
            iter_impl: ChunkPosIter::<V, S, M>::new(lower_bound, upper_bound),
        }
    }
}
