#![allow(incomplete_features)]
#![allow(
    clippy::option_map_unit_fn,
    clippy::blocks_in_if_conditions,
    clippy::identity_op
)]
#![allow(clippy::branches_sharing_code)] // TODO: evaluate
#![deny(clippy::clone_on_ref_ptr)]
#![feature(option_zip)]

mod all;
mod block;
pub mod canvas;
pub mod civ;
mod column;
pub mod config;
pub mod index;
pub mod land;
pub mod layer;
pub mod pathfinding;
pub mod sim;
pub mod sim2;
pub mod site;
pub mod site2;
pub mod util;

// Reexports
pub use crate::{
    canvas::{Canvas, CanvasInfo},
    config::{Features, CONFIG},
    land::Land,
    layer::PathLocals,
};
pub use block::BlockGen;
pub use column::ColumnSample;
pub use common::terrain::site::{DungeonKindMeta, SettlementKindMeta};
pub use index::{IndexOwned, IndexRef};

use crate::{
    column::ColumnGen,
    index::Index,
    layer::spot::Spot,
    site::{SiteKind, SpawnRules},
    util::{Grid, Sampler},
};
use common::{
    assets,
    calendar::Calendar,
    generation::{ChunkSupplement, EntityInfo},
    lod,
    resources::TimeOfDay,
    terrain::{
        Block, BlockKind, SpriteKind, TerrainChunk, TerrainChunkMeta, TerrainChunkSize, TerrainGrid,
    },
    vol::{ReadVol, RectVolSize, WriteVol},
};
use common_net::msg::{world_msg, WorldMapMsg};
use rand::{prelude::*, Rng};
use rand_chacha::ChaCha8Rng;
use rayon::iter::ParallelIterator;
use serde::Deserialize;
use std::time::Duration;
use vek::*;

#[cfg(all(feature = "be-dyn-lib", feature = "use-dyn-lib"))]
compile_error!("Can't use both \"be-dyn-lib\" and \"use-dyn-lib\" features at once");

#[cfg(feature = "use-dyn-lib")]
use {common_dynlib::LoadedLib, lazy_static::lazy_static, std::sync::Arc, std::sync::Mutex};

#[cfg(feature = "use-dyn-lib")]
lazy_static! {
    pub static ref LIB: Arc<Mutex<Option<LoadedLib>>> =
        common_dynlib::init("veloren-world", "world");
}

#[cfg(feature = "use-dyn-lib")]
pub fn init() { lazy_static::initialize(&LIB); }

#[derive(Debug)]
pub enum Error {
    Other(String),
}

pub struct World {
    sim: sim::WorldSim,
    civs: civ::Civs,
}

#[derive(Deserialize)]
pub struct Colors {
    pub deep_stone_color: (u8, u8, u8),
    pub block: block::Colors,
    pub column: column::Colors,
    pub layer: layer::Colors,
    pub site: site::Colors,
}

impl assets::Asset for Colors {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl World {
    pub fn generate(
        seed: u32,
        opts: sim::WorldOpts,
        threadpool: &rayon::ThreadPool,
    ) -> (Self, IndexOwned) {
        // NOTE: Generating index first in order to quickly fail if the color manifest
        // is broken.
        threadpool.install(|| {
            let mut index = Index::new(seed);

            let mut sim = sim::WorldSim::generate(seed, opts, threadpool);

            let civs = civ::Civs::generate(seed, &mut sim, &mut index);

            sim2::simulate(&mut index, &mut sim);

            Spot::generate(&mut sim);

            (Self { sim, civs }, IndexOwned::new(index))
        })
    }

    pub fn sim(&self) -> &sim::WorldSim { &self.sim }

    pub fn civs(&self) -> &civ::Civs { &self.civs }

    pub fn tick(&self, _dt: Duration) {
        // TODO
    }

