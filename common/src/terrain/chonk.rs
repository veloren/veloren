use crate::{
    vol::{
        BaseVol, IntoPosIterator, IntoVolIterator, ReadVol, RectRasterableVol, RectVolSize,
        SizedVol, VolSize, Vox, WriteVol,
    },
    volumes::chunk::{Chunk, ChunkError, ChunkPos, ChunkPosIter},
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
    fn sub_chunk_relative_z(&self, z: i32) -> i32 {
        let diff = z - self.z_offset;
        diff & (SubChunkSize::<S>::SIZE.z - 1) as i32
    }

    // Returns the z_offset of the sub_chunk that contains layer z
    fn sub_chunk_min_z(&self, z: i32) -> i32 {
        z - self.sub_chunk_relative_z(z)
    }
}

/// The type parameters make sure that `ChonkPos` instances from different
/// `Chonk` types can't be mixed. Note: They shouldn't even be mixed when
/// obtained from different `Chonk` INSTANCES which isn't enforced.
#[derive(Clone)]
pub struct ChonkPos<S: RectVolSize> {
    sub_chunk_min_z: i32,
    sub_chunk_pos: ChunkPos<SubChunkSize<S>>,
}

impl<V: Vox, S: RectVolSize, M: Clone> BaseVol for Chonk<V, S, M> {
    type Vox = V;
    type Error = ChonkError;
    type Pos = ChonkPos<S>;

    fn to_pos(&self, pos: Vec3<i32>) -> Result<Self::Pos, Self::Error> {
        let relative_z = self.sub_chunk_relative_z(pos.z);
        match SubChunk::<V, S, M>::to_pos(Vec3::new(pos.x, pos.y, relative_z)) {
            Ok(sub_chunk_pos) => Ok(ChonkPos {
                sub_chunk_min_z: pos.z - relative_z,
                sub_chunk_pos,
            }),
            Err(err) => Err(Self::Error::SubChunkError(err)),
        }
    }

    fn to_vec3(&self, pos: Self::Pos) -> Vec3<i32> {
        let mut p = SubChunk::<V, S, M>::to_vec3(pos.sub_chunk_pos);
        p.z += pos.sub_chunk_min_z;
        p
    }
}

impl<V: Vox, S: RectVolSize, M: Clone> RectRasterableVol for Chonk<V, S, M> {
    const RECT_SIZE: Vec2<u32> = S::RECT_SIZE;
}

impl<V: Vox, S: RectVolSize, M: Clone> ReadVol for Chonk<V, S, M> {
    #[inline(always)]
    fn get_pos(&self, pos: Self::Pos) -> &V {
        // If this assertion fails, then a `Pos` that was obtained from a
        // different chonk was used.
        debug_assert_eq!(
            pos.sub_chunk_min_z,
            self.sub_chunk_min_z(pos.sub_chunk_min_z)
        );
        if pos.sub_chunk_min_z < self.get_min_z() {
            // Below the terrain
            &self.below
        } else if pos.sub_chunk_min_z >= self.get_max_z() {
            // Above the terrain
            &self.above
        } else {
            // Within the terrain
            let sub_chunk_idx = self.sub_chunk_idx(pos.sub_chunk_min_z);
            self.sub_chunks[sub_chunk_idx as usize].get_pos(pos.sub_chunk_pos)
        }
    }
}

