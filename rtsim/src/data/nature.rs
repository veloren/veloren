use serde::{Serialize, Deserialize};
use enum_map::EnumMap;
use common::{
    grid::Grid,
    rtsim::ChunkResource,
};
use world::World;
use vek::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Nature {
    chunks: Grid<Chunk>,
}

impl Nature {
    pub fn generate(world: &World) -> Self {
        Self {
            chunks: Grid::populate_from(
                world.sim().get_size().map(|e| e as i32),
                |pos| Chunk {
                    res: EnumMap::<_, f32>::default().map(|_, _| 1.0),
                },
            ),
        }
    }

    // TODO: Clean up this API a bit
    pub fn get_chunk_resources(&self, key: Vec2<i32>) -> EnumMap<ChunkResource, f32> {
        self.chunks
            .get(key)
            .map(|c| c.res)
            .unwrap_or_default()
    }
    pub fn set_chunk_resources(&mut self, key: Vec2<i32>, res: EnumMap<ChunkResource, f32>) {
        if let Some(chunk) = self.chunks.get_mut(key) {
            chunk.res = res;
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Chunk {
    res: EnumMap<ChunkResource, f32>,
}