    pub fn get_map_data(&self, index: IndexRef, threadpool: &rayon::ThreadPool) -> WorldMapMsg {
        threadpool.install(|| {
            // we need these numbers to create unique ids for cave ends
            let num_sites = self.civs().sites().count() as u64;
            let num_caves = self.civs().caves.values().count() as u64;
            WorldMapMsg {
                pois: self.civs().pois.iter().map(|(_, poi)| {
                    world_msg::PoiInfo {
                        name: poi.name.clone(),
                        kind: match &poi.kind {
                            civ::PoiKind::Peak(alt) => world_msg::PoiKind::Peak(*alt),
                            civ::PoiKind::Biome(size) => world_msg::PoiKind::Lake(*size),
                        },
                        wpos: poi.loc * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
                    }
                }).collect(),
                sites: self
                    .civs()
                    .sites
                    .iter()
                    .map(|(_, site)| {
                        world_msg::SiteInfo {
                            id: site.site_tmp.map(|i| i.id()).unwrap_or_default(),
                            name: site.site_tmp.map(|id| index.sites[id].name().to_string()),
                            // TODO: Probably unify these, at some point
                            kind: match &site.kind {
                                civ::SiteKind::Settlement | civ::SiteKind::Refactor | civ::SiteKind::CliffTown | civ::SiteKind::SavannahPit | civ::SiteKind::DesertCity => world_msg::SiteKind::Town,
                                civ::SiteKind::Dungeon => world_msg::SiteKind::Dungeon {
                                    difficulty: match site.site_tmp.map(|id| &index.sites[id].kind) {
                                        Some(SiteKind::Dungeon(d)) => d.dungeon_difficulty().unwrap_or(0),
                                        _ => 0,
                                    },
                                },
                                civ::SiteKind::Castle => world_msg::SiteKind::Castle,
                                civ::SiteKind::Tree | civ::SiteKind::GiantTree => world_msg::SiteKind::Tree,
                                // TODO: Maybe change?
                                civ::SiteKind::Gnarling => world_msg::SiteKind::Gnarling,
                                civ::SiteKind::ChapelSite => world_msg::SiteKind::ChapelSite,
                                civ::SiteKind::Citadel => world_msg::SiteKind::Castle,
                                civ::SiteKind::Bridge(_, _) => world_msg::SiteKind::Bridge,
                            },
                            wpos: site.center * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
                        }
                    })
                    .chain(
                        self.civs()
                            .caves
                            .iter()
                            .flat_map(|(id, info)| {
                                // separate the two locations, combine with name
                                std::iter::once((id.id() + num_sites, info.name.clone(), info.location.0))
                                    // unfortunately we have to introduce a fake id (as it gets stored in a map in the client)
                                    .chain(std::iter::once((id.id() + num_sites + num_caves, info.name.clone(), info.location.1)))
                            }) // unwrap inner iteration
                            .map(|(id, name, pos)| world_msg::SiteInfo {
                                id,
                                name: Some(name),
                                kind: world_msg::SiteKind::Cave,
                                wpos: pos,
                            }),
                    )
                    .chain(layer::cave::surface_entrances(&Land::from_sim(self.sim()))
                        .enumerate()
                        .map(|(i, wpos)| world_msg::SiteInfo {
                            id: 65536 + i as u64, // Generate a fake ID, TODO: don't do this
                            name: None,
                            kind: world_msg::SiteKind::Cave,
                            wpos,
                        }))
                    .collect(),
                ..self.sim.get_map(index, self.sim().calendar.as_ref())
            }
        })
    }

    pub fn sample_columns(
        &self,
    ) -> impl Sampler<
        Index = (Vec2<i32>, IndexRef, Option<&'_ Calendar>),
        Sample = Option<ColumnSample>,
    > + '_ {
        ColumnGen::new(&self.sim)
    }

    pub fn sample_blocks(&self) -> BlockGen { BlockGen::new(ColumnGen::new(&self.sim)) }

    pub fn find_accessible_pos(
        &self,
        index: IndexRef,
        spawn_wpos: Vec2<i32>,
        ascending: bool,
    ) -> Vec3<f32> {
        let chunk_pos = TerrainGrid::chunk_key(spawn_wpos);

        // Unwrapping because generate_chunk only returns err when should_continue evals
        // to true
        let (tc, _cs) = self
            .generate_chunk(index, chunk_pos, || false, None)
            .unwrap();

        tc.find_accessible_pos(spawn_wpos, ascending)
    }

