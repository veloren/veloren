use crate::{
    block::{block_from_structure, ZCache},
    column::ColumnSample,
    index::IndexRef,
    land::Land,
    layer::spot::Spot,
    sim::{SimChunk, WorldSim},
    util::Grid,
};
use common::{
    terrain::{Block, Structure, TerrainChunk, TerrainChunkSize},
    vol::{ReadVol, RectVolSize, WriteVol},
};
use std::ops::Deref;
use vek::*;

#[derive(Copy, Clone)]
pub struct CanvasInfo<'a> {
    pub(crate) chunk_pos: Vec2<i32>,
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

    pub fn nearby_spots(&self) -> impl Iterator<Item = (Vec2<i32>, Spot, u32)> + '_ {
        (-1..2)
            .map(|x| (-1..2).map(move |y| Vec2::new(x, y)))
            .flatten()
            .filter_map(move |pos| {
                let pos = self.chunk_pos + pos;
                self.chunks.get(pos).and_then(|c| c.spot).map(|spot| {
                    let wpos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        e * sz as i32 + sz as i32 / 2
                    });
                    // TODO: Very dumb, not this.
                    let seed = pos.x as u32 | (pos.y as u32).wrapping_shl(16);

                    (wpos, spot, seed)
                })
            })
    }

    pub fn index(&self) -> IndexRef<'a> { self.index }

    pub fn chunk(&self) -> &'a SimChunk { self.chunk }

    pub fn chunks(&self) -> &'a WorldSim { self.chunks }

    pub fn land(&self) -> Land<'_> { Land::from_sim(self.chunks) }

    pub fn with_mock_canvas_info<A, F: for<'b> FnOnce(&CanvasInfo<'b>) -> A>(
        index: IndexRef<'a>,
        sim: &'a WorldSim,
        f: F,
    ) -> A {
        let zcache_grid = Grid::populate_from(Vec2::broadcast(1), |_| None);
        let sim_chunk = SimChunk {
            chaos: 0.0,
            alt: 0.0,
            basement: 0.0,
            water_alt: 0.0,
            downhill: None,
            flux: 0.0,
            temp: 0.0,
            humidity: 0.0,
            rockiness: 0.0,
            tree_density: 0.0,
            forest_kind: crate::all::ForestKind::Palm,
            spawn_rate: 0.0,
            river: Default::default(),
            surface_veg: 0.0,
            sites: Vec::new(),
            place: None,
            path: Default::default(),
            cave: Default::default(),
            cliff_height: 0.0,
            contains_waypoint: false,
        };
        f(&CanvasInfo {
            wpos: Vec2::broadcast(0),
            column_grid: &zcache_grid,
            column_grid_border: 0,
            chunks: sim,
            index,
            chunk: &sim_chunk,
        })
    }
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
            .unwrap_or_else(Block::empty)
    }

    pub fn set(&mut self, pos: Vec3<i32>, block: Block) {
        let _ = self.chunk.set(pos - self.wpos(), block);
    }

    pub fn map(&mut self, pos: Vec3<i32>, f: impl FnOnce(Block) -> Block) {
        let _ = self.chunk.map(pos - self.wpos(), f);
    }

    pub fn foreach_col_area(
        &mut self,
        aabr: Aabr<i32>,
        mut f: impl FnMut(&mut Self, Vec2<i32>, &ColumnSample),
    ) {
        let chunk_aabr = Aabr {
            min: self.wpos(),
            max: self.wpos() + Vec2::from(self.area().size().map(|e| e as i32)),
        };

        for y in chunk_aabr.min.y.max(aabr.min.y)..chunk_aabr.max.y.min(aabr.max.y) {
            for x in chunk_aabr.min.x.max(aabr.min.x)..chunk_aabr.max.x.min(aabr.max.x) {
                let wpos2d = Vec2::new(x, y);
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

    /// Execute an operation upon each column in this canvas.
    pub fn foreach_col(&mut self, f: impl FnMut(&mut Self, Vec2<i32>, &ColumnSample)) {
        self.foreach_col_area(
            Aabr {
                min: Vec2::broadcast(i32::MIN),
                max: Vec2::broadcast(i32::MAX),
            },
            f,
        );
    }

    /// Blit a structure on to the canvas at the given position.
    ///
    /// Note that this function should be called with identitical parameters by
    /// all chunks within the bounds of the structure to avoid cut-offs
    /// occurring at chunk borders. Deterministic RNG is advised!
    pub fn blit_structure(&mut self, origin: Vec3<i32>, structure: &Structure, seed: u32) {
        let aabr = Aabr {
            min: origin.xy() + structure.get_bounds().min.xy(),
            max: origin.xy() + structure.get_bounds().max.xy(),
        };
        let info = self.info();
        self.foreach_col_area(aabr, |canvas, wpos2d, col| {
            for z in structure.get_bounds().min.z..structure.get_bounds().max.z {
                if let Ok(sblock) = structure.get((wpos2d - origin.xy()).with_z(z)) {
                    let _ = canvas.map(wpos2d.with_z(origin.z + z), |block| {
                        if let Some(block) = block_from_structure(
                            info.index,
                            *sblock,
                            wpos2d.with_z(origin.z + z),
                            wpos2d - origin.xy(),
                            seed,
                            col,
                            |sprite| block.with_sprite(sprite),
                        ) {
                            block
                        } else {
                            block
                        }
                    });
                }
            }
        });
    }
}

impl<'a> Deref for Canvas<'a> {
    type Target = CanvasInfo<'a>;

    fn deref(&self) -> &Self::Target { &self.info }
}
