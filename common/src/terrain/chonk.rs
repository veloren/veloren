use super::{block::Block, TerrainChunkMeta, TerrainChunkSize};
use crate::{
    vol::{BaseVol, ReadVol, VolSize, WriteVol},
    volumes::chunk::{Chunk, ChunkErr},
};
use hashbrown::HashMap;
use serde_derive::{Deserialize, Serialize};
use std::ops::Add;
use vek::*;

#[derive(Debug)]
pub enum ChonkError {
    ChunkError(ChunkErr),
    OutOfBounds,
}

const SUB_CHUNK_HEIGHT: u32 = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubChunkSize;

impl VolSize for SubChunkSize {
    const SIZE: Vec3<u32> = Vec3 {
        x: TerrainChunkSize::SIZE.x,
        y: TerrainChunkSize::SIZE.y,
        z: SUB_CHUNK_HEIGHT,
    };
}

const SUB_CHUNK_HASH_LIMIT: usize =
    (SubChunkSize::SIZE.x * SubChunkSize::SIZE.y * SubChunkSize::SIZE.z) as usize / 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chonk {
    z_offset: i32,
    sub_chunks: Vec<SubChunk>,
    below: Block,
    above: Block,
    meta: TerrainChunkMeta,
}

impl Chonk {
    pub fn new(z_offset: i32, below: Block, above: Block, meta: TerrainChunkMeta) -> Self {
        Self {
            z_offset,
            sub_chunks: Vec::new(),
            below,
            above,
            meta,
        }
    }

    pub fn meta(&self) -> &TerrainChunkMeta {
        &self.meta
    }

    pub fn get_min_z(&self) -> i32 {
        self.z_offset
    }

    pub fn get_max_z(&self) -> i32 {
        self.z_offset + (self.sub_chunks.len() as u32 * SUB_CHUNK_HEIGHT) as i32
    }

    pub fn get_metrics(&self) -> ChonkMetrics {
        ChonkMetrics {
            chonks: 1,
            homogeneous: self
                .sub_chunks
                .iter()
                .filter(|s| match s {
                    SubChunk::Homogeneous(_) => true,
                    _ => false,
                })
                .count(),
            hash: self
                .sub_chunks
                .iter()
                .filter(|s| match s {
                    SubChunk::Hash(_, _) => true,
                    _ => false,
                })
                .count(),
            heterogeneous: self
                .sub_chunks
                .iter()
                .filter(|s| match s {
                    SubChunk::Heterogeneous(_) => true,
                    _ => false,
                })
                .count(),
        }
    }

    // Returns the index (in self.sub_chunks) of the SubChunk that contains
    // layer z; note that this index changes when more SubChunks are prepended
    fn sub_chunk_idx(&self, z: i32) -> usize {
        ((z - self.z_offset) / SUB_CHUNK_HEIGHT as i32) as usize
    }

    // Returns the z_offset of the sub_chunk that contains layer z
    fn sub_chunk_z_offset(&self, z: i32) -> i32 {
        let rem = (z - self.z_offset) % SUB_CHUNK_HEIGHT as i32;
        if rem < 0 {
            z - (rem + SUB_CHUNK_HEIGHT as i32)
        } else {
            z - rem
        }
    }
}

impl BaseVol for Chonk {
    type Vox = Block;
    type Err = ChonkError;
}

impl ReadVol for Chonk {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Block, ChonkError> {
        if pos.z < self.get_min_z() {
            // Below the terrain
            Ok(&self.below)
        } else if pos.z >= self.get_max_z() {
            // Above the terrain
            Ok(&self.above)
        } else {
            // Within the terrain

            let sub_chunk_idx = self.sub_chunk_idx(pos.z);

            match &self.sub_chunks[sub_chunk_idx] {
                // Can't fail
                SubChunk::Homogeneous(block) => Ok(block),
                SubChunk::Hash(cblock, map) => {
                    let rpos = pos
                        - Vec3::unit_z()
                            * (self.z_offset + sub_chunk_idx as i32 * SUB_CHUNK_HEIGHT as i32);

                    Ok(map.get(&rpos.map(|e| e as u8)).unwrap_or(cblock))
                }
                SubChunk::Heterogeneous(chunk) => {
                    let rpos = pos
                        - Vec3::unit_z()
                            * (self.z_offset + sub_chunk_idx as i32 * SUB_CHUNK_HEIGHT as i32);

                    chunk.get(rpos).map_err(ChonkError::ChunkError)
                }
            }
        }
    }
}