    #[allow(clippy::result_unit_err)]
    pub fn generate_chunk(
        &self,
        index: IndexRef,
        chunk_pos: Vec2<i32>,
        // TODO: misleading name
        mut should_continue: impl FnMut() -> bool,
        time: Option<(TimeOfDay, Calendar)>,
    ) -> Result<(TerrainChunk, ChunkSupplement), ()> {
        let calendar = time.as_ref().map(|(_, cal)| cal);

        let mut sampler = self.sample_blocks();

        let chunk_wpos2d = chunk_pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
        let chunk_center_wpos2d = chunk_wpos2d + TerrainChunkSize::RECT_SIZE.map(|e| e as i32 / 2);
        let grid_border = 4;
        let zcache_grid = Grid::populate_from(
            TerrainChunkSize::RECT_SIZE.map(|e| e as i32) + grid_border * 2,
            |offs| sampler.get_z_cache(chunk_wpos2d - grid_border + offs, index, calendar),
        );

        let air = Block::air(SpriteKind::Empty);
        let stone = Block::new(
            BlockKind::Rock,
            zcache_grid
                .get(grid_border + TerrainChunkSize::RECT_SIZE.map(|e| e as i32) / 2)
                .and_then(|zcache| zcache.as_ref())
                .map(|zcache| zcache.sample.stone_col)
                .unwrap_or_else(|| index.colors.deep_stone_color.into()),
        );

        let (base_z, sim_chunk) = match self
            .sim
            /*.get_interpolated(
                chunk_pos.map2(chunk_size2d, |e, sz: u32| e * sz as i32 + sz as i32 / 2),
                |chunk| chunk.get_base_z(),
            )
            .and_then(|base_z| self.sim.get(chunk_pos).map(|sim_chunk| (base_z, sim_chunk))) */
            .get_base_z(chunk_pos)
        {
            Some(base_z) => (base_z as i32, self.sim.get(chunk_pos).unwrap()),
            // Some((base_z, sim_chunk)) => (base_z as i32, sim_chunk),
            None => {
                // NOTE: This is necessary in order to generate a handful of chunks at the edges
                // of the map.
                return Ok((self.sim().generate_oob_chunk(), ChunkSupplement::default()));
            },
        };

        let meta = TerrainChunkMeta::new(
            sim_chunk
                .sites
                .iter()
                .filter(|id| {
                    index.sites[**id]
                        .get_origin()
                        .distance_squared(chunk_center_wpos2d) as f32
                        <= index.sites[**id].radius().powi(2)
                })
                .min_by_key(|id| {
                    index.sites[**id]
                        .get_origin()
                        .distance_squared(chunk_center_wpos2d)
                })
                .map(|id| index.sites[*id].name().to_string())
                .or_else(|| sim_chunk.poi.map(|poi| self.civs.pois[poi].name.clone())),
            sim_chunk.get_biome(),
            sim_chunk.alt,
            sim_chunk.tree_density,
            sim_chunk.cave.1.alt != 0.0,
            sim_chunk.river.is_river(),
            sim_chunk.river.velocity,
            sim_chunk.temp,
            sim_chunk.humidity,
            sim_chunk
                .sites
                .iter()
                .find_map(|site| index.sites[*site].kind.convert_to_meta()),
        );

        let mut chunk = TerrainChunk::new(base_z, stone, air, meta);

        for y in 0..TerrainChunkSize::RECT_SIZE.y as i32 {
            for x in 0..TerrainChunkSize::RECT_SIZE.x as i32 {
                if should_continue() {
                    return Err(());
                };

                let offs = Vec2::new(x, y);

                let z_cache = match zcache_grid.get(grid_border + offs) {
                    Some(Some(z_cache)) => z_cache,
                    _ => continue,
                };

                let (min_z, max_z) = z_cache.get_z_limits();

                (base_z..min_z as i32).for_each(|z| {
                    let _ = chunk.set(Vec3::new(x, y, z), stone);
                });

                (min_z as i32..max_z as i32).for_each(|z| {
                    let lpos = Vec3::new(x, y, z);
                    let wpos = Vec3::from(chunk_wpos2d) + lpos;

                    if let Some(block) = sampler.get_with_z_cache(wpos, Some(z_cache)) {
                        let _ = chunk.set(lpos, block);
                    }
                });
            }
        }

        let sample_get = |offs| {
            zcache_grid
                .get(grid_border + offs)
                .and_then(Option::as_ref)
                .map(|zc| &zc.sample)
        };

        // Only use for rng affecting dynamic elements like chests and entities!
        let mut dynamic_rng = ChaCha8Rng::from_seed(thread_rng().gen());

        // Apply layers (paths, caves, etc.)
        let mut canvas = Canvas {
            info: CanvasInfo {
                chunk_pos,
                wpos: chunk_pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32),
                column_grid: &zcache_grid,
                column_grid_border: grid_border,
                chunks: &self.sim,
                index,
                chunk: sim_chunk,
                calendar,
            },
            chunk: &mut chunk,
            entities: Vec::new(),
        };

