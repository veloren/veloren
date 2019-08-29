use crate::{
    vol::{
        BaseVol, DefaultVolIterator, IntoVolIterator, ReadVol, RectRasterableVol, RectVolSize,
        SizedVol, VolSize, Vox, WriteVol,
    },
    volumes::{
        chunk::{Chunk, ChunkError},
        morton::{morton_to_xyz, xyz_to_morton, MortonIter},
    },
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

    pub fn meta(&self) -> &M {
        &self.meta
    }

    pub fn get_min_z(&self) -> i32 {
        self.z_offset
    }

    pub fn get_max_z(&self) -> i32 {
        self.z_offset + (self.sub_chunks.len() as u32 * SubChunkSize::<S>::SIZE.z) as i32
    }

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

    // Returns the z_offset of the sub_chunk that contains layer z
    fn sub_chunk_z_offset(&self, z: i32) -> i32 {
        z - self.sub_chunk_z(z)
    }
}

impl<V: Vox, S: RectVolSize, M: Clone> BaseVol for Chonk<V, S, M> {
    type Vox = V;
    type Error = ChonkError;
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

struct OuterChonkIter<'a, V: Vox, S: RectVolSize, M: Clone> {
    chonk: &'a Chonk<V, S, M>,
    lower_bound: Vec3<i32>,
    upper_bound: Vec3<i32>,
}

enum OuterChonkIterItem<'a, V: Vox, S: RectVolSize, M: Clone> {
    ChunkIter(<&'a SubChunk<V, S, M> as IntoVolIterator<'a>>::IntoIter),
    DefaultIter((&'a V, MortonIter)),
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> Iterator for OuterChonkIter<'a, V, S, M> {
    type Item = OuterChonkIterItem<'a, V, S, M>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.lower_bound.z >= self.upper_bound.z {
            None
        } else {
            let sub_chunk_idx = self.chonk.sub_chunk_idx(self.lower_bound.z);
            let sub_chunk_min_z =
                self.chonk.get_min_z() + sub_chunk_idx * SubChunkSize::<S>::SIZE.z as i32;
            let mut lb = self.lower_bound;
            self.lower_bound.z = sub_chunk_min_z + SubChunkSize::<S>::SIZE.z as i32;
            let mut ub = self.upper_bound;
            ub.z = std::cmp::min(ub.z, self.upper_bound.z);
            lb.z -= sub_chunk_min_z;
            ub.z -= sub_chunk_min_z;
            if sub_chunk_idx < 0 || sub_chunk_idx >= self.chonk.sub_chunks.len() as i32 {
                let vox = if sub_chunk_idx < 0 {
                    &self.chonk.below
                } else {
                    &self.chonk.above
                };
                Some(Self::Item::DefaultIter((vox, MortonIter::new(lb, ub))))
            } else {
                Some(Self::Item::ChunkIter(
                    self.chonk.sub_chunks[sub_chunk_idx as usize].into_vol_iter(lb, ub),
                ))
            }
        }
    }
}

pub struct ChonkIter<'a, V: Vox, S: RectVolSize, M: Clone> {
    outer: OuterChonkIter<'a, V, S, M>,
    opt_inner: Option<OuterChonkIterItem<'a, V, S, M>>,
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> Iterator for ChonkIter<'a, V, S, M> {
    type Item = (Vec3<i32>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(ref mut inner) = self.opt_inner {
                match inner {
                    OuterChonkIterItem::<'a, V, S, M>::ChunkIter(i) => {
                        if let Some((mut pos, vox)) = i.next() {
                            // Transform relative coordinates back to absolute ones.
                            pos.z += self.outer.lower_bound.z - SubChunkSize::<S>::SIZE.z as i32;
                            return Some((pos, vox));
                        }
                    }
                    OuterChonkIterItem::<'a, V, S, M>::DefaultIter((vox, i)) => {
                        if let Some(morton) = i.next() {
                            let mut pos = morton_to_xyz(morton);
                            pos.z += self.outer.lower_bound.z - SubChunkSize::<S>::SIZE.z as i32;
                            return Some((pos, vox));
                        }
                    }
                }
            }
            match self.outer.next() {
                None => return None,
                opt_inner @ Some(_) => self.opt_inner = opt_inner,
            }
        }
    }
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> IntoVolIterator<'a> for &'a Chonk<V, S, M> {
    type IntoIter = ChonkIter<'a, V, S, M>;

    fn into_vol_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter {
            outer: OuterChonkIter::<'a, V, S, M> {
                chonk: &self,
                lower_bound,
                upper_bound,
            },
            opt_inner: None,
        }
    }
}
