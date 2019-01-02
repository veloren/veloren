// Standard
use std::collections::HashMap;

// Library
use vek::*;

// Crate
use crate::{
    vol::{
        BaseVol,
        ReadVol,
        WriteVol,
        VolSize,
    },
    volumes::chunk::{Chunk, ChunkErr},
};

pub enum VolMapErr {
    NoSuchChunk,
    ChunkErr(ChunkErr),
}

// V = Voxel
// S = Size (replace with a const when const generics is a thing)
// M = Chunk metadata
pub struct VolMap<V, S: VolSize, M> {
    chunks: HashMap<Vec3<i32>, Chunk<V, S, M>>,
}

impl<V, S: VolSize, M> VolMap<V, S, M> {
    #[inline(always)]
    fn chunk_key(pos: Vec3<i32>) -> Vec3<i32> {
        pos.map2(S::SIZE, |e, sz| e.div_euclid(sz as i32))
    }

    #[inline(always)]
    fn chunk_offs(pos: Vec3<i32>) -> Vec3<i32> {
        pos.map2(S::SIZE, |e, sz| e.rem_euclid(sz as i32))
    }
}

impl<V, S: VolSize, M> BaseVol for VolMap<V, S, M> {
    type Vox = V;
    type Err = VolMapErr;
}

impl<V, S: VolSize, M> ReadVol for VolMap<V, S, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&V, VolMapErr> {
        let ck = Self::chunk_key(pos);
        self.chunks.get(&ck)
            .ok_or(VolMapErr::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                chunk.get(co).map_err(|err| VolMapErr::ChunkErr(err))
            })
    }
}

impl<V, S: VolSize, M> WriteVol for VolMap<V, S, M> {
    #[inline(always)]
    fn set(&mut self, pos: Vec3<i32>, vox: V) -> Result<(), VolMapErr> {
        let ck = Self::chunk_key(pos);
        self.chunks.get_mut(&ck)
            .ok_or(VolMapErr::NoSuchChunk)
            .and_then(|chunk| {
                let co = Self::chunk_offs(pos);
                chunk.set(co, vox).map_err(|err| VolMapErr::ChunkErr(err))
            })
    }
}

impl<V, S: VolSize, M> VolMap<V, S, M> {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }
}