        if index.features.train_tracks {
            layer::apply_trains_to(&mut canvas, &self.sim, sim_chunk, chunk_center_wpos2d);
        }

        if index.features.caverns {
            layer::apply_caverns_to(&mut canvas, &mut dynamic_rng);
        }
        if index.features.caves {
            layer::apply_caves_to(&mut canvas, &mut dynamic_rng);
        }
        layer::apply_caves2_to(&mut canvas, &mut dynamic_rng);
        if index.features.rocks {
            layer::apply_rocks_to(&mut canvas, &mut dynamic_rng);
        }
        if index.features.shrubs {
            layer::apply_shrubs_to(&mut canvas, &mut dynamic_rng);
        }
        if index.features.trees {
            layer::apply_trees_to(&mut canvas, &mut dynamic_rng, calendar);
        }
        if index.features.scatter {
            layer::apply_scatter_to(&mut canvas, &mut dynamic_rng, calendar);
        }
        if index.features.paths {
            layer::apply_paths_to(&mut canvas);
        }
        if index.features.spots {
            layer::apply_spots_to(&mut canvas, &mut dynamic_rng);
        }
        // layer::apply_coral_to(&mut canvas);

        // Apply site generation
        sim_chunk
            .sites
            .iter()
            .for_each(|site| index.sites[*site].apply_to(&mut canvas, &mut dynamic_rng));

        let mut supplement = ChunkSupplement {
            entities: canvas.entities,
        };

        let gen_entity_pos = |dynamic_rng: &mut ChaCha8Rng| {
            let lpos2d = TerrainChunkSize::RECT_SIZE
                .map(|sz| dynamic_rng.gen::<u32>().rem_euclid(sz) as i32);
            let mut lpos = Vec3::new(
                lpos2d.x,
                lpos2d.y,
                sample_get(lpos2d).map(|s| s.alt as i32 - 32).unwrap_or(0),
            );

            while let Some(block) = chunk.get(lpos).ok().copied().filter(Block::is_solid) {
                lpos.z += block.solid_height().ceil() as i32;
            }

            (Vec3::from(chunk_wpos2d) + lpos).map(|e: i32| e as f32) + 0.5
        };

        if sim_chunk.contains_waypoint {
            let waypoint_pos = gen_entity_pos(&mut dynamic_rng);
            if sim_chunk
                .sites
                .iter()
                .map(|site| index.sites[*site].spawn_rules(waypoint_pos.xy().as_()))
                .fold(SpawnRules::default(), |a, b| a.combine(b))
                .waypoints
            {
                supplement.add_entity(EntityInfo::at(waypoint_pos).into_waypoint());
            }
        }

        // Apply layer supplement
        layer::apply_caves_supplement(
            &mut dynamic_rng,
            chunk_wpos2d,
            sample_get,
            &chunk,
            index,
            &mut supplement,
        );

        // Apply layer supplement
        layer::wildlife::apply_wildlife_supplement(
            &mut dynamic_rng,
            chunk_wpos2d,
            sample_get,
            &chunk,
            index,
            sim_chunk,
            &mut supplement,
            time.as_ref(),
        );