impl WriteVol for Chonk {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, block: Block) -> Result<(), ChonkError> {
        if pos.z < self.get_min_z() {
            // Prepend exactly sufficiently many SubChunks via Vec::splice
            let target_z_offset = self.sub_chunk_z_offset(pos.z);
            let c = SubChunk::Homogeneous(self.below);
            let n = (self.get_min_z() - target_z_offset) / SUB_CHUNK_HEIGHT as i32;
            self.sub_chunks
                .splice(0..0, std::iter::repeat(c).take(n as usize));
            self.z_offset = target_z_offset;
        } else if pos.z >= self.get_max_z() {
            // Append exactly sufficiently many SubChunks via Vec::extend
            let target_z_offset = self.sub_chunk_z_offset(pos.z);
            let c = SubChunk::Homogeneous(self.above);
            let n = (target_z_offset - self.get_max_z()) / SUB_CHUNK_HEIGHT as i32 + 1;
            self.sub_chunks
                .extend(std::iter::repeat(c).take(n as usize));
        }

        let sub_chunk_idx = self.sub_chunk_idx(pos.z);

        let rpos =
            pos - Vec3::unit_z() * (self.z_offset + sub_chunk_idx as i32 * SUB_CHUNK_HEIGHT as i32);

        match &mut self.sub_chunks[sub_chunk_idx] {
            // Can't fail
            SubChunk::Homogeneous(cblock) if block == *cblock => Ok(()),
            SubChunk::Homogeneous(cblock) => {
                let mut map = HashMap::default();
                map.insert(rpos.map(|e| e as u8), block);

                self.sub_chunks[sub_chunk_idx] = SubChunk::Hash(*cblock, map);
                Ok(())
            }
            SubChunk::Hash(cblock, map) if block == *cblock => {
                map.remove(&rpos.map(|e| e as u8));
                Ok(())
            }
            SubChunk::Hash(_cblock, map) if map.len() < SUB_CHUNK_HASH_LIMIT => {
                map.insert(rpos.map(|e| e as u8), block);
                Ok(())
            }
            SubChunk::Hash(cblock, map) => {
                let mut new_chunk = Chunk::filled(*cblock, ());
                for (map_pos, map_block) in map {
                    new_chunk
                        .set(map_pos.map(|e| i32::from(e)), *map_block)
                        .unwrap(); // Can't fail (I hope!)
                }

                new_chunk.set(rpos, block).unwrap(); // Can't fail (I hope)

                self.sub_chunks[sub_chunk_idx] = SubChunk::Heterogeneous(new_chunk);
                Ok(())
            }

            /*
            SubChunk::Homogeneous(cblock) => {
                let mut new_chunk = Chunk::filled(*cblock, ());

                new_chunk.set(rpos, block).unwrap(); // Can't fail (I hope!)

                self.sub_chunks[sub_chunk_idx] = SubChunk::Heterogeneous(new_chunk);
                Ok(())
            }
            */
            SubChunk::Heterogeneous(chunk) => {
                chunk.set(rpos, block).map_err(ChonkError::ChunkError)
            } //_ => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubChunk {
    Homogeneous(Block),
    Hash(Block, HashMap<Vec3<u8>, Block>),
    Heterogeneous(Chunk<Block, SubChunkSize, ()>),
}

impl SubChunk {
    pub fn filled(block: Block) -> Self {
        SubChunk::Homogeneous(block)
    }
}

#[derive(Debug)]
pub struct ChonkMetrics {
    chonks: usize,
    homogeneous: usize,
    hash: usize,
    heterogeneous: usize,
}

impl Default for ChonkMetrics {
    fn default() -> Self {
        ChonkMetrics {
            chonks: 0,
            homogeneous: 0,
            hash: 0,
            heterogeneous: 0,
        }
    }
}

impl Add for ChonkMetrics {
    type Output = Self;

    fn add(self, other: Self::Output) -> Self {
        Self::Output {
            chonks: self.chonks + other.chonks,
            homogeneous: self.homogeneous + other.homogeneous,
            hash: self.hash + other.hash,
            heterogeneous: self.heterogeneous + other.heterogeneous,
        }
    }
}
