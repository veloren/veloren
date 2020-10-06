use crate::{column::ColumnSample, sim::SimChunk, util::RandomField, IndexRef, CONFIG};
use common::{
    comp::{quadruped_low, quadruped_medium, quadruped_small, Alignment},
    generation::{ChunkSupplement, EntityInfo},
    terrain::{Block, SpriteKind},
    vol::{BaseVol, ReadVol, RectSizedVol, WriteVol},
};
use noise::NoiseFn;
use rand::prelude::*;
use std::{f32, ops::Range};
use vek::*;

fn close(x: f32, tgt: f32, falloff: f32) -> f32 {
    (1.0 - (x - tgt).abs() / falloff).max(0.0).powf(0.125)
}

const BASE_DENSITY: f32 = 1.0e-5; // Base wildlife density

#[allow(clippy::eval_order_dependence)]
pub fn apply_wildlife_supplement<'a, R: Rng>(
    // NOTE: Used only for dynamic elements like chests and entities!
    dynamic_rng: &mut R,
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &(impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
    chunk: &SimChunk,
    supplement: &mut ChunkSupplement,
) {
    let scatter: &[(
        fn(Vec3<f32>, &mut R) -> EntityInfo, // Entity
        Range<usize>,                        // Group size range
        bool,                                // Underwater?
        fn(&SimChunk, &ColumnSample) -> f32, // Density
    )] = &[
        // Wolves
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(
                        quadruped_medium::Body::random_with(rng, &quadruped_medium::Species::Wolf)
                            .into(),
                    )
                    .with_alignment(Alignment::Enemy)
            },
            3..8,
            false,
            |c, col| close(c.temp, CONFIG.snow_temp, 0.7) * col.tree_density * BASE_DENSITY * 0.3,
        ),
        // Frostfang
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(
                        quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Frostfang,
                        )
                        .into(),
                    )
                    .with_alignment(Alignment::Enemy)
            },
            1..4,
            false,
            |c, col| close(c.temp, CONFIG.snow_temp, 0.15) * BASE_DENSITY * 0.15,
        ),
        // Bonerattler
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(
                        quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Bonerattler,
                        )
                        .into(),
                    )
                    .with_alignment(Alignment::Enemy)
            },
            1..3,
            false,
            |c, col| close(c.humidity, CONFIG.desert_hum, 0.3) * BASE_DENSITY * 0.5,
        ),
        // Forest herd (Deer, Hirdrasil, etc.)
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 10) {
                        0 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Deer,
                        )
                        .into(),
                        1 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Hirdrasil,
                        )
                        .into(),
                        2 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Rabbit,
                        )
                        .into(),
                        3 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Jackalope,
                        )
                        .into(),
                        4 => {
                            quadruped_small::Body::random_with(rng, &quadruped_small::Species::Boar)
                                .into()
                        },
                        5 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Sheep,
                        )
                        .into(),
                        6 => {
                            quadruped_small::Body::random_with(rng, &quadruped_small::Species::Pig)
                                .into()
                        },
                        7 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Truffler,
                        )
                        .into(),
                        8 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Squirrel,
                        )
                        .into(),
                        _ => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Porcupine,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Wild)
            },
            3..8,
            false,
            |c, col| {
                close(c.temp, CONFIG.temperate_temp, 0.7) * col.tree_density * BASE_DENSITY * 1.5
            },
        ),
        // Amphibians (Frog, Axolotl, Crocodile, etc.)
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 6) {
                        0 => {
                            quadruped_small::Body::random_with(rng, &quadruped_small::Species::Frog)
                                .into()
                        },
                        1 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Axolotl,
                        )
                        .into(),
                        2 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Fungome,
                        )
                        .into(),
                        // WE GROW 'EM BIG 'ERE
                        3 => quadruped_low::Body::random_with(
                            rng,
                            &quadruped_low::Species::Crocodile,
                        )
                        .into(),
                        4 => quadruped_low::Body::random_with(
                            rng,
                            &quadruped_low::Species::Alligator,
                        )
                        .into(),
                        _ => quadruped_low::Body::random_with(
                            rng,
                            &quadruped_low::Species::Salamander,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Wild)
            },
            1..3,
            false,
            |c, col| {
                close(col.temp, CONFIG.tropical_temp, 0.8)
                    * if col.water_dist.map(|d| d < 10.0).unwrap_or(false) {
                        0.001
                    } else {
                        0.0
                    }
            },
        ),
    ];

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

            let underwater = col_sample.water_level > col_sample.alt;

            let entity_group = scatter.iter().enumerate().find_map(
                |(i, (make_entity, group_size, is_underwater, f))| {
                    let density = f(chunk, col_sample);
                    if density > 0.0
                        && RandomField::new(i as u32 * 7)
                            .chance(Vec3::new(wpos2d.x, wpos2d.y, i as i32), density)
                        && underwater == *is_underwater
                    {
                        Some((make_entity, group_size.clone()))
                    } else {
                        None
                    }
                },
            );

            if let Some((make_entity, group_size)) = entity_group {
                let alt = col_sample.alt as i32;

                // Find the intersection between ground and air, if there is one near the
                // surface
                if let Some(solid_end) = (-4..8)
                    .find(|z| {
                        vol.get(Vec3::new(offs.x, offs.y, alt + z))
                            .map(|b| b.is_solid())
                            .unwrap_or(false)
                    })
                    .and_then(|solid_start| {
                        (1..8).map(|z| solid_start + z).find(|z| {
                            vol.get(Vec3::new(offs.x, offs.y, alt + z))
                                .map(|b| !b.is_solid())
                                .unwrap_or(true)
                        })
                    })
                {
                    let group_size = dynamic_rng.gen_range(group_size.start, group_size.end);
                    let entity = make_entity(
                        Vec3::new(wpos2d.x, wpos2d.y, alt + solid_end).map(|e| e as f32),
                        dynamic_rng,
                    );
                    for _ in 0..group_size {
                        let mut entity = entity.clone();
                        entity.pos = entity.pos.map(|e| e + dynamic_rng.gen::<f32>());
                        supplement.add_entity(entity.with_automatic_name());
                    }
                }
            }
        }
    }
}
