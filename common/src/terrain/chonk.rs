use super::{block::Block, TerrainChunkMeta, TerrainChunkSize};
use crate::{
    vol::{BaseVol, ReadVol, VolSize, WriteVol},
    volumes::chunk::{Chunk, ChunkErr},
};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use vek::*;

#[derive(Debug)]
pub enum ChonkError {
    ChunkError(ChunkErr),
    OutOfBounds,
}

const SUB_CHUNK_HEIGHT: u32 = 16;

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

    pub fn get_z_min(&self) -> i32 {
        self.z_offset
    }

    pub fn get_z_max(&self) -> i32 {
        self.z_offset + (self.sub_chunks.len() as u32 * SUB_CHUNK_HEIGHT) as i32
    }

    fn sub_chunk_idx(&self, z: i32) -> usize {
        ((z - self.z_offset) as u32 / SUB_CHUNK_HEIGHT as u32) as usize
    }
}

impl BaseVol for Chonk {
    type Vox = Block;
    type Err = ChonkError;
}

impl ReadVol for Chonk {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Block, ChonkError> {
        if pos.z < self.z_offset {
            // Below the terrain
            Ok(&self.below)
        } else if pos.z >= self.z_offset + SUB_CHUNK_HEIGHT as i32 * self.sub_chunks.len() as i32 {
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

                    Ok(map.get(&rpos).unwrap_or(cblock))
                }
                SubChunk::Heterogeneous(chunk) => {
                    let rpos = pos
                        - Vec3::unit_z()
                            * (self.z_offset + sub_chunk_idx as i32 * SUB_CHUNK_HEIGHT as i32);

                    chunk.get(rpos).map_err(|err| ChonkError::ChunkError(err))
                }
            }
        }
    }
}

impl WriteVol for Chonk {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, block: Block) -> Result<(), ChonkError> {
        if pos.z < self.z_offset {
            Err(ChonkError::OutOfBounds)
        } else {
            let sub_chunk_idx = self.sub_chunk_idx(pos.z);

            while self.sub_chunks.get(sub_chunk_idx).is_none() {
                self.sub_chunks.push(SubChunk::Homogeneous(self.above));
            }

            let rpos = pos
                - Vec3::unit_z() * (self.z_offset + sub_chunk_idx as i32 * SUB_CHUNK_HEIGHT as i32);

            match &mut self.sub_chunks[sub_chunk_idx] {
                // Can't fail
                SubChunk::Homogeneous(cblock) if *cblock == block => Ok(()),
                SubChunk::Homogeneous(cblock) => {
                    let mut map = HashMap::new();
                    map.insert(rpos, block);

                    self.sub_chunks[sub_chunk_idx] = SubChunk::Hash(*cblock, map);
                    Ok(())
                }
                SubChunk::Hash(cblock, map) if map.len() < 1024 => {
                    map.insert(rpos, block);
                    Ok(())
                }
                SubChunk::Hash(cblock, map) => {
                    let mut new_chunk = Chunk::filled(*cblock, ());
                    new_chunk.set(rpos, block).unwrap(); // Can't fail (I hope)

                    for (map_pos, map_block) in map {
                        new_chunk.set(*map_pos, *map_block).unwrap(); // Can't fail (I hope!)
                    }

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
                SubChunk::Heterogeneous(chunk) => chunk
                    .set(rpos, block)
                    .map_err(|err| ChonkError::ChunkError(err)),
                //_ => unimplemented!(),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubChunk {
    Homogeneous(Block),
    Hash(Block, HashMap<Vec3<i32>, Block>),
    Heterogeneous(Chunk<Block, TerrainChunkSize, ()>),
}

impl SubChunk {
    pub fn filled(block: Block) -> Self {
        SubChunk::Homogeneous(block)
    }
}