impl<V: Vox, S: RectVolSize, M: Clone> WriteVol for Chonk<V, S, M> {
    #[inline(always)]
    fn set_pos(&mut self, pos: Self::Pos, vox: V) {
        // If this assertion fails, then a `Pos` that was obtained from a
        // different chonk was used.
        debug_assert_eq!(
            pos.sub_chunk_min_z,
            self.sub_chunk_min_z(pos.sub_chunk_min_z)
        );

        let mut sub_chunk_idx = self.sub_chunk_idx(pos.sub_chunk_min_z);

        if sub_chunk_idx < 0 {
            // Prepend exactly sufficiently many SubChunks via Vec::splice
            let c = Chunk::<V, SubChunkSize<S>, M>::filled(self.below.clone(), self.meta.clone());
            let n = (-sub_chunk_idx) as usize;
            self.sub_chunks.splice(0..0, std::iter::repeat(c).take(n));
            self.z_offset += sub_chunk_idx * SubChunkSize::<S>::SIZE.z as i32;
            sub_chunk_idx = 0;
        } else if sub_chunk_idx >= self.sub_chunks.len() as i32 {
            // Append exactly sufficiently many SubChunks via Vec::extend
            let c = Chunk::<V, SubChunkSize<S>, M>::filled(self.above.clone(), self.meta.clone());
            let n = 1 + sub_chunk_idx as usize - self.sub_chunks.len();
            self.sub_chunks.extend(std::iter::repeat(c).take(n));
        }

        self.sub_chunks[sub_chunk_idx as usize].set_pos(pos.sub_chunk_pos, vox);
    }
}

impl<V: Vox, S: RectVolSize, M: Clone> SizedVol for Chonk<V, S, M> {
    fn lower_bound(&self) -> Vec3<i32> {
        Vec3::new(0, 0, self.get_min_z())
    }

    fn upper_bound(&self) -> Vec3<i32> {
        Vec3::new(
            Self::RECT_SIZE.x as i32,
            Self::RECT_SIZE.y as i32,
            self.get_max_z(),
        )
    }
}

struct OuterChonkIter<S: RectVolSize> {
    sub_chunk_min_z: i32,
    lower_bound: Vec3<i32>,
    upper_bound: Vec3<i32>,
    phantom: PhantomData<S>,
}

impl<S: RectVolSize> Iterator for OuterChonkIter<S> {
    type Item = (i32, ChunkPosIter<SubChunkSize<S>>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.lower_bound.z >= self.upper_bound.z {
            None
        } else {
            let mut lb = self.lower_bound;
            let mut ub = self.upper_bound;
            lb.z -= self.sub_chunk_min_z;
            ub.z -= self.sub_chunk_min_z;
            ub.z = std::cmp::min(ub.z, SubChunkSize::<S>::SIZE.z as i32);
            let current_min_z = self.sub_chunk_min_z;
            self.sub_chunk_min_z += SubChunkSize::<S>::SIZE.z as i32;
            self.lower_bound.z = self.sub_chunk_min_z;
            Some((current_min_z, ChunkPosIter::new(lb, ub)))
        }
    }
}

pub struct ChonkPosIter<S: RectVolSize> {
    outer: OuterChonkIter<S>,
    opt_inner: Option<<OuterChonkIter<S> as Iterator>::Item>,
}

impl<S: RectVolSize> Iterator for ChonkPosIter<S> {
    type Item = ChonkPos<S>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some((sub_chunk_min_z, ref mut inner)) = self.opt_inner {
                if let Some(sub_chunk_pos) = inner.next() {
                    return Some(ChonkPos::<S> {
                        sub_chunk_min_z,
                        sub_chunk_pos,
                    });
                }
            }
            match self.outer.next() {
                None => return None,
                opt_inner @ Some(_) => self.opt_inner = opt_inner,
            }
        }
    }
}

pub struct ChonkVolIter<'a, V: Vox, S: RectVolSize, M: Clone> {
    chonk: &'a Chonk<V, S, M>,
    iter: ChonkPosIter<S>,
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> Iterator for ChonkVolIter<'a, V, S, M> {
    type Item = (ChonkPos<S>, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|pos| (pos.clone(), self.chonk.get_pos(pos)))
    }
}

impl<'a, V: Vox, S: RectVolSize, M: Clone> IntoPosIterator for &'a Chonk<V, S, M> {
    type IntoIter = ChonkPosIter<S>;

    fn pos_iter(self, lower_bound: Vec3<i32>, upper_bound: Vec3<i32>) -> Self::IntoIter {
        Self::IntoIter {
            outer: OuterChonkIter::<S> {
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
            chonk: &self,
            iter: self.pos_iter(lower_bound, upper_bound),
        }
    }
}
