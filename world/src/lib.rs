#![deny(unsafe_code)]
#![allow(incomplete_features)]
#![feature(arbitrary_enum_discriminant, const_generics, label_break_value)]

mod all;
mod block;
pub mod civ;
mod column;
pub mod config;
pub mod layer;
pub mod sim;
pub mod site;
pub mod util;

// Reexports
pub use crate::config::CONFIG;

use crate::{
    block::BlockGen,
    column::{ColumnGen, ColumnSample},
    util::{Grid, Sampler},
};
use common::{
    comp::{self, bird_medium, critter, quadruped_medium, quadruped_small, dragon},
    generation::{ChunkSupplement, EntityInfo},
    terrain::{Block, BlockKind, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    vol::{ReadVol, RectVolSize, Vox, WriteVol},
};
use rand::Rng;
use std::time::Duration;
use vek::*;

#[derive(Debug)]
pub enum Error {
    Other(String),
}

pub struct World {
    sim: sim::WorldSim,
    civs: civ::Civs,
}

impl World {
    pub fn generate(seed: u32, opts: sim::WorldOpts) -> Self {
        let mut sim = sim::WorldSim::generate(seed, opts);
        let civs = civ::Civs::generate(seed, &mut sim);
        Self { sim, civs }
    }

    pub fn sim(&self) -> &sim::WorldSim { &self.sim }

    pub fn civs(&self) -> &civ::Civs { &self.civs }

    pub fn tick(&self, _dt: Duration) {
        // TODO
    }

    pub fn sample_columns(
        &self,
    ) -> impl Sampler<Index = Vec2<i32>, Sample = Option<ColumnSample>> + '_ {
        ColumnGen::new(&self.sim)
    }

    pub fn sample_blocks(&self) -> BlockGen { BlockGen::new(ColumnGen::new(&self.sim)) }

    pub fn generate_chunk(
        &self,
        chunk_pos: Vec2<i32>,
        // TODO: misleading name
        mut should_continue: impl FnMut() -> bool,
    ) -> Result<(TerrainChunk, ChunkSupplement), ()> {
        let mut sampler = self.sample_blocks();

        let chunk_wpos2d = Vec2::from(chunk_pos) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
        let grid_border = 4;
        let zcache_grid = Grid::populate_from(
            TerrainChunkSize::RECT_SIZE.map(|e| e as i32) + grid_border * 2,
            |offs| sampler.get_z_cache(chunk_wpos2d - grid_border + offs),
        );

        let air = Block::empty();
        let stone = Block::new(
            BlockKind::Dense,
            zcache_grid
                .get(grid_border + TerrainChunkSize::RECT_SIZE.map(|e| e as i32) / 2)
                .and_then(|zcache| zcache.as_ref())
                .map(|zcache| zcache.sample.stone_col)
                .unwrap_or(Rgb::new(125, 120, 130)),
        );
        let water = Block::new(BlockKind::Water, Rgb::new(60, 90, 190));

        let _chunk_size2d = TerrainChunkSize::RECT_SIZE;
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
                return Ok((
                    TerrainChunk::new(
                        CONFIG.sea_level as i32,
                        water,
                        air,
                        TerrainChunkMeta::void(),
                    ),
                    ChunkSupplement::default(),
                ));
            },
        };

        let meta = TerrainChunkMeta::new(sim_chunk.get_name(&self.sim), sim_chunk.get_biome());

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

                let (min_z, only_structures_min_z, max_z) = z_cache.get_z_limits(&mut sampler);

                (base_z..min_z as i32).for_each(|z| {
                    let _ = chunk.set(Vec3::new(x, y, z), stone);
                });

                (min_z as i32..max_z as i32).for_each(|z| {
                    let lpos = Vec3::new(x, y, z);
                    let wpos = Vec3::from(chunk_wpos2d) + lpos;
                    let only_structures = lpos.z >= only_structures_min_z as i32;

                    if let Some(block) =
                        sampler.get_with_z_cache(wpos, Some(&z_cache), only_structures)
                    {
                        let _ = chunk.set(lpos, block);
                    }
                });
            }
        }

        let sample_get = |offs| {
            zcache_grid
                .get(grid_border + offs)
                .map(Option::as_ref)
                .flatten()
                .map(|zc| &zc.sample)
        };

        let mut rng = rand::thread_rng();

        // Apply site generation
        sim_chunk
            .sites
            .iter()
            .for_each(|site| site.apply_to(chunk_wpos2d, sample_get, &mut chunk));

        // Apply paths
        layer::apply_paths_to(chunk_wpos2d, sample_get, &mut chunk);

        let gen_entity_pos = || {
            let lpos2d = TerrainChunkSize::RECT_SIZE
                .map(|sz| rand::thread_rng().gen::<u32>().rem_euclid(sz) as i32);
            let mut lpos = Vec3::new(
                lpos2d.x,
                lpos2d.y,
                sample_get(lpos2d).map(|s| s.alt as i32 - 32).unwrap_or(0),
            );

            while chunk.get(lpos).map(|vox| !vox.is_empty()).unwrap_or(false) {
                lpos.z += 1;
            }

            (Vec3::from(chunk_wpos2d) + lpos).map(|e: i32| e as f32) + 0.5
        };

        const SPAWN_RATE: f32 = 0.1;
        let mut supplement = ChunkSupplement {
            entities: if rng.gen::<f32>() < SPAWN_RATE
                && sim_chunk.chaos < 0.5
                && !sim_chunk.is_underwater()
            {
                let entity = EntityInfo::at(gen_entity_pos())
                    .with_alignment(comp::Alignment::Wild)
                    .do_if(rng.gen_range(0, 8) == 0, |e| e.into_giant())
                    .with_body(match rng.gen_range(0, 4) {
                        0 => comp::Body::QuadrupedMedium(quadruped_medium::Body::random()),
                        1 => comp::Body::BirdMedium(bird_medium::Body::random()),
                        2 => comp::Body::Critter(critter::Body::random()),
                        _ => comp::Body::QuadrupedSmall(quadruped_small::Body::random()),
                    })
                    .with_automatic_name();

                vec![entity]
            } else {
                Vec::new()
            },
        };

        if sim_chunk.contains_waypoint {
            supplement.add_entity(EntityInfo::at(gen_entity_pos()).into_waypoint());
        }

        // Apply site supplementary information
        sim_chunk.sites.iter().for_each(|site| {
            site.apply_supplement(&mut rng, chunk_wpos2d, sample_get, &mut supplement)
        });

        Ok((chunk, supplement))
    }
}
