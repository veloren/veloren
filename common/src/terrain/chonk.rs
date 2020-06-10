use crate::{
    vol::{
        BaseVol, IntoPosIterator, IntoVolIterator, ReadVol, RectRasterableVol, RectVolSize,
        VolSize, Vox, WriteVol,
    },
    volumes::chunk::{Chunk, ChunkError, ChunkPosIter, ChunkVolIter},
};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;
use vek::*;

#[derive(Debug)]
pub enum ChonkError {
    SubChunkError(ChunkError),
    OutOfBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubChunkSize<ChonkSize: RectVolSize> {
    phantom: PhantomData<ChonkSize>,
}

// TODO (haslersn): Assert ChonkSize::RECT_SIZE.x == ChonkSize::RECT_SIZE.y

impl<ChonkSize: RectVolSize> VolSize for SubChunkSize<ChonkSize> {
    const SIZE: Vec3<u32> = Vec3 {
        x: ChonkSize::RECT_SIZE.x,
        y: ChonkSize::RECT_SIZE.x,
        // NOTE: Currently, use 32 instead of 2 for RECT_SIZE.x = 128.
        z: ChonkSize::RECT_SIZE.x / 2,
    };
}

type SubChunk<V, S, M> = Chunk<V, SubChunkSize<S>, M>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chonk<V: Vox, S: RectVolSize, M: Clone> {
    z_offset: i32,
    sub_chunks: Vec<SubChunk<V, S, M>>,
    below: V,
    above: V,
    meta: M,
    phantom: PhantomData<S>,
}

impl<V: Vox, S: RectVolSize, M: Clone> Chonk<V, S, M> {
    pub fn new(z_offset: i32, below: V, above: V, meta: M) -> Self {
        Self {
            z_offset,
            sub_chunks: Vec::new(),
            below,
            above,
            meta,
            phantom: PhantomData,
        }
    }

    pub fn meta(&self) -> &M { &self.meta }

    pub fn get_min_z(&self) -> i32 { self.z_offset }

    pub fn get_max_z(&self) -> i32 {
        self.z_offset + (self.sub_chunks.len() as u32 * SubChunkSize::<S>::SIZE.z) as i32
    }

    pub fn sub_chunks_len(&self) -> usize { self.sub_chunks.len() }

    // Returns the index (in self.sub_chunks) of the SubChunk that contains
    // layer z; note that this index changes when more SubChunks are prepended
    fn sub_chunk_idx(&self, z: i32) -> i32 {
        let diff = z - self.z_offset;
        diff >> (SubChunkSize::<S>::SIZE.z - 1).count_ones()
    }

    // Converts a z coordinate into a local z coordinate within a sub chunk
    fn sub_chunk_z(&self, z: i32) -> i32 {
        let diff = z - self.z_offset;
        diff & (SubChunkSize::<S>::SIZE.z - 1) as i32
    }

    // Returns the z offset of the sub_chunk that contains layer z
    fn sub_chunk_min_z(&self, z: i32) -> i32 { z - self.sub_chunk_z(z) }
}

impl<V: Vox, S: RectVolSize, M: Clone> BaseVol for Chonk<V, S, M> {
    type Error = ChonkError;
    type Vox = V;
}

impl<V: Vox, S: RectVolSize, M: Clone> RectRasterableVol for Chonk<V, S, M> {
    const RECT_SIZE: Vec2<u32> = S::RECT_SIZE;
}

impl<V: Vox, S: RectVolSize, M: Clone> ReadVol for Chonk<V, S, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V, Self::Error> {
        if pos.z < self.get_min_z() {
            // Below the terrain
            Ok(&self.below)
        } else if pos.z >= self.get_max_z() {
            // Above the terrain
            Ok(&self.above)
        } else {
            // Within the terrain
            let sub_chunk_idx = self.sub_chunk_idx(pos.z);
            let rpos = pos
                - Vec3::unit_z()
                    * (self.z_offset + sub_chunk_idx * SubChunkSize::<S>::SIZE.z as i32);
            self.sub_chunks[sub_chunk_idx as usize]
                .get(rpos)
                .map_err(Self::Error::SubChunkError)
        }
    }
}

impl<V: Vox, S: RectVolSize, M: Clone> WriteVol for Chonk<V, S, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, block: Self::Vox) -> Result<(), Self::Error> {
        let mut sub_chunk_idx = self.sub_chunk_idx(pos.z);

        if pos.z < self.get_min_z() {
            // Prepend exactly sufficiently many SubChunks via Vec::splice
            let c = Chunk::<V, SubChunkSize<S>, M>::filled(self.below.clone(), self.meta.clone());
            let n = (-sub_chunk_idx) as usize;
            self.sub_chunks.splice(0..0, std::iter::repeat(c).take(n));
            self.z_offset += sub_chunk_idx * SubChunkSize::<S>::SIZE.z as i32;
            sub_chunk_idx = 0;
        } else if pos.z >= self.get_max_z() {
            // Append exactly sufficiently many SubChunks via Vec::extend
            let c = Chunk::<V, SubChunkSize<S>, M>::filled(self.above.clone(), self.meta.clone());
            let n = 1 + sub_chunk_idx as usize - self.sub_chunks.len();
            self.sub_chunks.extend(std::iter::repeat(c).take(n));
        }

