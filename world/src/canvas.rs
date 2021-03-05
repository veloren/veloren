use crate::{
    block::ZCache,
    column::ColumnSample,
    index::IndexRef,
    land::Land,
    sim::{SimChunk, WorldSim},
    util::Grid,
};
use common::{
    terrain::{Block, TerrainChunk, TerrainChunkSize},
    vol::{ReadVol, RectVolSize, WriteVol},
};
use std::ops::Deref;
use vek::*;

#[derive(Copy, Clone)]
pub struct CanvasInfo<'a> {
    pub(crate) wpos: Vec2<i32>,
    pub(crate) column_grid: &'a Grid<Option<ZCache<'a>>>,
    pub(crate) column_grid_border: i32,
    pub(crate) chunks: &'a WorldSim,
    pub(crate) index: IndexRef<'a>,
    pub(crate) chunk: &'a SimChunk,
}

impl<'a> CanvasInfo<'a> {
    pub fn wpos(&self) -> Vec2<i32> { self.wpos }

    pub fn area(&self) -> Aabr<i32> {
        Rect::from((
            self.wpos(),
            Extent2::from(TerrainChunkSize::RECT_SIZE.map(|e| e as i32)),
        ))
        .into()
    }

    pub fn col(&self, pos: Vec2<i32>) -> Option<&'a ColumnSample> {
        self.column_grid
            .get(self.column_grid_border + pos - self.wpos())
            .map(Option::as_ref)
            .flatten()
            .map(|zc| &zc.sample)
    }

    pub fn index(&self) -> IndexRef<'a> { self.index }

    pub fn chunk(&self) -> &'a SimChunk { self.chunk }

    pub fn chunks(&self) -> &'a WorldSim { self.chunks }

    pub fn land(&self) -> Land<'_> { Land::from_sim(self.chunks) }
}

pub struct Canvas<'a> {
    pub(crate) info: CanvasInfo<'a>,
    pub(crate) chunk: &'a mut TerrainChunk,
}

impl<'a> Canvas<'a> {
    /// The borrow checker complains at immutable features of canvas (column
    /// sampling, etc.) being used at the same time as mutable features
    /// (writing blocks). To avoid this, this method extracts the
    /// inner `CanvasInfo` such that it may be used independently.
    pub fn info(&mut self) -> CanvasInfo<'a> { self.info }

    pub fn get(&mut self, pos: Vec3<i32>) -> Block {
        self.chunk
            .get(pos - self.wpos())
            .ok()
            .copied()
            .unwrap_or(Block::empty())
    }

    pub fn set(&mut self, pos: Vec3<i32>, block: Block) {
        let _ = self.chunk.set(pos - self.wpos(), block);
    }

    pub fn map(&mut self, pos: Vec3<i32>, f: impl FnOnce(Block) -> Block) {
        let _ = self.chunk.map(pos - self.wpos(), f);
    }

    /// Execute an operation upon each column in this canvas.
    pub fn foreach_col(&mut self, mut f: impl FnMut(&mut Self, Vec2<i32>, &ColumnSample)) {
        for y in 0..self.area().size().h as i32 {
            for x in 0..self.area().size().w as i32 {
                let wpos2d = self.wpos() + Vec2::new(x, y);
                let info = self.info;
                let col = if let Some(col) = info.col(wpos2d) {
                    col
                } else {
                    return;
                };
                f(self, wpos2d, col);
            }
        }
    }
}

impl<'a> Deref for Canvas<'a> {
    type Target = CanvasInfo<'a>;

    fn deref(&self) -> &Self::Target { &self.info }
}
