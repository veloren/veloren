use vek::*;
use common::{
    terrain::{TerrainChunk, Block, TerrainChunkSize},
    vol::{ReadVol, WriteVol, RectVolSize},
};
use crate::{
    block::ZCache,
    util::Grid,
    column::ColumnSample,
    index::IndexRef,
};

pub struct CanvasInfo<'a> {
    pub(crate) wpos: Vec2<i32>,
    pub(crate) column_grid: &'a Grid<Option<ZCache<'a>>>,
    pub(crate) column_grid_border: i32,
    pub(crate) index: IndexRef<'a>,
}

impl<'a> CanvasInfo<'a> {
    pub fn wpos(&self) -> Vec2<i32> {
        self.wpos
    }

    pub fn area(&self) -> Aabr<i32> {
        Rect::from((self.wpos(), Extent2::from(TerrainChunkSize::RECT_SIZE.map(|e| e as i32)))).into()
    }

    pub fn col(&self, pos: Vec2<i32>) -> Option<&ColumnSample> {
        self.column_grid
            .get(self.column_grid_border + pos - self.wpos())
            .map(Option::as_ref)
            .flatten()
            .map(|zc| &zc.sample)
    }

    pub fn index(&self) -> IndexRef {
        self.index
    }
}

pub struct Canvas<'a> {
    pub(crate) wpos: Vec2<i32>,
    pub(crate) chunk: &'a mut TerrainChunk,
}

impl<'a> Canvas<'a> {
    pub fn wpos(&self) -> Vec2<i32> {
        self.wpos
    }

    pub fn area(&self) -> Aabr<i32> {
        Rect::from((self.wpos(), Extent2::from(TerrainChunkSize::RECT_SIZE.map(|e| e as i32)))).into()
    }

    pub fn get(&mut self, pos: Vec3<i32>) -> Option<Block> {
        self.chunk.get(pos - self.wpos()).ok().copied()
    }

    pub fn set(&mut self, pos: Vec3<i32>, block: Block) {
        let _ = self.chunk.set(pos - self.wpos(), block);
    }
}
