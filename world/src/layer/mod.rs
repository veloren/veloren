pub mod scatter;
pub mod spot;
pub mod tree;
pub mod wildlife;

pub use self::{scatter::apply_scatter_to, spot::apply_spots_to, tree::apply_trees_to};

use crate::{
    column::ColumnSample,
    util::{FastNoise, RandomField, Sampler},
    Canvas, IndexRef,
};
use common::{
    assets::AssetExt,
    comp,
    generation::{ChunkSupplement, EntityInfo},
    lottery::Lottery,
    terrain::{Block, BlockKind, SpriteKind},
    vol::{BaseVol, ReadVol, RectSizedVol, WriteVol},
};
use noise::NoiseFn;
use rand::prelude::*;
use serde::Deserialize;
use std::{
    f32,
    ops::{Mul, Range, Sub},
};
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    pub bridge: (u8, u8, u8),
    pub stalagtite: (u8, u8, u8),
    pub cave_floor: (u8, u8, u8),
    pub cave_roof: (u8, u8, u8),
    pub dirt: (u8, u8, u8),
    pub scaffold: (u8, u8, u8),
    pub lava: (u8, u8, u8),
    pub vein: (u8, u8, u8),
}

const EMPTY_AIR: Block = Block::air(SpriteKind::Empty);

pub fn apply_paths_to(canvas: &mut Canvas) {
    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let surface_z = col.riverless_alt.floor() as i32;

        let noisy_color = |color: Rgb<u8>, factor: u32| {
            let nz = RandomField::new(0).get(Vec3::new(wpos2d.x, wpos2d.y, surface_z));
            color.map(|e| {
                (e as u32 + nz % (factor * 2))
                    .saturating_sub(factor)
                    .min(255) as u8
            })
        };

        if let Some((path_dist, path_nearest, path, _)) =
            col.path.filter(|(dist, _, path, _)| *dist < path.width)
        {
            let inset = 0;

            // Try to use the column at the centre of the path for sampling to make them
            // flatter
            let col_pos = -info.wpos().map(|e| e as f32) + path_nearest;
            let col00 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 0));
            let col10 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 0));
            let col01 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 1));
            let col11 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 1));
            let col_attr = |col: &ColumnSample| {
                Vec3::new(col.riverless_alt, col.alt, col.water_dist.unwrap_or(1000.0))
            };
            let [riverless_alt, alt, water_dist] = match (col00, col10, col01, col11) {
                (Some(col00), Some(col10), Some(col01), Some(col11)) => Lerp::lerp(
                    Lerp::lerp(col_attr(col00), col_attr(col10), path_nearest.x.fract()),
                    Lerp::lerp(col_attr(col01), col_attr(col11), path_nearest.x.fract()),
                    path_nearest.y.fract(),
                ),
                _ => col_attr(col),
            }
            .into_array();
            let (bridge_offset, depth) = (
                ((water_dist.max(0.0) * 0.2).min(f32::consts::PI).cos() + 1.0) * 5.0,
                ((1.0 - ((water_dist + 2.0) * 0.3).min(0.0).cos().abs())
                    * (riverless_alt + 5.0 - alt).max(0.0)
                    * 1.75
                    + 3.0) as i32,
            );
            let surface_z = (riverless_alt + bridge_offset).floor() as i32;

            for z in inset - depth..inset {
                let _ = canvas.set(
                    Vec3::new(wpos2d.x, wpos2d.y, surface_z + z),
                    if bridge_offset >= 2.0 && path_dist >= 3.0 || z < inset - 1 {
                        Block::new(
                            BlockKind::Rock,
                            noisy_color(info.index().colors.layer.bridge.into(), 8),
                        )
                    } else {
                        let path_color =
                            path.surface_color(col.sub_surface_color.map(|e| (e * 255.0) as u8));
                        Block::new(BlockKind::Earth, noisy_color(path_color, 8))
                    },
                );
            }
            let head_space = path.head_space(path_dist);
            for z in inset..inset + head_space {
                let pos = Vec3::new(wpos2d.x, wpos2d.y, surface_z + z);
                if canvas.get(pos).kind() != BlockKind::Water {
                    let _ = canvas.set(pos, EMPTY_AIR);
                }
            }
        }
    });
}

