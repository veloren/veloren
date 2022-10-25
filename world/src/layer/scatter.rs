use crate::{column::ColumnSample, sim::SimChunk, Canvas, CONFIG};
use common::{
    calendar::{Calendar, CalendarEvent},
    terrain::{Block, BlockKind, SpriteKind},
};
use noise::NoiseFn;
use num::traits::Pow;
use rand::prelude::*;
use std::f32;
use vek::*;

pub fn close(x: f32, tgt: f32, falloff: f32) -> f32 {
    (1.0 - (x - tgt).abs() / falloff).max(0.0).powf(0.125)
}

/// Returns a decimal value between 0 and 1.  
/// The density is maximum at the middle of the highest and the lowest allowed
/// altitudes, and zero otherwise. Quadratic curve.
///
/// The formula used is:
///
/// ```latex
/// \max\left(-\frac{4\left(x-u\right)\left(x-l\right)}{\left(u-l\right)^{2}},\ 0\right)
/// ```
pub fn density_factor_by_altitude(lower_limit: f32, altitude: f32, upper_limit: f32) -> f32 {
    let maximum: f32 = (upper_limit - lower_limit).pow(2) / 4.0f32;
    (-((altitude - lower_limit) * (altitude - upper_limit)) / maximum).max(0.0)
}

