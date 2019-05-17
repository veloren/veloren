use vek::*;
use serde_derive::{Deserialize, Serialize};
use crate::{
    vol::{
        BaseVol,
        ReadVol,
        WriteVol,
        VolSize,
    },
    volumes::chunk::{Chunk, ChunkErr},
};
use super::{
    block::Block,
    TerrainChunkSize,
    TerrainChunkMeta,
};

#[derive(Debug)]
pub enum ChonkError {
    ChunkError(ChunkErr),
    OutOfBounds,
}

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

    fn sub_chunk_idx(&self, z: i32) -> usize {
        ((z - self.z_offset) as u32 / TerrainChunkSize::SIZE.z as u32) as usize
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
        } else if pos.z >= self.z_offset + TerrainChunkSize::SIZE.z as i32 * self.sub_chunks.len() as i32 {
            // Above the terrain
            Ok(&self.above)
        } else {
            // Within the terrain

            let sub_chunk_idx = self.sub_chunk_idx(pos.z);

            match &self.sub_chunks[sub_chunk_idx] { // Can't fail
                SubChunk::Homogeneous(block) => Ok(block),
                SubChunk::Heterogeneous(chunk) => {
                    let rpos = pos - Vec3::unit_z() * (
                        self.z_offset +
                        sub_chunk_idx as i32 * TerrainChunkSize::SIZE.z as i32
                    );
                    chunk
                        .get(rpos)
                        .map_err(|err| ChonkError::ChunkError(err))
                },
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

            let rpos = pos - Vec3::unit_z() * (
                self.z_offset +
                sub_chunk_idx as i32 * TerrainChunkSize::SIZE.z as i32
            );

            match &mut self.sub_chunks[sub_chunk_idx] { // Can't fail
                SubChunk::Homogeneous(cblock) if *cblock == block => Ok(()),
                SubChunk::Homogeneous(cblock) => {
                    let mut new_chunk = Chunk::filled(*cblock, ());
                    match new_chunk
                        .set(rpos, block)
                        .map_err(|err| {
                            println!("Error!! {:?}", rpos);
                            ChonkError::ChunkError(err)
                        })
                    {
                        Ok(()) => {
                            self.sub_chunks[sub_chunk_idx] = SubChunk::Heterogeneous(new_chunk);
                            Ok(())
                        },
                        Err(err) => Err(err),
                    }

                },
                SubChunk::Heterogeneous(chunk) => chunk
                    .set(rpos, block)
                    .map_err(|err| ChonkError::ChunkError(err)),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubChunk {
    Homogeneous(Block),
    Heterogeneous(Chunk<Block, TerrainChunkSize, ()>),
}

impl SubChunk {
    pub fn filled(block: Block) -> Self {
        SubChunk::Homogeneous(block)
    }
}
