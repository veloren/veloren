use crate::vol::{BaseVol, ReadVol, SizedVol, VolSize, Vox, WriteVol};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;
use vek::*;

#[derive(Debug)]
pub enum ChunkErr {
    OutOfBounds,
}

/// Converts `(x, 0, 0)` to its index in the morton order. Arithmetically that
/// is, the bits of `x` are interleaved by two zero bits between every adjacent
/// pair of bits. Precondition is `0 <= x && x < 2048`.
///
/// Eidetic:
///
/// ```
/// x00_to_morton(0bKJIHGFEDCBA) == 0bK00J00I00H00G00F00E00D00C00B00A
/// ```
#[inline(always)]
fn x00_to_morton(x: i32) -> u32 {
    // Because the result is u32, we support only inputs of up to 11 bits:
    assert!(0 <= x && x < 2048); // 2048 = 2^11
    let mut m = x as u32;
    m = (m | (m << 12)) & 0x7C003F;
    m = (m | (m << 4) | (m << 8)) & 0x430C30C3;
    m = (m | (m << 2)) & 0x49249249;
    m
}

/// Converts (pos.x, pos.y, pos.z) to its index in the morton order.
/// Arithmetically that is, the bits of `pos.x`, `pos.y` and `pos.z` are
/// interleaved such that the least (resp. 2nd to least, resp. 3rd to least)
/// significant bit and every third bit from thereon originates from `pos.x`
/// (resp. `pos.y`, resp. `pos.z`). Precondition is for all `c` in `pos`:
/// `0 <= c && c < 2048`.
///
/// Eidetic:
///
/// ```
/// xyz_to_morton(Vec3<i32>::new(
///     0bKJIHGFEDCBA,
///     0b00000000000,
///     0b11111111111
/// )) == 0bK10J10I10H10G10F10E10D10C10B10A
/// ```
#[inline(always)]
fn xyz_to_morton(pos: Vec3<i32>) -> u32 {
    let x = x00_to_morton(pos.x);
    let y = x00_to_morton(pos.y) << 1;
    let z = x00_to_morton(pos.z) << 2;
    x | y | z
}

fn morton_to_x(morton: u32) -> i32 {
    let mut m = ((morton & 0x08208208) >> 2) | (morton & 0x41041041);
    m = ((m & 0x40003000) >> 8) | ((m & 0x30000C0) >> 4) | (m & 0xC0003);
    m = ((m & 0xFC0000) >> 12) | (m & 0x3F);
    m as i32
}

fn morton_to_y(morton: u32) -> i32 {
    morton_to_x(morton >> 1)
}

fn morton_to_z(morton: u32) -> i32 {
    morton_to_x(morton >> 2)
}

fn morton_to_xyz(morton: u32) -> Vec3<i32> {
    Vec3::<i32>::new(
        morton_to_x(morton),
        morton_to_y(morton),
        morton_to_z(morton),
    )
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
        assert!(0 < S::SIZE.x);
        assert!(0 < S::SIZE.y);
        assert!(0 < S::SIZE.z);
        assert!(S::SIZE.x <= 64);
        assert!(S::SIZE.y <= 64);
        assert!(S::SIZE.z <= 64);
        assert!(S::SIZE.x & (S::SIZE.x - 1) == 0);
        assert!(S::SIZE.y & (S::SIZE.y - 1) == 0);
        assert!(S::SIZE.z & (S::SIZE.z - 1) == 0);
        assert!(S::SIZE.x >= S::SIZE.y);
        assert!(S::SIZE.y >= S::SIZE.z);
        assert!(2 * S::SIZE.z >= S::SIZE.x);

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

    fn get_from_morton(&self, morton: u32) -> Result<&V, ChunkErr> {
        if morton as usize >= Self::BLOCK_COUNT {
            Err(ChunkErr::OutOfBounds)
        } else {
            Ok(self.get_from_morton_unchecked(morton))
        }
    }

    fn set_from_morton_unchecked(&mut self, morton: u32, vox: V) {
        let idx = self.force_idx_from_morton_unchecked(morton);
        self.vox[idx] = vox;
    }

    fn set_from_morton(&mut self, morton: u32, vox: V) -> Result<(), ChunkErr> {
        if morton as usize >= Self::BLOCK_COUNT {
            Err(ChunkErr::OutOfBounds)
        } else {
            Ok(self.set_from_morton_unchecked(morton, vox))
        }
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
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, ChunkErr> {
        self.get_from_morton(xyz_to_morton(pos))
    }
}

impl<V: Vox, S: VolSize, M> WriteVol for Chunk<V, S, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: Self::Vox) -> Result<(), ChunkErr> {
        self.set_from_morton(xyz_to_morton(pos), vox)
    }
}
