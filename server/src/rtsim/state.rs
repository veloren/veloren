use vek::*;
use world::util::Grid;

pub struct SimState {
    chunks: Grid<Chunk>,
}

impl SimState {
    pub fn new(world_chunk_size: Vec2<u32>) -> Self {
        Self {
            chunks: Grid::populate_from(world_chunk_size.map(|e| e as i32), |_| Chunk {
                is_loaded: false,
            }),
        }
    }
}

pub struct Chunk {
    is_loaded: bool,
}
