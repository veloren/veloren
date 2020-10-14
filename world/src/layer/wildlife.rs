use crate::{column::ColumnSample, sim::SimChunk, util::RandomField, IndexRef, CONFIG};
use common::{
    comp::{biped_large, bird_medium, quadruped_low, quadruped_medium, quadruped_small, Alignment},
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
        // Taiga pack ennemies
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
            |c, col| {
                close(c.temp, CONFIG.snow_temp + 0.2, 0.7) * col.tree_density * BASE_DENSITY * 0.05
            },
        ),
        // Taiga pack wild
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(
                        quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Mouflon,
                        )
                        .into(),
                    )
                    .with_alignment(Alignment::Wild)
            },
            1..4,
            false,
            |c, col| close(c.temp, CONFIG.snow_temp + 0.2, 0.2) * BASE_DENSITY * 0.1,
        ),
        // Taiga solitary wild
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 4) {
                        0 => {
                            bird_medium::Body::random_with(rng, &bird_medium::Species::Eagle).into()
                        },
                        1 => quadruped_low::Body::random_with(rng, &quadruped_low::Species::Asp)
                            .into(),
                        2 => bird_medium::Body::random_with(rng, &bird_medium::Species::Snowyowl)
                            .into(),
                        _ => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Tuskram,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Enemy)
            },
            1..2,
            false,
            |c, col| close(c.temp, CONFIG.snow_temp + 0.2, 0.7) * BASE_DENSITY * 0.3,
        ),
        // Tundra pack ennemies
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 2) {
                        0 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Frostfang,
                        )
                        .into(),
                        _ => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Grolgar,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Enemy)
            },
            1..4,
            false,
            |c, col| close(c.temp, CONFIG.snow_temp, 0.15) * BASE_DENSITY * 0.2,
        ),
        // Tundra solitary ennemies
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(
                        biped_large::Body::random_with(rng, &biped_large::Species::Wendigo).into(),
                    )
                    .with_alignment(Alignment::Enemy)
            },
            1..2,
            false,
            |c, col| close(c.temp, CONFIG.snow_temp, 0.15) * BASE_DENSITY * 0.1,
        ),
        // Tundra solitary wild
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(
                        quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Tarasque,
                        )
                        .into(),
                    )
                    .with_alignment(Alignment::Wild)
            },
            1..2,
            false,
            |c, col| close(c.temp, CONFIG.temperate_temp, 0.15) * BASE_DENSITY * 0.1,
        ),
        // Forest pack wild
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 10) {
                        0 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Deer,
                        )
                        .into(),
                        1 => {
                            quadruped_small::Body::random_with(rng, &quadruped_small::Species::Rat)
                                .into()
                        },
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
                        _ => bird_medium::Body::random_with(rng, &bird_medium::Species::Chicken)
                            .into(),
                    })
                    .with_alignment(Alignment::Wild)
            },
            1..8,
            false,
            |c, col| {
                close(c.temp, CONFIG.temperate_temp, 0.7) * col.tree_density * BASE_DENSITY * 6.0
            },
        ),
        // Temperate solitary wild
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 12) {
                        0 => {
                            quadruped_small::Body::random_with(rng, &quadruped_small::Species::Fox)
                                .into()
                        },
                        1 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Quokka,
                        )
                        .into(),
                        2 => {
                            bird_medium::Body::random_with(rng, &bird_medium::Species::Goose).into()
                        },
                        3 => bird_medium::Body::random_with(rng, &bird_medium::Species::Peacock)
                            .into(),
                        4 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Porcupine,
                        )
                        .into(),
                        5 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Skunk,
                        )
                        .into(),
                        6 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Raccoon,
                        )
                        .into(),
                        7 => bird_medium::Body::random_with(rng, &bird_medium::Species::Cockatrice)
                            .into(),
                        8 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Catoblepas,
                        )
                        .into(),
                        9 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Turtle,
                        )
                        .into(),
                        10 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Hirdrasil,
                        )
                        .into(),
                        _ => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Batfox,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Wild)
            },
            1..2,
            false,
            |c, col| close(c.temp, CONFIG.temperate_temp, 0.15) * BASE_DENSITY * 10.0,
        ),
        // Rare temperate solitary enemies
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 12) {
                        0 => {
                            biped_large::Body::random_with(rng, &biped_large::Species::Ogre).into()
                        },
                        1 => {
                            biped_large::Body::random_with(rng, &biped_large::Species::Troll).into()
                        },
                        2 => biped_large::Body::random_with(rng, &biped_large::Species::Dullahan)
                            .into(),
                        3 => biped_large::Body::random_with(rng, &biped_large::Species::Cyclops)
                            .into(),
                        _ => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Batfox,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Wild)
            },
            1..2,
            false,
            |c, col| close(c.temp, CONFIG.temperate_temp, 0.8) * BASE_DENSITY * 0.3,
        ),
        // Temperate rare river wildlife
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
                close(col.temp, CONFIG.tropical_temp, 0.3)
                    * if col.water_dist.map(|d| d < 10.0).unwrap_or(false) {
                        0.001
                    } else {
                        0.0
                    }
            },
        ),
        // Temperate common river wildlife
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 3) {
                        0 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Beaver,
                        )
                        .into(),
                        _ => {
                            bird_medium::Body::random_with(rng, &bird_medium::Species::Duck).into()
                        },
                    })
                    .with_alignment(Alignment::Wild)
            },
            1..3,
            false,
            |c, col| {
                close(col.temp, CONFIG.temperate_temp, 0.6)
                    * if col.water_dist.map(|d| d < 10.0).unwrap_or(false) {
                        0.001
                    } else {
                        0.0
                    }
            },
        ),
        // Tropical rock solitary ennemies
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 8) {
                        0 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Dodarock,
                        )
                        .into(),
                        _ => quadruped_low::Body::random_with(
                            rng,
                            &quadruped_low::Species::Rocksnapper,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Wild)
            },
            1..2,
            false,
            |c, col| close(c.temp, CONFIG.tropical_temp, 0.3) * col.rock * BASE_DENSITY * 5.0,
        ),
        // Jungle solitary ennemies
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 3) {
                        0 => {
                            quadruped_low::Body::random_with(rng, &quadruped_low::Species::Maneater)
                                .into()
                        },
                        1 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Tiger,
                        )
                        .into(),
                        _ => quadruped_low::Body::random_with(
                            rng,
                            &quadruped_low::Species::Rocksnapper,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Enemy)
            },
            1..2,
            false,
            |c, col| {
                close(c.temp, CONFIG.tropical_temp, 0.3)
                    * close(c.humidity, CONFIG.jungle_hum, 0.3)
                    * BASE_DENSITY
                    * 5.0
            },
        ),
        // Jungle solitary wild
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 4) {
                        0 => bird_medium::Body::random_with(rng, &bird_medium::Species::Parrot)
                            .into(),
                        1 => {
                            quadruped_low::Body::random_with(rng, &quadruped_low::Species::Monitor)
                                .into()
                        },
                        _ => {
                            quadruped_low::Body::random_with(rng, &quadruped_low::Species::Tortoise)
                                .into()
                        },
                    })
                    .with_alignment(Alignment::Wild)
            },
            1..2,
            false,
            |c, col| {
                close(c.temp, CONFIG.tropical_temp, 0.3)
                    * close(c.humidity, CONFIG.jungle_hum, 0.3)
                    * BASE_DENSITY
                    * 5.0
            },
        ),
        // Tropical pack enemies
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 3) {
                        0 => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Lion,
                        )
                        .into(),
                        1 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Hyena,
                        )
                        .into(),
                        _ => quadruped_medium::Body::random_with(
                            rng,
                            &quadruped_medium::Species::Saber,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Enemy)
            },
            1..3,
            false,
            |c, col| close(c.temp, CONFIG.tropical_temp, 0.15) * BASE_DENSITY * 0.4,
        ),
        // Desert solitary enemies
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
            1..2,
            false,
            |c, col| close(c.humidity, CONFIG.desert_hum, 0.3) * BASE_DENSITY * 0.3,
        ),
        // Desert solitary wild
        (
            |pos, rng| {
                EntityInfo::at(pos)
                    .with_body(match rng.gen_range(0, 3) {
                        0 => {
                            bird_medium::Body::random_with(rng, &bird_medium::Species::Eagle).into()
                        },
                        1 => {
                            quadruped_low::Body::random_with(rng, &quadruped_low::Species::Pangolin)
                                .into()
                        },
                        2 => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Holladon,
                        )
                        .into(),
                        _ => quadruped_small::Body::random_with(
                            rng,
                            &quadruped_small::Species::Gecko,
                        )
                        .into(),
                    })
                    .with_alignment(Alignment::Enemy)
            },
            1..2,
            false,
            |c, col| close(c.temp, CONFIG.desert_temp + 0.2, 0.7) * BASE_DENSITY * 0.4,
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