        // Apply site supplementary information
        sim_chunk.sites.iter().for_each(|site| {
            index.sites[*site].apply_supplement(
                &mut dynamic_rng,
                chunk_wpos2d,
                sample_get,
                &mut supplement,
                site.id(),
            )
        });

        // Finally, defragment to minimize space consumption.
        chunk.defragment();

        Ok((chunk, supplement))
    }

    // Zone coordinates
    pub fn get_lod_zone(&self, pos: Vec2<i32>, index: IndexRef) -> lod::Zone {
        let min_wpos = pos.map(lod::to_wpos);
        let max_wpos = (pos + 1).map(lod::to_wpos);

        let mut objects = Vec::new();

        // Add trees
        objects.append(
            &mut self
                .sim()
                .get_area_trees(min_wpos, max_wpos)
                .filter_map(|attr| {
                    ColumnGen::new(self.sim())
                        .get((attr.pos, index, self.sim().calendar.as_ref()))
                        .filter(|col| layer::tree::tree_valid_at(attr.pos, col, None, attr.seed))
                        .zip(Some(attr))
                })
                .filter_map(|(col, tree)| {
                    Some(lod::Object {
                        kind: match tree.forest_kind {
                            all::ForestKind::Oak => lod::ObjectKind::Oak,
                            all::ForestKind::Dead => lod::ObjectKind::Dead,
                            all::ForestKind::Pine
                            | all::ForestKind::Frostpine
                            | all::ForestKind::Redwood => lod::ObjectKind::Pine,
                            _ => lod::ObjectKind::Oak,
                        },
                        pos: {
                            let rpos = tree.pos - min_wpos;
                            if rpos.is_any_negative() {
                                return None;
                            } else {
                                rpos.map(|e| e as i16).with_z(col.alt as i16)
                            }
                        },
                        flags: lod::Flags::empty()
                            | if col.snow_cover {
                                lod::Flags::SNOW_COVERED
                            } else {
                                lod::Flags::empty()
                            },
                    })
                })
                .collect(),
        );

        // Add buildings
        objects.extend(
            index
                .sites
                .iter()
                .filter(|(_, site)| {
                    site.get_origin()
                        .map2(min_wpos.zip(max_wpos), |e, (min, max)| e >= min && e < max)
                        .reduce_and()
                })
                .filter_map(|(_, site)| match &site.kind {
                    SiteKind::Refactor(site) => {
                        Some(site.plots().filter_map(|plot| match &plot.kind {
                            site2::plot::PlotKind::House(_) => Some(site.tile_wpos(plot.root_tile)),
                            _ => None,
                        }))
                    },
                    _ => None,
                })
                .flatten()
                .map(|wpos2d| lod::Object {
                    kind: lod::ObjectKind::House,
                    pos: (wpos2d - min_wpos)
                        .map(|e| e as i16)
                        .with_z(self.sim().get_alt_approx(wpos2d).unwrap_or(0.0) as i16),
                    flags: lod::Flags::empty(),
                }),
        );

        // Add giant trees
        objects.extend(
            index
                .sites
                .iter()
                .filter(|(_, site)| {
                    site.get_origin()
                        .map2(min_wpos.zip(max_wpos), |e, (min, max)| e >= min && e < max)
                        .reduce_and()
                })
                .filter(|(_, site)| matches!(&site.kind, SiteKind::GiantTree(_)))
                .filter_map(|(_, site)| {
                    let wpos2d = site.get_origin();
                    let col = ColumnGen::new(self.sim()).get((
                        wpos2d,
                        index,
                        self.sim().calendar.as_ref(),
                    ))?;
                    Some(lod::Object {
                        kind: lod::ObjectKind::GiantTree,
                        pos: {
                            (wpos2d - min_wpos)
                                .map(|e| e as i16)
                                .with_z(self.sim().get_alt_approx(wpos2d).unwrap_or(0.0) as i16)
                        },
                        flags: lod::Flags::empty()
                            | if col.snow_cover {
                                lod::Flags::SNOW_COVERED
                            } else {
                                lod::Flags::empty()
                            },
                    })
                }),
        );

        lod::Zone { objects }
    }
}
