#![deny(unsafe_code)]
#![allow(incomplete_features)]
#![feature(arbitrary_enum_discriminant, const_generics, label_break_value)]

mod all;
mod block;
mod column;
pub mod config;
pub mod generator;
pub mod sim;
pub mod util;

// Reexports
pub use crate::config::CONFIG;

use crate::{
    block::BlockGen,
    column::{ColumnGen, ColumnSample},
    util::Sampler,
};
use common::{
    generation::{ChunkSupplement, EntityInfo, EntityKind},
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
}

impl World {
    pub fn generate(seed: u32, opts: sim::WorldOpts) -> Self {
        Self {
            sim: sim::WorldSim::generate(seed, opts),
        }
    }

    pub fn sim(&self) -> &sim::WorldSim {
        &self.sim
    }

    pub fn tick(&self, _dt: Duration) {
        // TODO
    }

    pub fn sample_columns(
        &self,
    ) -> impl Sampler<Index = Vec2<i32>, Sample = Option<ColumnSample>> + '_ {
        ColumnGen::new(&self.sim)
    }

    pub fn sample_blocks(&self) -> BlockGen {
        BlockGen::new(ColumnGen::new(&self.sim))
    }

    pub fn generate_chunk(
        &self,
        chunk_pos: Vec2<i32>,
        // TODO: misleading name
        mut should_continue: impl FnMut() -> bool,
    ) -> Result<(TerrainChunk, ChunkSupplement), ()> {
        let air = Block::empty();
        let stone = Block::new(BlockKind::Dense, Rgb::new(200, 220, 255));
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
                ))
            }
        };

        let meta = TerrainChunkMeta::new(sim_chunk.get_name(&self.sim), sim_chunk.get_biome());
        let mut sampler = self.sample_blocks();

        let chunk_block_pos = Vec3::from(chunk_pos) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);

        let mut chunk = TerrainChunk::new(base_z, stone, air, meta);
        for y in 0..TerrainChunkSize::RECT_SIZE.y as i32 {
            for x in 0..TerrainChunkSize::RECT_SIZE.x as i32 {
                if should_continue() {
                    return Err(());
                };
                let wpos2d = Vec2::new(x, y)
                    + Vec2::from(chunk_pos) * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);

                let z_cache = match sampler.get_z_cache(wpos2d) {
                    Some(z_cache) => z_cache,
                    None => continue,
                };

                let (min_z, only_structures_min_z, max_z) = z_cache.get_z_limits(&mut sampler);

                (base_z..min_z as i32).for_each(|z| {
                    let _ = chunk.set(Vec3::new(x, y, z), stone);
                });

                (min_z as i32..max_z as i32).for_each(|z| {
                    let lpos = Vec3::new(x, y, z);
                    let wpos = chunk_block_pos + lpos;
                    let only_structures = lpos.z >= only_structures_min_z as i32;

                    if let Some(block) =
                        sampler.get_with_z_cache(wpos, Some(&z_cache), only_structures)
                    {
                        let _ = chunk.set(lpos, block);
                    }
                });
            }
        }

        let gen_entity_pos = || {
            let lpos2d = TerrainChunkSize::RECT_SIZE
                .map(|sz| rand::thread_rng().gen::<u32>().rem_euclid(sz));
            let mut lpos = Vec3::new(lpos2d.x as i32, lpos2d.y as i32, 0);

            while chunk.get(lpos).map(|vox| !vox.is_empty()).unwrap_or(false) {
                lpos.z += 1;
            }

            (chunk_block_pos + lpos).map(|e| e as f32) + 0.5
        };

        const SPAWN_RATE: f32 = 0.1;
        const BOSS_RATE: f32 = 0.03;
        let mut supplement = ChunkSupplement {
            entities: if rand::thread_rng().gen::<f32>() < SPAWN_RATE
                && sim_chunk.chaos < 0.5
                && !sim_chunk.is_underwater
            {
                vec![EntityInfo {
                    pos: gen_entity_pos(),
                    kind: if rand::thread_rng().gen::<f32>() < BOSS_RATE {
                        EntityKind::Boss
                    } else {
                        EntityKind::Enemy
                    },
                }]
            } else {
                Vec::new()
            },
        };

        if sim_chunk.contains_waypoint {
            supplement = supplement.with_entity(EntityInfo {
                pos: gen_entity_pos(),
                kind: EntityKind::Waypoint,
            });
        }

        Ok((chunk, supplement))
    }
}
