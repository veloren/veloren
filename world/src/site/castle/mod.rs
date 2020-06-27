use super::SpawnRules;
use crate::{
    block::block_from_structure,
    column::ColumnSample,
    sim::WorldSim,
    site::{
        BlockMask,
        settlement::building::{Archetype, Ori, Branch, archetype::keep::{Keep, Attr}},
    },
    util::{attempt, Grid, RandomField, Sampler, CARDINALS, DIRS},
};
use common::{
    assets,
    astar::Astar,
    comp,
    generation::{ChunkSupplement, EntityInfo},
    npc,
    store::{Id, Store},
    terrain::{Block, BlockKind, Structure, TerrainChunkSize},
    vol::{BaseVol, ReadVol, RectSizedVol, RectVolSize, Vox, WriteVol},
};
use core::{f32, hash::BuildHasherDefault};
use fxhash::FxHasher64;
use lazy_static::lazy_static;
use rand::prelude::*;
use std::sync::Arc;
use vek::*;

struct Segment {
    offset: Vec2<i32>,
    locus: i32,
    height: i32,
    is_tower: bool,
}

struct Tower {
    offset: Vec2<i32>,
    alt: i32,
}

pub struct Castle {
    origin: Vec2<i32>,
    alt: i32,
    seed: u32,
    towers: Vec<Tower>,
    segments: Vec<Segment>,
}

pub struct GenCtx<'a, R: Rng> {
    sim: Option<&'a WorldSim>,
    rng: &'a mut R,
}

impl Castle {
    #[allow(clippy::let_and_return)] // TODO: Pending review in #587
    pub fn generate(wpos: Vec2<i32>, sim: Option<&WorldSim>, rng: &mut impl Rng) -> Self {
        let mut ctx = GenCtx { sim, rng };

        let boundary_towers = ctx.rng.gen_range(5, 10);

        let this = Self {
            origin: wpos,
            alt: ctx
                .sim
                .and_then(|sim| sim.get_alt_approx(wpos))
                .unwrap_or(0.0) as i32
                + 6,
            seed: ctx.rng.gen(),

            towers: (0..boundary_towers)
                .map(|i| {
                    let angle = (i as f32 / boundary_towers as f32) * f32::consts::PI * 2.0;
                    let dir = Vec2::new(
                        angle.cos(),
                        angle.sin(),
                    );
                    let dist = ctx.rng.gen_range(45.0, 190.0).clamped(75.0, 135.0);

                    let offset = (dir * dist).map(|e| e as i32);

                    Tower {
                        offset,
                        alt: ctx
                            .sim
                            .and_then(|sim| sim.get_alt_approx(wpos + offset))
                            .unwrap_or(0.0) as i32 + 2,
                    }
                })
                .collect(),

            segments: (0..0)//rng.gen_range(18, 24))
                .map(|_| {
                    let dir = Vec2::new(
                        rng.gen_range(-1.0, 1.0),
                        rng.gen_range(-1.0, 1.0),
                    ).normalized();
                    let dist = 16.0 + rng.gen_range(0.0f32, 1.0).powf(0.5) * 64.0;
                    let height = 48.0 - (dist / 64.0).powf(2.0) * 32.0;

                    Segment {
                       offset: (dir * dist).map(|e| e as i32),
                       locus: rng.gen_range(6, 26),
                       height: height as i32,
                       is_tower: height > 36.0,
                   }
                })
                .collect(),
        };

        this
    }

    pub fn get_origin(&self) -> Vec2<i32> { self.origin }

    pub fn radius(&self) -> f32 { 1200.0 }

    #[allow(clippy::needless_update)] // TODO: Pending review in #587
    pub fn spawn_rules(&self, wpos: Vec2<i32>) -> SpawnRules {
        SpawnRules {
            trees: wpos.distance_squared(self.origin) > 64i32.pow(2),
            ..SpawnRules::default()
        }
    }

    pub fn apply_to<'a>(
        &'a self,
        wpos2d: Vec2<i32>,
        mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    ) {
        for y in 0..vol.size_xy().y as i32 {
            for x in 0..vol.size_xy().x as i32 {
                let offs = Vec2::new(x, y);

                let wpos2d = wpos2d + offs;
                let rpos = wpos2d - self.origin;

                // Apply the dungeon entrance
                let col_sample = if let Some(col) = get_column(offs) {
                    col
                } else {
                    continue;
                };

                let (wall_dist, wall_pos, wall_alt) = (0..self.towers.len())
                    .map(|i| {
                        let tower0 = &self.towers[i];
                        let tower1 = &self.towers[(i + 1) % self.towers.len()];

                        let wall = LineSegment2 {
                            start: tower0.offset.map(|e| e as f32),
                            end: tower1.offset.map(|e| e as f32),
                        };

                        let projected = wall.projected_point(rpos.map(|e| e as f32)).map(|e| e as i32);

                        let tower0_dist = tower0.offset.map(|e| e as f32).distance(projected.map(|e| e as f32));
                        let tower1_dist = tower1.offset.map(|e| e as f32).distance(projected.map(|e| e as f32));
                        let tower_lerp = tower0_dist / (tower0_dist + tower1_dist);

                        (
                            wall.distance_to_point(rpos.map(|e| e as f32)) as i32,
                            projected,
                            Lerp::lerp(tower0.alt as f32, tower1.alt as f32, tower_lerp) as i32,
                        )
                    })
                    .min_by_key(|x| x.0)
                    .unwrap();

                for z in -10..64 {
                    let wpos = Vec3::new(
                        wpos2d.x,
                        wpos2d.y,
                        col_sample.alt as i32 + z,
                    );

                    // Boundary
                    let border_pos = (wall_pos - rpos).map(|e| e.abs());
                    let mut mask = Keep.draw(
                        Vec3::from(rpos) + Vec3::unit_z() * wpos.z - wall_alt,
                        wall_dist,
                        Vec2::new(border_pos.reduce_max(), border_pos.reduce_min()),
                        rpos - wall_pos,
                        wpos.z - wall_alt,
                        Ori::North,
                        &Branch {
                            len: 0,
                            attr: Attr {
                                height: 16,
                                is_tower: false,
                            },
                            locus: 4,
                            border: 0,
                            children: Vec::new(),
                        }
                    );
                    for tower in &self.towers {
                        let tower_wpos = Vec3::new(
                            self.origin.x + tower.offset.x,
                            self.origin.y + tower.offset.y,
                            tower.alt,
                        );
                        let tower_locus = 10;

                        let border_pos = (tower_wpos - wpos).xy().map(|e| e.abs());
                        mask = mask.resolve_with(Keep.draw(
                            wpos - tower_wpos,
                            border_pos.reduce_max() - tower_locus,
                            Vec2::new(border_pos.reduce_max(), border_pos.reduce_min()),
                            (wpos - tower_wpos).xy(),
                            wpos.z - tower.alt,
                            Ori::North,
                            &Branch {
                                len: 0,
                                attr: Attr {
                                    height: 28,
                                    is_tower: true,
                                },
                                locus: tower_locus,
                                border: 0,
                                children: Vec::new(),
                            }
                        ));
                    }

                    if let Some(block) = mask.finish() {
                        let _ = vol.set(Vec3::new(offs.x, offs.y, wpos.z), block);
                    }
                }
            }
        }
    }

    #[allow(clippy::or_fun_call)] // TODO: Pending review in #587
    pub fn apply_supplement<'a>(
        &'a self,
        rng: &mut impl Rng,
        wpos2d: Vec2<i32>,
        _get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
        supplement: &mut ChunkSupplement,
    ) {
        // TODO
    }
}