const MUSH_FACT: f32 = 1.0e-4; // To balance things around the mushroom spawning rate
const GRASS_FACT: f32 = 1.0e-3; // To balance things around the grass spawning rate
const DEPTH_WATER_NORM: f32 = 15.0; // Water depth at which regular underwater sprites start spawning
pub fn apply_scatter_to(canvas: &mut Canvas, rng: &mut impl Rng, calendar: Option<&Calendar>) {
    enum WaterMode {
        Underwater,
        Floating,
        Ground,
    }
    use WaterMode::*;

    use SpriteKind::*;

    struct ScatterConfig {
        kind: SpriteKind,
        water_mode: WaterMode,
        permit: fn(BlockKind) -> bool,
        f: fn(&SimChunk, &ColumnSample) -> (f32, Option<(f32, f32, f32)>),
    }

    // TODO: Add back all sprites we had before
    let scatter: &[ScatterConfig] = &[
        // (density, Option<(base_density_proportion, wavelen, threshold)>)
        // Flowers
        ScatterConfig {
            kind: BlueFlower,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
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
            },
        },
        ScatterConfig {
            kind: PinkFlower,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * col.tree_density
                        * MUSH_FACT
                        * 350.0,
                    Some((0.0, 100.0, 0.1)),
                )
            },
        },
        ScatterConfig {
            kind: PurpleFlower,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
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
            },
        },
        ScatterConfig {
            kind: RedFlower,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
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
            },
        },
        ScatterConfig {
            kind: WhiteFlower,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * col.tree_density
                        * MUSH_FACT
                        * 350.0,
                    Some((0.0, 100.0, 0.1)),
                )
            },
        },
        ScatterConfig {
            kind: YellowFlower,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * col.tree_density
                        * MUSH_FACT
                        * 350.0,
                    Some((0.0, 100.0, 0.1)),
                )
            },
        },
        ScatterConfig {
            kind: Cotton,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Earth | BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.tropical_temp, 0.7).min(close(
                        col.humidity,
                        CONFIG.jungle_hum,
                        0.4,
                    )) * col.tree_density
                    * MUSH_FACT
                    * 200.0
                    * (!col.snow_cover) as i32 as f32 /* To prevent spawning in snow covered areas */
                    * density_factor_by_altitude(-500.0 , col.alt, 500.0), /* To prevent
                                                                            * spawning at high
                                                                            * altitudes */
                    Some((0.0, 128.0, 0.30)),
                )
            },
        },
        ScatterConfig {
            kind: Sunflower,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.0, 0.7).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * col.tree_density
                        * MUSH_FACT
                        * 350.0,
                    Some((0.0, 100.0, 0.15)),
                )
            },
        },
        ScatterConfig {
            kind: WildFlax,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.temperate_temp, 0.7).min(close(
                        col.humidity,
                        CONFIG.forest_hum,
                        0.4,
                    )) * col.tree_density
                        * MUSH_FACT
                        * 600.0
                        * density_factor_by_altitude(200.0, col.alt, 1000.0), /* To control
                                                                               * spawning based
                                                                               * on altitude */
                    Some((0.0, 100.0, 0.15)),
                )
            },
        },
        // Herbs and Spices
        ScatterConfig {
            kind: LingonBerry,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.jungle_hum, 0.5))
                        * MUSH_FACT
                        * 2.5,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: LeafyPlant,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.jungle_hum, 0.3))
                        * GRASS_FACT
                        * 4.0,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: JungleLeafyPlant,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * GRASS_FACT
                        * 32.0,
                    Some((0.15, 64.0, 0.2)),
                )
            },
        },
        ScatterConfig {
            kind: Fern,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.forest_hum, 0.5))
                        * GRASS_FACT
                        * 0.25,
                    Some((0.0, 64.0, 0.2)),
                )
            },
        },
        ScatterConfig {
            kind: JungleFern,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * col.tree_density
                        * MUSH_FACT
                        * 200.0,
                    Some((0.0, 84.0, 0.35)),
                )
            },
        },
        ScatterConfig {
            kind: Blueberry,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.temperate_temp, 0.5).min(close(
                        col.humidity,
                        CONFIG.forest_hum,
                        0.5,
                    )) * MUSH_FACT
                        * 0.3,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: Pumpkin,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: if calendar.map_or(false, |calendar| {
                calendar.is_event(CalendarEvent::Halloween)
            }) {
                |_, _| (0.1, Some((0.0003, 128.0, 0.1)))
            } else {
                |_, col| {
                    (
                        close(col.temp, CONFIG.temperate_temp, 0.5).min(close(
                            col.humidity,
                            CONFIG.forest_hum,
                            0.5,
                        )) * MUSH_FACT
                            * 500.0,
                        Some((0.0, 512.0, 0.05)),
                    )
                }
            },
        },
        // Collectable Objects
        // Only spawn twigs in temperate forests
        ScatterConfig {
            kind: Twigs,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    (col.tree_density * 1.25 - 0.25).powf(0.5).max(0.0) * 0.75e-3,
                    None,
                )
            },
        },
        // Only spawn logs in temperate forests (arbitrarily set to ~20% twig density)
        ScatterConfig {
            kind: Wood,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    (col.tree_density * 1.25 - 0.25).powf(0.5).max(0.0) * 0.15e-3,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: Stones,
            water_mode: Ground,
            permit: |b| {
                matches!(
                    b,
                    BlockKind::Earth | BlockKind::Grass | BlockKind::Rock | BlockKind::Sand
                )
            },
            f: |chunk, _| ((chunk.rockiness - 0.5).max(0.025) * 1.0e-3, None),
        },
        ScatterConfig {
            kind: Copper,
            water_mode: Ground,
            permit: |b| {
                matches!(
                    b,
                    BlockKind::Earth | BlockKind::Grass | BlockKind::Rock | BlockKind::Sand
                )
            },
            f: |chunk, _| ((chunk.rockiness - 0.5).max(0.0) * 1.5e-3, None),
        },
        ScatterConfig {
            kind: Tin,
            water_mode: Ground,
            permit: |b| {
                matches!(
                    b,
                    BlockKind::Earth | BlockKind::Grass | BlockKind::Rock | BlockKind::Sand
                )
            },
            f: |chunk, _| ((chunk.rockiness - 0.5).max(0.0) * 1.5e-3, None),
        },
        // Don't spawn Mushrooms in snowy regions
        ScatterConfig {
            kind: Mushroom,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.forest_hum, 0.35))
                        * MUSH_FACT,
                    None,
                )
            },
        },
        // Grass
        ScatterConfig {
            kind: ShortGrass,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.2, 0.75).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * GRASS_FACT
                        * 150.0,
                    Some((0.3, 64.0, 0.3)),
                )
            },
        },
        ScatterConfig {
            kind: MediumGrass,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.2, 0.6).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * GRASS_FACT
                        * 120.0,
                    Some((0.3, 64.0, 0.3)),
                )
            },
        },
        ScatterConfig {
            kind: LongGrass,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.35).min(close(col.humidity, CONFIG.jungle_hum, 0.3))
                        * GRASS_FACT
                        * 150.0,
                    Some((0.1, 48.0, 0.3)),
                )
            },
        },
        ScatterConfig {
            kind: JungleRedGrass,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.3, 0.4).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * col.tree_density
                        * MUSH_FACT
                        * 350.0,
                    Some((0.0, 128.0, 0.25)),
                )
            },
        },
        // Jungle Sprites
        // (LongGrass, Ground, |c, col| {
        //     (
        //         close(col.temp, CONFIG.tropical_temp, 0.4).min(close(
        //             col.humidity,
        //             CONFIG.jungle_hum,
        //             0.6,
        //         )) * 0.08,
        //         Some((0.0, 60.0, 5.0)),
        //     )
        // }),
        /*(WheatGreen, Ground, |c, col| {
            (
                close(col.temp, 0.4, 0.2).min(close(col.humidity, CONFIG.forest_hum, 0.1))
                    * MUSH_FACT
                    * 0.001,
                None,
            )
        }),*/
        ScatterConfig {
            kind: GrassSnow,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.snow_temp - 0.2, 0.4).min(close(
                        col.humidity,
                        CONFIG.forest_hum,
                        0.5,
                    )) * GRASS_FACT
                        * 100.0,
                    Some((0.0, 48.0, 0.2)),
                )
            },
        },
        ScatterConfig {
            kind: Moonbell,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.snow_temp - 0.2, 0.4).min(close(
                        col.humidity,
                        CONFIG.forest_hum,
                        0.5,
                    )) * 0.003,
                    Some((0.0, 48.0, 0.2)),
                )
            },
        },
        // Savanna Plants
        ScatterConfig {
            kind: SavannaGrass,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    {
                        let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                        let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                        (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 250.0
                    },
                    Some((0.15, 64.0, 0.2)),
                )
            },
        },
        ScatterConfig {
            kind: TallSavannaGrass,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    {
                        let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                        let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                        (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 150.0
                    },
                    Some((0.1, 48.0, 0.2)),
                )
            },
        },
        ScatterConfig {
            kind: RedSavannaGrass,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    {
                        let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                        let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                        (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 120.0
                    },
                    Some((0.15, 48.0, 0.25)),
                )
            },
        },
        ScatterConfig {
            kind: SavannaBush,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    {
                        let savanna = close(col.temp, 1.0, 0.4) * close(col.humidity, 0.2, 0.25);
                        let desert = close(col.temp, 1.0, 0.25) * close(col.humidity, 0.0, 0.1);
                        (savanna - desert * 5.0).max(0.0) * GRASS_FACT * 40.0
                    },
                    Some((0.1, 96.0, 0.15)),
                )
            },
        },
        // Desert Plants
        ScatterConfig {
            kind: DeadBush,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.95).min(close(col.humidity, 0.0, 0.3)) * MUSH_FACT * 7.5,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: Pyrebloom,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.95).min(close(col.humidity, 0.0, 0.3))
                        * MUSH_FACT
                        * 0.35,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: LargeCactus,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 1.5,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: RoundCactus,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: ShortCactus,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: MedFlatCactus,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                    None,
                )
            },
        },
        ScatterConfig {
            kind: ShortFlatCactus,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.25).min(close(col.humidity, 0.0, 0.1)) * MUSH_FACT * 2.5,
                    None,
                )
            },
        },
        // Underwater chests
        ScatterConfig {
            kind: ChestBuried,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth | BlockKind::Sand),
            f: |_, col| {
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
            },
        },
        // Underwater mud piles
        ScatterConfig {
            kind: Mud,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
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
            },
        },
        // Underwater grass
        ScatterConfig {
            kind: GrassBlue,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
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
            },
        },
        // seagrass
        ScatterConfig {
            kind: Seagrass,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.temperate_temp, 0.8)
                        * MUSH_FACT
                        * 300.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 18.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 150.0, 0.3)),
                )
            },
        },
        // seagrass, coastal patches
        ScatterConfig {
            kind: Seagrass,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    MUSH_FACT
                        * 600.0
                        * if col.water_level <= CONFIG.sea_level
                            && (col.water_level - col.alt) < 3.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 150.0, 0.4)),
                )
            },
        },
        // scattered seaweed (temperate species)
        ScatterConfig {
            kind: SeaweedTemperate,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.temperate_temp, 0.8)
                        * MUSH_FACT
                        * 50.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 11.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 500.0, 0.75)),
                )
            },
        },
        // scattered seaweed (tropical species)
        ScatterConfig {
            kind: SeaweedTropical,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.95)
                        * MUSH_FACT
                        * 50.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 11.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 500.0, 0.75)),
                )
            },
        },
        // Caulerpa lentillifera algae patch
        ScatterConfig {
            kind: SeaGrapes,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    MUSH_FACT
                        * 250.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 100.0, 0.15)),
                )
            },
        },
        // Caulerpa prolifera algae patch
        ScatterConfig {
            kind: WavyAlgae,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    MUSH_FACT
                        * 250.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 100.0, 0.15)),
                )
            },
        },
        // Mermaids' fan algae patch
        ScatterConfig {
            kind: MermaidsFan,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.95)
                        * MUSH_FACT
                        * 500.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 50.0, 0.10)),
                )
            },
        },
        // Sea anemones
        ScatterConfig {
            kind: SeaAnemone,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.temperate_temp, 0.8)
                        * MUSH_FACT
                        * 125.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM - 9.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 100.0, 0.3)),
                )
            },
        },
        // Giant Kelp
        ScatterConfig {
            kind: GiantKelp,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.temperate_temp, 0.8)
                        * MUSH_FACT
                        * 220.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM - 9.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 200.0, 0.4)),
                )
            },
        },
        // Bull Kelp
        ScatterConfig {
            kind: BullKelp,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    close(col.temp, CONFIG.temperate_temp, 0.7)
                        * MUSH_FACT
                        * 300.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 3.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 75.0, 0.3)),
                )
            },
        },
        // Stony Corals
        ScatterConfig {
            kind: StonyCoral,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.9)
                        * MUSH_FACT
                        * 160.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 120.0, 0.4)),
                )
            },
        },
        // Soft Corals
        ScatterConfig {
            kind: SoftCoral,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |_, col| {
                (
                    close(col.temp, 1.0, 0.9)
                        * MUSH_FACT
                        * 120.0
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 10.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    Some((0.0, 120.0, 0.4)),
                )
            },
        },
        // Seashells
        ScatterConfig {
            kind: Seashells,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |c, col| {
                (
                    (c.rockiness - 0.5).max(0.0)
                        * 1.0e-3
                        * if col.water_level <= CONFIG.sea_level
                            && col.alt < col.water_level - DEPTH_WATER_NORM + 20.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    None,
                )
            },
        },
        ScatterConfig {
            kind: Stones,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Earth),
            f: |c, col| {
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
            },
        },
        //River-related scatter
        ScatterConfig {
            kind: LillyPads,
            water_mode: Floating,
            permit: |_| true,
            f: |_, col| {
                (
                    close(col.temp, 0.2, 0.6).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * GRASS_FACT
                        * 100.0
                        * ((col.alt - CONFIG.sea_level) / 12.0).clamped(0.0, 1.0)
                        * col
                            .water_dist
                            .map_or(0.0, |d| 1.0 / (1.0 + (d.abs() * 0.4).powi(2))),
                    Some((0.0, 128.0, 0.35)),
                )
            },
        },
        ScatterConfig {
            kind: Reed,
            water_mode: Underwater,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.temp, 0.2, 0.6).min(close(col.humidity, CONFIG.jungle_hum, 0.4))
                        * GRASS_FACT
                        * 100.0
                        * ((col.alt - CONFIG.sea_level) / 12.0).clamped(0.0, 1.0)
                        * col
                            .water_dist
                            .map_or(0.0, |d| 1.0 / (1.0 + (d.abs() * 0.40).powi(2))),
                    Some((0.2, 128.0, 0.5)),
                )
            },
        },
        ScatterConfig {
            kind: Reed,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    close(col.humidity, CONFIG.jungle_hum, 0.9)
                        * col
                            .water_dist
                            .map(|wd| Lerp::lerp(0.2, 0.0, (wd / 8.0).clamped(0.0, 1.0)))
                            .unwrap_or(0.0)
                        * ((col.alt - CONFIG.sea_level) / 12.0).clamped(0.0, 1.0),
                    Some((0.2, 128.0, 0.5)),
                )
            },
        },
        ScatterConfig {
            kind: Bamboo,
            water_mode: Ground,
            permit: |b| matches!(b, BlockKind::Grass),
            f: |_, col| {
                (
                    0.014
                        * close(col.humidity, CONFIG.jungle_hum, 0.9)
                        * col
                            .water_dist
                            .map(|wd| Lerp::lerp(0.2, 0.0, (wd / 8.0).clamped(0.0, 1.0)))
                            .unwrap_or(0.0)
                        * ((col.alt - CONFIG.sea_level) / 12.0).clamped(0.0, 1.0),
                    Some((0.2, 128.0, 0.5)),
                )
            },
        },
    ];

    canvas.foreach_col(|canvas, wpos2d, col| {
        let underwater = col.water_level.floor() > col.alt;

        let kind = scatter.iter().enumerate().find_map(
            |(
                i,
                ScatterConfig {
                    kind,
                    water_mode,
                    permit,
                    f,
                },
            )| {
                let block_kind = canvas
                    .get(Vec3::new(wpos2d.x, wpos2d.y, col.alt as i32))
                    .kind();
                if !permit(block_kind) {
                    return None;
                }
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
                    && matches!(&water_mode, Underwater | Floating) == underwater
                {
                    Some((*kind, water_mode))
                } else {
                    None
                }
            },
        );

        if let Some((kind, water_mode)) = kind {
            let (alt, is_under): (_, fn(Block) -> bool) = match water_mode {
                Ground | Underwater => (col.alt as i32, |block| block.is_solid()),
                Floating => (col.water_level as i32, |block| !block.is_air()),
            };

            // Find the intersection between ground and air, if there is one near the
            // Ground
            if let Some(solid_end) = (-4..8)
                .find(|z| is_under(canvas.get(Vec3::new(wpos2d.x, wpos2d.y, alt + z))))
                .and_then(|solid_start| {
                    (1..8)
                        .map(|z| solid_start + z)
                        .find(|z| !is_under(canvas.get(Vec3::new(wpos2d.x, wpos2d.y, alt + z))))
                })
            {
                canvas.map(Vec3::new(wpos2d.x, wpos2d.y, alt + solid_end), |block| {
                    block.with_sprite(kind)
                });
            }
        }
    });
}
