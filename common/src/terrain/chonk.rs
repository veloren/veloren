use crate::{
    vol::{BaseVol, ReadVol, SizedVol, VolSize, Vox, WriteVol},
    volumes::chunk::{Chunk, ChunkErr},
};
use serde_derive::{Deserialize, Serialize};
use std::marker::PhantomData;
use vek::*;

#[derive(Debug)]
pub enum ChonkError {
    SubChunkError(ChunkErr),
    OutOfBounds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubChunkSize<ChonkSize: VolSize> {
    phantom: PhantomData<ChonkSize>,
}
// TODO (haslersn): Assert ChonkSize::SIZE.x == ChonkSize::SIZE.y

impl<ChonkSize: VolSize> VolSize for SubChunkSize<ChonkSize> {
    const SIZE: Vec3<u32> = Vec3 {
        x: ChonkSize::SIZE.x,
        y: ChonkSize::SIZE.x,
        z: ChonkSize::SIZE.x / 2,
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chonk<V: Vox, S: VolSize, M : Clone> {
    z_offset: i32,
    sub_chunks: Vec<Chunk<V, SubChunkSize<S>, M>>,
    below: V,
    above: V,
    meta: M,
    phantom: PhantomData<S>,
}

impl<V: Vox, S: VolSize, M : Clone> Chonk<V, S, M> {
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
    fn sub_chunk_idx(&self, z: i32) -> usize {
        ((z - self.z_offset) / SubChunkSize::<S>::SIZE.z as i32) as usize
    }

    // Returns the z_offset of the sub_chunk that contains layer z
    fn sub_chunk_z_offset(&self, z: i32) -> i32 {
        let rem = (z - self.z_offset) % SubChunkSize::<S>::SIZE.z as i32;
        if rem < 0 {
            z - (rem + SubChunkSize::<S>::SIZE.z as i32)
        } else {
            z - rem
        }
    }
}

impl<V: Vox, S: VolSize, M : Clone> BaseVol for Chonk<V, S, M> {
    type Vox = V;
    type Err = ChonkError;
}

impl<V: Vox, S: VolSize, M : Clone> SizedVol for Chonk<V, S, M> {
    #[inline(always)]
    fn get_size(&self) -> Vec3<u32> {
        S::SIZE
    }
}

impl<V: Vox, S: VolSize, M : Clone> ReadVol for Chonk<V, S, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V, Self::Err> {
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
                    * (self.z_offset + sub_chunk_idx as i32 * SubChunkSize::<S>::SIZE.z as i32);
            self.sub_chunks[sub_chunk_idx]
                .get(rpos)
                .map_err(Self::Err::SubChunkError)
        }
    }
}

impl<V: Vox, S: VolSize, M : Clone> WriteVol for Chonk<V, S, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, block: Self::Vox) -> Result<(), Self::Err> {
        if pos.z < self.get_min_z() {
            // Prepend exactly sufficiently many SubChunks via Vec::splice
            let target_z_offset = self.sub_chunk_z_offset(pos.z);
            let c = Chunk::<V, SubChunkSize<S>, M>::filled(self.below.clone(), self.meta.clone());
            let n = (self.get_min_z() - target_z_offset) / SubChunkSize::<S>::SIZE.z as i32;
            self.sub_chunks
                .splice(0..0, std::iter::repeat(c).take(n as usize));
            self.z_offset = target_z_offset;
        } else if pos.z >= self.get_max_z() {
            // Append exactly sufficiently many SubChunks via Vec::extend
            let target_z_offset = self.sub_chunk_z_offset(pos.z);
            let c = Chunk::<V, SubChunkSize<S>, M>::filled(self.above.clone(), self.meta.clone());
            let n = (target_z_offset - self.get_max_z()) / SubChunkSize::<S>::SIZE.z as i32 + 1;
            self.sub_chunks
                .extend(std::iter::repeat(c).take(n as usize));
        }

        let sub_chunk_idx = self.sub_chunk_idx(pos.z);
        let rpos = pos
            - Vec3::unit_z() * (self.z_offset + sub_chunk_idx as i32 * SubChunkSize::<S>::SIZE.z as i32);
        self.sub_chunks[sub_chunk_idx]
            .set(rpos, block)
            .map_err(Self::Err::SubChunkError)
    }
}
