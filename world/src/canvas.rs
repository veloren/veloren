use crate::{
    block::{block_from_structure, ZCache},
    column::{ColumnGen, ColumnSample},
    index::IndexRef,
    land::Land,
    layer::spot::Spot,
    sim::{SimChunk, WorldSim},
    util::{seed_expan, Grid, Sampler},
};
use common::{
    calendar::Calendar,
    generation::EntityInfo,
    terrain::{
        structure::StructureBlock, Block, BlockKind, SpriteCfg, Structure, TerrainChunk,
        TerrainChunkSize,
    },
    vol::{ReadVol, RectVolSize, WriteVol},
};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use std::{borrow::Cow, ops::Deref};
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
    pub(crate) calendar: Option<&'a Calendar>,
}

impl<'a> CanvasInfo<'a> {
    pub fn calendar(&self) -> Option<&'a Calendar> { self.calendar }

    pub fn wpos(&self) -> Vec2<i32> { self.wpos }

    pub fn area(&self) -> Aabr<i32> {
        Rect::from((
            self.wpos(),
            Extent2::from(TerrainChunkSize::RECT_SIZE.map(|e| e as i32)),
        ))
        .into()
    }

    pub fn col(&self, wpos: Vec2<i32>) -> Option<&'a ColumnSample> {
        self.column_grid
            .get(self.column_grid_border + wpos - self.wpos())
            .and_then(Option::as_ref)
            .map(|zc| &zc.sample)
    }

    /// Attempt to get the data for the given column, generating it if we don't
    /// have it.
    ///
    /// This function does not (currently) cache generated columns.
    pub fn col_or_gen(&self, wpos: Vec2<i32>) -> Option<Cow<'a, ColumnSample>> {
        self.col(wpos).map(Cow::Borrowed).or_else(|| {
            Some(Cow::Owned(ColumnGen::new(self.chunks()).get((
                wpos,
                self.index(),
                self.calendar,
            ))?))
        })
    }

    /// Find all spots within range of this canvas's chunk. Returns `(wpos,
    /// spot, seed)`.
    pub fn nearby_spots(&self) -> impl Iterator<Item = (Vec2<i32>, Spot, u32)> + '_ {
        (-1..2)
            .flat_map(|x| (-1..2).map(move |y| Vec2::new(x, y)))
            .filter_map(move |pos| {
                let pos = self.chunk_pos + pos;
                self.chunks.get(pos).and_then(|c| c.spot).map(|spot| {
                    let wpos = pos.map2(TerrainChunkSize::RECT_SIZE, |e, sz| {
                        e * sz as i32 + sz as i32 / 2
                    });
                    // TODO: Very dumb, not this.
                    let seed = pos.x as u32 | (pos.y as u32).wrapping_shl(16);

                    (wpos, spot, seed ^ 0xA801D82E)
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
            poi: None,
            path: Default::default(),
            cave: Default::default(),
            cliff_height: 0.0,
            contains_waypoint: false,
            spot: None,
        };
        f(&CanvasInfo {
            chunk_pos: Vec2::zero(),
            wpos: Vec2::zero(),
            column_grid: &zcache_grid,
            column_grid_border: 0,
            chunks: sim,
            index,
            chunk: &sim_chunk,
            calendar: None,
        })
    }
}

pub struct Canvas<'a> {
    pub(crate) info: CanvasInfo<'a>,
    pub(crate) chunk: &'a mut TerrainChunk,
    pub(crate) entities: Vec<EntityInfo>,
}

impl<'a> Canvas<'a> {
    /// The borrow checker complains at immutable features of canvas (column
    /// sampling, etc.) being used at the same time as mutable features
    /// (writing blocks). To avoid this, this method extracts the
    /// inner `CanvasInfo` such that it may be used independently.
    pub fn info(&mut self) -> CanvasInfo<'a> { self.info }

