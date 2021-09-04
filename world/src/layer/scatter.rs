use crate::{column::ColumnSample, sim::SimChunk, Canvas, CONFIG};
use common::terrain::SpriteKind;
use noise::NoiseFn;
use rand::prelude::*;
use std::f32;
use vek::*;

fn close(x: f32, tgt: f32, falloff: f32) -> f32 {
    (1.0 - (x - tgt).abs() / falloff).max(0.0).powf(0.125)
}

const MUSH_FACT: f32 = 1.0e-4; // To balance things around the mushroom spawning rate
const GRASS_FACT: f32 = 1.0e-3; // To balance things around the grass spawning rate
const DEPTH_WATER_NORM: f32 = 15.0; // Water depth at which regular underwater sprites start spawning
pub fn apply_scatter_to(canvas: &mut Canvas, rng: &mut impl Rng) {
    use SpriteKind::*;
    #[allow(clippy::type_complexity)]
    // TODO: Add back all sprites we had before
    let scatter: &[(
        _,
        bool,
        fn(&SimChunk, &ColumnSample) -> (f32, Option<(f32, f32, f32)>),
    )] = &[
        // (density, Option<(base_density_proportion, wavelen, threshold)>)
        // Flowers
        (BlueFlower, false, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.7).min(close(
                    col.humidity,
                    CONFIG.jungle_hum,
                    0.4,
                )) * col.tree_density
                    * MUSH_FACT
                    * 256.0,
                Some((0.0, 256.0, 0.25)),
            )
        }),
        (PinkFlower, false, |_, col| {
            (
                close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((0.0, 100.0, 0.1)),
            )
        }),
        (PurpleFlower, false, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.7).min(close(
                    col.humidity,
                    CONFIG.jungle_hum,
                    0.4,
                )) * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((0.0, 100.0, 0.1)),
            )
        }),
        (RedFlower, false, |_, col| {
            (
                close(col.temp, CONFIG.tropical_temp, 0.7).min(close(
                    col.humidity,
                    CONFIG.jungle_hum,
                    0.4,
                )) * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((0.0, 100.0, 0.1)),
            )
        }),
        (WhiteFlower, false, |_, col| {
            (
                close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((0.0, 100.0, 0.1)),
            )
        }),
        (YellowFlower, false, |_, col| {
            (
                close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((0.0, 100.0, 0.1)),
            )
        }),
        (Cotton, false, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.7).min(close(
                    col.humidity,
                    CONFIG.jungle_hum,
                    0.4,
                )) * col.tree_density
                    * MUSH_FACT
                    * 75.0,
                Some((0.0, 256.0, 0.25)),
            )
        }),
        (Sunflower, false, |_, col| {
            (
                close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((0.0, 100.0, 0.15)),
            )
        }),
        (WildFlax, false, |_, col| {
            (
                close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 600.0,
                Some((0.0, 100.0, 0.15)),
            )
        }),
        // Herbs and Spices
        (LingonBerry, false, |_, col| {
            (
                close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.jungle_hum, 0.5))
                    * MUSH_FACT
                    * 2.5,
                None,
            )
        }),
        (LeafyPlant, false, |_, col| {
            (
                close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.jungle_hum, 0.3))
                    * GRASS_FACT
                    * 4.0,
                None,
            )
        }),
        (Fern, false, |_, col| {
            (
                close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.forest_hum, 0.5))
                    * GRASS_FACT
                    * 0.25,
                Some((0.0, 64.0, 0.2)),
            )
        }),
        (Blueberry, false, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.5).min(close(
                    col.humidity,
                    CONFIG.forest_hum,
                    0.5,
                )) * MUSH_FACT
                    * 0.3,
                None,
            )
        }),
        (Pumpkin, false, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.5).min(close(
                    col.humidity,
                    CONFIG.forest_hum,
                    0.5,
                )) * MUSH_FACT
                    * 500.0,
                Some((0.0, 512.0, 0.05)),
            )
        }),
        // Collectable Objects
        // Only spawn twigs in temperate forests
        (Twigs, false, |_, col| {
            (
                (col.tree_density * 1.25 - 0.25).powf(0.5).max(0.0) * 0.75e-3,
                None,
            )
        }),
        (Stones, false, |chunk, _| {
            ((chunk.rockiness - 0.5).max(0.025) * 1.0e-3, None)
        }),
        (Copper, false, |chunk, _| {
            ((chunk.rockiness - 0.5).max(0.0) * 1.5e-3, None)
        }),
        (Tin, false, |chunk, _| {
            ((chunk.rockiness - 0.5).max(0.0) * 1.5e-3, None)
        }),
        // Don't spawn Mushrooms in snowy regions
        (Mushroom, false, |_, col| {
            (
                close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.forest_hum, 0.35))
                    * MUSH_FACT,
                None,
            )
        }),
        // Grass
        (ShortGrass, false, |_, col| {
            (
                close(col.temp, 0.2, 0.75).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                    * GRASS_FACT
                    * 150.0,
                Some((0.3, 64.0, 0.3)),
            )
        }),
        (MediumGrass, false, |_, col| {
            (
                close(col.temp, 0.2, 0.6).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                    * GRASS_FACT
                    * 120.0,
                Some((0.3, 64.0, 0.3)),
            )
        }),
        (LongGrass, false, |_, col| {
            (
                close(col.temp, 0.3, 0.35).min(close(col.humidity, CONFIG.jungle_hum, 0.3))
                    * GRASS_FACT
                    * 150.0,
                Some((0.1, 48.0, 0.3)),
            )
        }),
        // Jungle Sprites
        // (LongGrass, false, |c, col| {
        //     (
        //         close(col.temp, CONFIG.tropical_temp, 0.4).min(close(
        //             col.humidity,
        //             CONFIG.jungle_hum,
        //             0.6,
        //         )) * 0.08,
        //         Some((0.0, 60.0, 5.0)),
        //     )
        // }),
        /*(WheatGreen, false, |c, col| {
            (
                close(col.temp, 0.4, 0.2).min(close(col.humidity, CONFIG.forest_hum, 0.1))
                    * MUSH_FACT
                    * 0.001,
                None,
            )
        }),*/
        (GrassSnow, false, |_, col| {
            (
                close(col.temp, CONFIG.snow_temp - 0.2, 0.4).min(close(
                    col.humidity,
                    CONFIG.forest_hum,
                    0.5,
                )) * GRASS_FACT
                    * 100.0,
                Some((0.0, 48.0, 0.2)),
            )
        }),
        (Moonbell, false, |_, col| {
            (
                close(col.temp, CONFIG.snow_temp - 0.2, 0.4).min(close(
                    col.humidity,
                    CONFIG.forest_hum,
                    0.5,
                )) * 0.003,
                Some((0.0, 48.0, 0.2)),
            )
        }),
        // Savanna Plants
        (SavannaGrass, false, |_, col| {
            (
                {
                    let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                    let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                    (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 250.0
                },
                Some((0.15, 64.0, 0.2)),
            )
        }),
        (TallSavannaGrass, false, |_, col| {
            (
                {
                    let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                    let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                    (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 150.0
                },
                Some((0.1, 48.0, 0.2)),
            )
        }),
        (RedSavannaGrass, false, |_, col| {
            (
                {
                    let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                    let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                    (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 120.0
                },
                Some((0.15, 48.0, 0.25)),
            )
        }),
        (SavannaBush, false, |_, col| {
            (
                {
                    let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                    let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                    (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 40.0
                },
                Some((0.1, 96.0, 0.15)),
            )
        }),
        // Desert Plants
        (DeadBush, false, |_, col| {
            (
                close(col.temp, 1.0, 0.95).min(close(col.humidity, 0.0, 0.3)) * MUSH_FACT * 7.5,
                None,
            )
        }),
        (Pyrebloom, false, |_, col| {
            (
                close(col.temp, 1.0, 0.95).min(close(col.humidity, 0.0, 0.3)) * MUSH_FACT * 0.35,
                None,
            )
        }),
        (LargeCactus, false, |_, col| {
            (
                close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 1.5,
                None,
            )
        }),
        (RoundCactus, false, |_, col| {
            (
                close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                None,
            )
        }),
        (ShortCactus, false, |_, col| {
            (
                close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                None,
            )
        }),
        (MedFlatCactus, false, |_, col| {
            (
                close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                None,
            )
        }),
        (ShortFlatCactus, false, |_, col| {
            (
                close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                None,
            )
        }),
        (Reed, false, |_, col| {
            (
                close(col.humidity, CONFIG.jungle_hum, 0.9)
                    * col
                        .water_dist
                        .map(|wd| Lerp::lerp(0.2, 0.0, (wd / 8.0).clamped(0.0, 1.0)))
                        .unwrap_or(0.0)
                    * ((col.alt - CONFIG.sea_level) / 12.0).clamped(0.0, 1.0),
                Some((0.2, 128.0, 0.5)),
            )
        }),
        // Underwater chests
        (ChestBuried, true, |_, col| {
            (
                MUSH_FACT
                    * 1.0e-6
                    * if col.alt < col.water_level - DEPTH_WATER_NORM + 30.0 {
                        1.0
                    } else {
                        0.0
                    },
                None,
            )
        }),
        // Underwater mud piles
        (Mud, true, |_, col| {
            (
                MUSH_FACT
                    * 1.0e-3
                    * if col.alt < col.water_level - DEPTH_WATER_NORM {
                        1.0
                    } else {
                        0.0
                    },
                None,
            )
        }),
        // Underwater grass
        (GrassBlue, true, |_, col| {
            (
                MUSH_FACT
                    * 250.0
                    * if col.alt < col.water_level - DEPTH_WATER_NORM {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 100.0, 0.15)),
            )
        }),
        // seagrass
        (Seagrass, true, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.8)
                    * MUSH_FACT
                    * 300.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 18.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 150.0, 0.3)),
            )
        }),
        // seagrass, coastal patches
        (Seagrass, true, |_, col| {
            (
                MUSH_FACT
                    * 600.0
                    * if col.water_level < CONFIG.sea_level && (col.water_level - col.alt) < 3.0 {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 150.0, 0.4)),
            )
        }),
        // scattered seaweed (temperate species)
        (SeaweedTemperate, true, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.8)
                    * MUSH_FACT
                    * 50.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 11.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 500.0, 0.75)),
            )
        }),
        // scattered seaweed (tropical species)
        (SeaweedTropical, true, |_, col| {
            (
                close(col.temp, 1.0, 0.95)
                    * MUSH_FACT
                    * 50.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 11.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 500.0, 0.75)),
            )
        }),
        // Caulerpa lentillifera algae patch
        (SeaGrapes, true, |_, col| {
            (
                MUSH_FACT
                    * 250.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 100.0, 0.15)),
            )
        }),
        // Caulerpa prolifera algae patch
        (WavyAlgae, true, |_, col| {
            (
                MUSH_FACT
                    * 250.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 100.0, 0.15)),
            )
        }),
        // Mermaids' fan algae patch
        (MermaidsFan, true, |_, col| {
            (
                close(col.temp, 1.0, 0.95)
                    * MUSH_FACT
                    * 500.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 50.0, 0.10)),
            )
        }),
        // Sea anemones
        (SeaAnemone, true, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.8)
                    * MUSH_FACT
                    * 125.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM - 9.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 100.0, 0.3)),
            )
        }),
        // Giant Kelp
        (GiantKelp, true, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.8)
                    * MUSH_FACT
                    * 220.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM - 9.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 200.0, 0.4)),
            )
        }),
        // Bull Kelp
        (BullKelp, true, |_, col| {
            (
                close(col.temp, CONFIG.temperate_temp, 0.7)
                    * MUSH_FACT
                    * 300.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 3.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 75.0, 0.3)),
            )
        }),
        // Stony Corals
        (StonyCoral, true, |_, col| {
            (
                close(col.temp, 1.0, 0.9)
                    * MUSH_FACT
                    * 160.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 120.0, 0.4)),
            )
        }),
        // Soft Corals
        (SoftCoral, true, |_, col| {
            (
                close(col.temp, 1.0, 0.9)
                    * MUSH_FACT
                    * 120.0
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                Some((0.0, 120.0, 0.4)),
            )
        }),
        // Seashells
        (Seashells, true, |c, col| {
            (
                (c.rockiness - 0.5).max(0.0)
                    * 1.0e-3
                    * if col.water_level < CONFIG.sea_level
                        && col.alt < col.water_level - DEPTH_WATER_NORM + 20.0
                    {
                        1.0
                    } else {
                        0.0
                    },
                None,
            )
        }),
        (Stones, true, |c, col| {
            (
                (c.rockiness - 0.5).max(0.0)
                    * 1.0e-3
                    * if col.alt < col.water_level - DEPTH_WATER_NORM {
                        1.0
                    } else {
                        0.0
                    },
                None,
            )
        }),
    ];

    canvas.foreach_col(|canvas, wpos2d, col| {
        // TODO: Why do we need to add 1.0 here? Idk...
        let underwater = col.alt.floor() + 1.0 < col.water_level.floor();

        let kind = scatter
            .iter()
            .enumerate()
            .find_map(|(i, (kind, is_underwater, f))| {
                let (density, patch) = f(canvas.chunk(), col);
                let density = patch
                    .map(|(base_density_prop, wavelen, threshold)| {
                        if canvas
                            .index()
                            .noise
                            .scatter_nz
                            .get(
                                wpos2d
                                    .map(|e| e as f64 / wavelen as f64 + i as f64 * 43.0)
                                    .into_array(),
                            )
                            .abs()
                            > 1.0 - threshold as f64
                        {
                            density
                        } else {
                            density * base_density_prop
                        }
                    })
                    .unwrap_or(density);
                if density > 0.0
                    && rng.gen::<f32>() < density //RandomField::new(i as u32).chance(Vec3::new(wpos2d.x, wpos2d.y, 0), density)
                    && underwater == *is_underwater
                {
                    Some(*kind)
                } else {
                    None
                }
            });

        if let Some(kind) = kind {
            let alt = col.alt as i32;

            // Find the intersection between ground and air, if there is one near the
            // surface
            if let Some(solid_end) = (-4..8)
                .find(|z| {
                    canvas
                        .get(Vec3::new(wpos2d.x, wpos2d.y, alt + z))
                        .is_solid()
                })
                .and_then(|solid_start| {
                    (1..8).map(|z| solid_start + z).find(|z| {
                        !canvas
                            .get(Vec3::new(wpos2d.x, wpos2d.y, alt + z))
                            .is_solid()
                    })
                })
            {
                canvas.map(Vec3::new(wpos2d.x, wpos2d.y, alt + solid_end), |block| {
                    block.with_sprite(kind)
                });
            }
        }
    });
}
