use super::*;
use ::world::util::Grid;

pub struct Chunks {
    chunks: Grid<Chunk>,
    pub chunks_to_load: Vec<Vec2<i32>>,
    pub chunks_to_unload: Vec<Vec2<i32>>,
}

impl Chunks {
    pub fn new(size: Vec2<u32>) -> Self {
        Chunks {
            chunks: Grid::populate_from(size.map(|e| e as i32), |_| Chunk { is_loaded: false }),
            chunks_to_load: Vec::new(),
            chunks_to_unload: Vec::new(),
        }
    }

    pub fn chunk(&self, key: Vec2<i32>) -> Option<&Chunk> { self.chunks.get(key) }

    pub fn size(&self) -> Vec2<u32> { self.chunks.size().map(|e| e as u32) }

    pub fn chunk_mut(&mut self, key: Vec2<i32>) -> Option<&mut Chunk> { self.chunks.get_mut(key) }

    pub fn chunk_at(&self, pos: Vec2<f32>) -> Option<&Chunk> {
        self.chunks.get(pos.map2(TerrainChunk::RECT_SIZE, |e, sz| {
            (e.floor() as i32).div_euclid(sz as i32)
        }))
    }
}

pub struct Chunk {
    pub is_loaded: bool,
}