    pub fn get(&self, pos: Vec3<i32>) -> Block {
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

    pub fn set_sprite_cfg(&mut self, pos: Vec3<i32>, sprite_cfg: SpriteCfg) {
        let rpos = pos - self.wpos();
        self.chunk.meta_mut().set_sprite_cfg_at(rpos, sprite_cfg);
    }

    pub fn foreach_col_area(
        &mut self,
        aabr: Aabr<i32>,
        mut f: impl FnMut(&mut Self, Vec2<i32>, &ColumnSample),
    ) {
        let chunk_aabr = Aabr {
            min: self.wpos(),
            max: self.wpos() + Vec2::from(self.area().size()),
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
    /// Note that this function should be called with identical parameters by
    /// all chunks within the bounds of the structure to avoid cut-offs
    /// occurring at chunk borders. Deterministic RNG is advised!
    pub fn blit_structure(
        &mut self,
        origin: Vec3<i32>,
        structure: &Structure,
        seed: u32,
        units: Vec2<Vec2<i32>>,
        with_snow: bool,
    ) {
        let mut entities: Vec<(Vec3<f32>, String)> = Vec::new();
        let mut rng = ChaChaRng::from_seed(seed_expan::rng_state(seed));
        let info = self.info();
        self.foreach_col(|canvas, wpos2d, col| {
            let rpos2d = wpos2d - origin.xy();
            let rpos2d = units.x * rpos2d.x + units.y * rpos2d.y;

            let mut above = true;
            for z in (structure.get_bounds().min.z..structure.get_bounds().max.z).rev() {
                if let Ok(sblock) = structure.get(rpos2d.with_z(z)) {
                    let mut add_snow = false;
                    let mut new_sprite_cfg = None;
                    let wpos = wpos2d.with_z(origin.z + z);
                    canvas.map(wpos, |block| {
                        if let Some((new_block, sprite_cfg)) = block_from_structure(
                            info.index,
                            sblock,
                            wpos2d.with_z(origin.z + z),
                            origin.xy(),
                            seed,
                            col,
                            |sprite| block.with_sprite(sprite),
                            info.calendar,
                            &units,
                        ) {
                            new_sprite_cfg = sprite_cfg;
                            if !new_block.is_air() {
                                if with_snow && col.snow_cover && above {
                                    add_snow = true;
                                }
                                above = false;
                            }
                            new_block
                        } else {
                            block
                        }
                    });

                    // Add sprite configuration for those that need it
                    if let Some(sprite_cfg) = new_sprite_cfg {
                        canvas.set_sprite_cfg(wpos, sprite_cfg);
                    }

                    // Spawn NPCs at the pos of this block
                    if let StructureBlock::EntitySpawner(spec, spawn_chance) = sblock {
                        if rng.gen::<f32>() < *spawn_chance {
                            entities.push((
                                wpos2d.with_z(origin.z + z).map(|e| e as f32)
                                    + Vec3::new(0.5, 0.5, 0.0),
                                spec.clone(),
                            ));
                        }
                    }

                    if add_snow {
                        canvas.set(
                            wpos2d.with_z(origin.z + z + 1),
                            Block::new(BlockKind::Snow, Rgb::new(210, 210, 255)),
                        );
                    }
                }
            }
        });
        for (pos, spec) in entities.drain(..) {
            self.spawn(EntityInfo::at(pos).with_asset_expect(&spec, &mut rng));
        }
    }

    pub fn find_spawn_pos(&self, wpos: Vec3<i32>) -> Option<Vec3<i32>> {
        let height = 2;
        let search_dist: i32 = 8;

        (1..search_dist * 2 + 1)
            .rev()
            .map(|z| wpos.z + if z % 2 != 0 { z / 2 } else { -(z / 2) })
            .find(|&z| {
                self.get(wpos.xy().with_z(z - 1)).is_solid()
                    && (0..height).all(|z_offs| self.get(wpos.xy().with_z(z + z_offs)).is_fluid())
            })
            .map(|z| wpos.xy().with_z(z))
    }

    pub fn spawn(&mut self, entity: EntityInfo) { self.entities.push(entity); }
}

impl<'a> Deref for Canvas<'a> {
    type Target = CanvasInfo<'a>;

    fn deref(&self) -> &Self::Target { &self.info }
}
