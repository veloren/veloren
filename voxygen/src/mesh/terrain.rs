// Library
use vek::*;

// Project
use common::{
    terrain::{Block, TerrainMap, TerrainChunkSize, TerrainChunkMeta},
    vol::{BaseVol, ReadVol, WriteVol, SizedVol, VolSize, Vox},
    volumes::chunk::Chunk,
};
use std::collections::HashMap;
use std::sync::Arc;

// Crate
use crate::{
    mesh::{vol, Meshable},
    render::{self, Mesh, Quad, TerrainPipeline},
};

type TerrainVertex = <TerrainPipeline as render::Pipeline>::Vertex;

#[derive(Debug)]
pub enum CombiErr {
    NoSuchChunk,
}

// Here S stands for the size of the individual chunks
pub struct Combi<V: Vox, S: VolSize, M> {
    aabb: Aabb<i32>,
    chunks: HashMap<Vec3<i32>, Arc<Chunk<V, S, M>>>,
}

impl Combi<Block, TerrainChunkSize, TerrainChunkMeta> {
    pub fn from_chunks(aabb: Aabb<i32>, chunks: HashMap<Vec3<i32>, Arc<Chunk<Block, TerrainChunkSize, TerrainChunkMeta>>>) -> Self {
        Self {
            aabb,
            chunks: chunks,
        }
    }
}

impl<V: Vox, S: VolSize, M> BaseVol for Combi<V, S, M> {
    type Vox = V;
    type Err = CombiErr;
}

impl<V: Vox, S: VolSize, M> SizedVol for Combi<V, S, M> {
    #[inline(always)]
    fn get_size(&self) -> Vec3<u32> {
        (self.aabb.max - self.aabb.min).map(|i| i as u32)
    }
}

impl<V: Vox + Clone, S: VolSize, M> ReadVol for Combi<V, S, M> {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<V, CombiErr> {
        let chunk = &self.chunks.get(&TerrainMap::chunk_key(pos)).ok_or(CombiErr::NoSuchChunk)?;
        Ok(chunk.get(TerrainMap::chunk_offs(pos)).map_err(|_| CombiErr::NoSuchChunk)?)
    }
}

impl Meshable for Combi<Block, TerrainChunkSize, TerrainChunkMeta> {
    type Pipeline = TerrainPipeline;
    type Supplement = ();

    fn generate_mesh(&self, _: Self::Supplement) -> Mesh<Self::Pipeline> {
        //println!("GENERATING");
        let mut mesh = Mesh::new();

        for x in self.aabb.min.x..=self.aabb.max.x {
            for y in self.aabb.min.y..=self.aabb.max.y {
                for pos in (self.aabb.min.z..=self.aabb.max.z)
                    .map(|z| Vec3::new(x, y, z))
                    //TODO: Figure out what this does
                    //.filter(|pos| pos.map(|e| e >= 1).reduce_and())
                    //.filter(|pos| {
                    //    pos.map2(self.get_size(), |e, sz| e < sz as i32 - 1)
                    //        .reduce_and()
                    //})
                {
                    //eprint!(".");
                    let offs = pos.map(|e| e as f32 - 1.0);

                    if let Some(col) = self.get(pos).ok().and_then(|vox| vox.get_color()) {
                        let col = col.map(|e| e as f32 / 255.0);
                        vol::push_vox_verts(&mut mesh, self, pos, offs, col, TerrainVertex::new);
                        //eprintln!("{:?} WORKED!", pos);
                    }

                }
            }
        }

        mesh
    }
}