        let rpos = pos
            - Vec3::unit_z() * (self.z_offset + sub_chunk_idx * SubChunkSize::<S>::SIZE.z as i32);
        self.sub_chunks[sub_chunk_idx as usize] // TODO (haslersn): self.sub_chunks.get(...).and_then(...)
            .set(rpos, block)
            .map_err(Self::Error::SubChunkError)
    }
}

struct ChonkIterHelper<V: Vox, S: RectVolSize, M: Clone> {
    sub_chunk_min_z: i32,
    lower_bound: Vec3<i32>,
    upper_bound: Vec3<i32>,
    phantom: PhantomData<Chonk<V, S, M>>,
}

impl<V: Vox, S: RectVolSize, M: Clone> Iterator for ChonkIterHelper<V, S, M> {
    type Item = (i32, Vec3<i32>, Vec3<i32>);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.lower_bound.z >= self.upper_bound.z {
            return None;
        }
        let mut lb = self.lower_bound;
        let mut ub = self.upper_bound;
        let current_min_z = self.sub_chunk_min_z;
        lb.z -= current_min_z;
        ub.z -= current_min_z;
        ub.z = std::cmp::min(ub.z, SubChunkSize::<S>::SIZE.z as i32);
        self.sub_chunk_min_z += SubChunkSize::<S>::SIZE.z as i32;
        self.lower_bound.z = self.sub_chunk_min_z;
        Some((current_min_z, lb, ub))
    }
}

#[allow(clippy::type_complexity)] // TODO: Pending review in #587
pub struct ChonkPosIter<V: Vox, S: RectVolSize, M: Clone> {
    outer: ChonkIterHelper<V, S, M>,
    opt_inner: Option<(i32, ChunkPosIter<V, SubChunkSize<S>, M>)>,
}

impl<V: Vox, S: RectVolSize, M: Clone> Iterator for ChonkPosIter<V, S, M> {
    type Item = Vec3<i32>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((sub_chunk_min_z, ref mut inner)) = self.opt_inner {
                if let Some(mut pos) = inner.next() {
                    pos.z += sub_chunk_min_z;
                    return Some(pos);
                }
            }
            match self.outer.next() {
                None => return None,
                Some((sub_chunk_min_z, lb, ub)) => {
                    self.opt_inner = Some((sub_chunk_min_z, SubChunk::<V, S, M>::pos_iter(lb, ub)))
                },
            }
        }
    }
}

enum InnerChonkVolIter<'a, V: Vox, S: RectVolSize, M: Clone> {
    Vol(ChunkVolIter<'a, V, SubChunkSize<S>, M>),
    Pos(ChunkPosIter<V, SubChunkSize<S>, M>),
}

pub struct ChonkVolIter<'a, V: Vox, S: RectVolSize, M: Clone> {
    chonk: &'a Chonk<V, S, M>,
    outer: ChonkIterHelper<V, S, M>,
    opt_inner: Option<(i32, InnerChonkVolIter<'a, V, S, M>)>,
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> Iterator for ChonkVolIter<'a, V, S, M> {
    type Item = (Vec3<i32>, &'a V);

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((sub_chunk_min_z, ref mut inner)) = self.opt_inner {
                let got = match inner {
                    InnerChonkVolIter::<'a, V, S, M>::Vol(iter) => iter.next(),
                    InnerChonkVolIter::<'a, V, S, M>::Pos(iter) => iter.next().map(|pos| {
                        if sub_chunk_min_z < self.chonk.get_min_z() {
                            (pos, &self.chonk.below)
                        } else {
                            (pos, &self.chonk.above)
                        }
                    }),
                };
                if let Some((mut pos, vox)) = got {
                    pos.z += sub_chunk_min_z;
                    return Some((pos, vox));
                }
            }
            match self.outer.next() {
                None => return None,
                Some((sub_chunk_min_z, lb, ub)) => {
                    let inner = if sub_chunk_min_z < self.chonk.get_min_z()
                        || sub_chunk_min_z >= self.chonk.get_max_z()
                    {
                        InnerChonkVolIter::<'a, V, S, M>::Pos(SubChunk::<V, S, M>::pos_iter(lb, ub))
                    } else {
                        InnerChonkVolIter::<'a, V, S, M>::Vol(
                            self.chonk.sub_chunks
                                [self.chonk.sub_chunk_idx(sub_chunk_min_z) as usize]
                                .vol_iter(lb, ub),
                        )
                    };
                    self.opt_inner = Some((sub_chunk_min_z, inner));
                },
            }
        }
    }
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> IntoPosIterator for &'a Chonk<V, S, M> {
    type IntoIter = ChonkPosIter<V, S, M>;

    fn pos_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter {
            outer: ChonkIterHelper::<V, S, M> {
                sub_chunk_min_z: self.sub_chunk_min_z(lower_bound.z),
                lower_bound,
                upper_bound,
                phantom: PhantomData,
            },
            opt_inner: None,
        }
    }
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> IntoVolIterator<'a> for &'a Chonk<V, S, M> {
    type IntoIter = ChonkVolIter<'a, V, S, M>;

    fn vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter {
            chonk: self,
            outer: ChonkIterHelper::<V, S, M> {
                sub_chunk_min_z: self.sub_chunk_min_z(lower_bound.z),
                lower_bound,
                upper_bound,
                phantom: PhantomData,
            },
            opt_inner: None,
        }
    }
}