pub fn apply_caves_to(canvas: &mut Canvas, rng: &mut impl Rng) {
    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let surface_z = col.alt.floor() as i32;

        if let Some((cave_dist, _, cave, _)) =
            col.cave.filter(|(dist, _, cave, _)| *dist < cave.width)
        {
            let cave_x = (cave_dist / cave.width).min(1.0);

            // Relative units
            let cave_depth = (col.alt - cave.alt).max(0.0);

            let cave_floor = 0.0 - 0.5 * (1.0 - cave_x.powi(2)).max(0.0).sqrt() * cave.width;
            let cave_height = (1.0 - cave_x.powi(2)).max(0.0).sqrt() * cave.width;

            // Abs units
            let cave_base = (cave.alt + cave_floor) as i32;
            let cave_roof = (cave.alt + cave_height) as i32;

            for z in cave_base..cave_roof {
                if cave_x < 0.95
                    || info.index().noise.cave_nz.get(
                        Vec3::new(wpos2d.x, wpos2d.y, z)
                            .map(|e| e as f64 * 0.15)
                            .into_array(),
                    ) < 0.0
                {
                    // If the block a little above is liquid, we should stop carving out the cave in
                    // order to leave a ceiling, and not floating water
                    if canvas.get(Vec3::new(wpos2d.x, wpos2d.y, z + 2)).is_liquid() {
                        break;
                    }

                    canvas.map(Vec3::new(wpos2d.x, wpos2d.y, z), |b| {
                        if b.is_liquid() { b } else { EMPTY_AIR }
                    });
                }
            }
            let noisy_color = |color: Rgb<u8>, factor: u32| {
                let nz = RandomField::new(0).get(Vec3::new(wpos2d.x, wpos2d.y, surface_z));
                color.map(|e| {
                    (e as u32 + nz % (factor * 2))
                        .saturating_sub(factor)
                        .min(255) as u8
                })
            };

            let ridge_condition = cave_depth % 10.0 > 8.0 && cave_depth > 10.0;
            let pit_condition = cave_depth % 42.0 > 37.0 && cave_x > 0.6 && cave_depth > 200.0;
            let pit_depth = 30;
            let floor_dist = pit_condition as i32 * pit_depth as i32;
            let vein_condition =
                cave_depth % 12.0 > 11.5 && cave_x > 0.1 && cave_x < 0.6 && cave_depth > 200.0;
            let stalagtite_condition = cave_depth > 150.0;
            let vein_depth = 3;
            let vein_floor = cave_base - vein_depth;
            // Stalagtites
            let stalagtites = info
                .index()
                .noise
                .cave_nz
                .get(wpos2d.map(|e| e as f64 * 0.18).into_array())
                .sub(0.5)
                .max(0.0)
                .mul(
                    (col.alt - cave_roof as f32 - 5.0)
                        .mul(0.15)
                        .clamped(0.0, 1.0) as f64,
                )
                .mul(45.0) as i32;

            // Generate stalagtites if there's something for them to hold on to
            if canvas
                .get(Vec3::new(wpos2d.x, wpos2d.y, cave_roof))
                .is_filled()
                && stalagtite_condition
            {
                for z in cave_roof - stalagtites..cave_roof {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::WeakRock,
                            noisy_color(info.index().colors.layer.stalagtite.into(), 8),
                        ),
                    );
                }
            }

            let ground_colors = if cave_roof - cave_base > 23 {
                noisy_color(info.index().colors.layer.cave_floor.into(), 8)
            } else {
                noisy_color(info.index().colors.layer.dirt.into(), 8)
            };

            //make pits
            for z in cave_base - pit_depth..cave_base {
                if pit_condition && (cave_roof - cave_base) > 10 {
                    let kind = if z < (cave_base - pit_depth) + (3 * pit_depth / 4) {
                        BlockKind::Lava
                    } else {
                        BlockKind::Air
                    };
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(kind, noisy_color(info.index().colors.layer.lava.into(), 8)),
                    );
                }
            }
            //fill bottom of pits
            for z in cave_base - pit_depth
                ..cave_base - pit_depth + ((cave_x.powf(4.0) * (pit_depth as f32 + 3.0)) as i32) + 1
            {
                if (cave_roof - cave_base) > 10 && pit_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(BlockKind::WeakRock, ground_colors),
                    );
                }
            }
            //empty veins
            for z in cave_base - vein_depth..cave_base {
                if vein_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::Air,
                            noisy_color(info.index().colors.layer.scaffold.into(), 8),
                        ),
                    );
                }
            }

            //fill veins except bottom later
            for z in cave_base - vein_depth + 1..cave_base {
                if vein_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::GlowingRock,
                            noisy_color(info.index().colors.layer.vein.into(), 16),
                        ),
                    );
                }
            }
            //fill some of bottom
            for z in cave_base - vein_depth..cave_base - vein_depth + 1 {
                if rng.gen::<f32>() < 0.5 && vein_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::GlowingRock,
                            noisy_color(info.index().colors.layer.vein.into(), 16),
                        ),
                    );
                }
            }
            if vein_condition && rng.gen::<f32>() > 0.7 {
                let kind = *Lottery::<SpriteKind>::load_expect("common.cave_scatter.vein")
                    .read()
                    .choose();
                canvas.map(Vec3::new(wpos2d.x, wpos2d.y, vein_floor), |block| {
                    block.with_sprite(kind)
                });
            }

            //fill normal floor
            for z in cave_base..cave_base + 1 {
                if cave_depth > 15.0
                    && (cave_roof - cave_base) > 10
                    && !pit_condition
                    && !vein_condition
                {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(BlockKind::WeakRock, ground_colors),
                    );
                }
            }
            //fill roof
            for z in cave_roof - 1..cave_roof {
                if cave_depth > 30.0 && (cave_roof - cave_base) > 10 {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::WeakRock,
                            noisy_color(info.index().colors.layer.cave_roof.into(), 8),
                        ),
                    );
                }
            }
            //add ridges
            for z in cave_roof - 4..cave_roof {
                if ridge_condition && (cave_roof - cave_base) > 10 {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::WeakRock,
                            noisy_color(info.index().colors.layer.scaffold.into(), 8),
                        ),
                    );
                }
            }

            let cave_roof_adjusted = if (cave_roof - cave_base) > 10 {
                cave_roof - 1
            } else {
                cave_roof
            };

            let cave_floor_adjusted = if (cave_roof - cave_base) > 10 {
                cave_base + 1 - floor_dist
            } else {
                cave_base - floor_dist
            };

            // Scatter things in caves
            if cave_depth > 40.0 && cave_depth < 80.0 {
                if rng.gen::<f32>() < 0.14 * (cave_x.max(0.5).powf(4.0)) && !vein_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.shallow_floor")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                        |block| block.with_sprite(kind),
                    );
                }
                if rng.gen::<f32>() < 0.3 * (cave_x.max(0.5).powf(4.0)) && !ridge_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.shallow_ceiling")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                        |block| block.with_sprite(kind),
                    );
                }
            } else if cave_depth < 200.0 && cave_depth > 80.0 {
                if rng.gen::<f32>() < 0.065 * (cave_x.max(0.5).powf(4.0)) && !vein_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.deep_floor")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                        |block| block.with_sprite(kind),
                    );
                }
                if rng.gen::<f32>() < 0.3 * (cave_x.max(0.5).powf(4.0)) && !ridge_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.deep_ceiling")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                        |block| block.with_sprite(kind),
                    );
                }
            } else {
                if rng.gen::<f32>() < 0.08 * (cave_x.max(0.5).powf(4.0))
                    && cave_depth > 40.0
                    && !vein_condition
                {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.dark_floor")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                        |block| block.with_sprite(kind),
                    );
                }
                if rng.gen::<f32>() < 0.02 * (cave_x.max(0.5).powf(4.0))
                    && !ridge_condition
                    && cave_depth > 40.0
                {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.dark_ceiling")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                        |block| block.with_sprite(kind),
                    );
                }
            };
        }
    });
}
#[allow(clippy::eval_order_dependence)]
pub fn apply_caves_supplement<'a>(
    // NOTE: Used only for dynamic elements like chests and entities!
    dynamic_rng: &mut impl Rng,
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &(impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
    supplement: &mut ChunkSupplement,
) {
    for y in 0..vol.size_xy().y as i32 {
        for x in 0..vol.size_xy().x as i32 {
            let offs = Vec2::new(x, y);

            let wpos2d = wpos2d + offs;

            // Sample terrain
            let col_sample = if let Some(col_sample) = get_column(offs) {
                col_sample
            } else {
                continue;
            };
            let surface_z = col_sample.riverless_alt.floor() as i32;

            if let Some((cave_dist, _, cave, _)) = col_sample
                .cave
                .filter(|(dist, _, cave, _)| *dist < cave.width)
            {
                let cave_x = (cave_dist / cave.width).min(1.0);

                // Relative units
                let cave_floor = 0.0 - 0.5 * (1.0 - cave_x.powi(2)).max(0.0).sqrt() * cave.width;

                // Abs units
                let cave_base = (cave.alt + cave_floor) as i32;

                let cave_depth = (col_sample.alt - cave.alt).max(0.0); //slightly different from earlier cave depth?

                // Scatter things in caves
                if let Some(z) = (-4..8).map(|z| cave_base + z).find(|z| {
                    (0..2).all(|z_offs| {
                        vol.get(offs.with_z(z + z_offs))
                            .map_or(true, |b| b.is_fluid())
                    })
                }) {
                    if RandomField::new(index.seed).chance(wpos2d.into(), 0.0014)
                        && cave_base < surface_z as i32 - 40
                    {
                        let is_hostile: bool;
                        let entity =
                            EntityInfo::at(Vec3::new(wpos2d.x as f32, wpos2d.y as f32, z as f32))
                                .with_body(if cave_depth < 70.0 {
                                    is_hostile = false;
                                    let species = match dynamic_rng.gen_range(0..4) {
                                        0 => comp::quadruped_small::Species::Truffler,
                                        1 => comp::quadruped_small::Species::Dodarock,
                                        2 => comp::quadruped_small::Species::Holladon,
                                        _ => comp::quadruped_small::Species::Batfox,
                                    };
                                    comp::quadruped_small::Body::random_with(dynamic_rng, &species)
                                        .into()
                                } else if cave_depth < 120.0 {
                                    is_hostile = true;
                                    let species = match dynamic_rng.gen_range(0..3) {
                                        0 => comp::quadruped_low::Species::Rocksnapper,
                                        1 => comp::quadruped_low::Species::Salamander,
                                        _ => comp::quadruped_low::Species::Asp,
                                    };
                                    comp::quadruped_low::Body::random_with(dynamic_rng, &species)
                                        .into()
                                } else if cave_depth < 190.0 {
                                    is_hostile = true;
                                    let species = match dynamic_rng.gen_range(0..3) {
                                        0 => comp::quadruped_low::Species::Rocksnapper,
                                        1 => comp::quadruped_low::Species::Lavadrake,
                                        _ => comp::quadruped_low::Species::Basilisk,
                                    };
                                    comp::quadruped_low::Body::random_with(dynamic_rng, &species)
                                        .into()
                                } else {
                                    is_hostile = true;
                                    let species = match dynamic_rng.gen_range(0..5) {
                                        0 => comp::biped_large::Species::Ogre,
                                        1 => comp::biped_large::Species::Cyclops,
                                        2 => comp::biped_large::Species::Wendigo,
                                        3 => match dynamic_rng.gen_range(0..2) {
                                            0 => comp::biped_large::Species::Blueoni,
                                            _ => comp::biped_large::Species::Redoni,
                                        },
                                        _ => comp::biped_large::Species::Cavetroll,
                                    };
                                    comp::biped_large::Body::random_with(dynamic_rng, &species)
                                        .into()
                                })
                                .with_alignment(if is_hostile {
                                    comp::Alignment::Enemy
                                } else {
                                    comp::Alignment::Wild
                                })
                                .with_automatic_name();

                        supplement.add_entity(entity);
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
pub fn apply_coral_to(canvas: &mut Canvas) {
    let info = canvas.info();

    if !info.chunk.river.near_water() {
        return; // Don't bother with coral for a chunk nowhere near water
    }

    canvas.foreach_col(|canvas, wpos2d, col| {
        const CORAL_DEPTH: Range<f32> = 14.0..32.0;
        const CORAL_HEIGHT: f32 = 14.0;
        const CORAL_DEPTH_FADEOUT: f32 = 5.0;
        const CORAL_SCALE: f32 = 10.0;

        let water_depth = col.water_level - col.alt;

        if !CORAL_DEPTH.contains(&water_depth) {
            return; // Avoid coral entirely for this column if we're outside coral depths
        }

        for z in col.alt.floor() as i32..(col.alt + CORAL_HEIGHT) as i32 {
            let wpos = Vec3::new(wpos2d.x, wpos2d.y, z);

            let coral_factor = Lerp::lerp(
                1.0,
                0.0,
                // Fade coral out due to incorrect depth
                ((water_depth.clamped(CORAL_DEPTH.start, CORAL_DEPTH.end) - water_depth).abs()
                    / CORAL_DEPTH_FADEOUT)
                    .min(1.0),
            ) * Lerp::lerp(
                1.0,
                0.0,
                // Fade coral out due to incorrect altitude above the seabed
                ((z as f32 - col.alt) / CORAL_HEIGHT).powi(2),
            ) * FastNoise::new(info.index.seed + 7)
                .get(wpos.map(|e| e as f64) / 32.0)
                .sub(0.2)
                .mul(100.0)
                .clamped(0.0, 1.0);

            let nz = Vec3::iota().map(|e: u32| FastNoise::new(info.index.seed + e * 177));

            let wpos_warped = wpos.map(|e| e as f32)
                + nz.map(|nz| {
                    nz.get(wpos.map(|e| e as f64) / CORAL_SCALE as f64) * CORAL_SCALE * 0.3
                });

            // let is_coral = FastNoise2d::new(info.index.seed + 17)
            //     .get(wpos_warped.xy().map(|e| e as f64) / CORAL_SCALE)
            //     .sub(1.0 - coral_factor)
            //     .max(0.0)
            //     .div(coral_factor) > 0.5;

            let is_coral = [
                FastNoise::new(info.index.seed),
                FastNoise::new(info.index.seed + 177),
            ]
            .iter()
            .all(|nz| {
                nz.get(wpos_warped.map(|e| e as f64) / CORAL_SCALE as f64)
                    .abs()
                    < coral_factor * 0.3
            });

            if is_coral {
                let _ = canvas.set(wpos, Block::new(BlockKind::Rock, Rgb::new(170, 220, 210)));
            }
        }
    });
}
