use crate::{column::ColumnSample, sim::SimChunk, util::RandomField, IndexRef, CONFIG};
use common::{
    terrain::{Block, BlockKind},
    vol::{BaseVol, ReadVol, RectSizedVol, WriteVol},
};
use noise::NoiseFn;
use std::f32;
use vek::*;

fn close(x: f32, tgt: f32, falloff: f32) -> f32 {
    (1.0 - (x - tgt).abs() / falloff).max(0.0).powf(0.125)
}
const MUSH_FACT: f32 = 1.0e-4; // To balance everything around the mushroom spawning rate
pub fn apply_scatter_to<'a>(
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &mut (impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
    chunk: &SimChunk,
) {
    use BlockKind::*;
    #[allow(clippy::type_complexity)]
    // TODO: Add back all sprites we had before
    let scatter: &[(
        _,
        bool,
        fn(&SimChunk, &ColumnSample) -> (f32, Option<(f32, f32)>),
    )] = &[
        // (density, Option<(wavelen, threshold)>)
        // Flowers
        (BlueFlower, false, |c, col| {
            (
                close(c.temp, CONFIG.temperate_temp, 0.7).min(close(
                    c.humidity,
                    CONFIG.jungle_hum,
                    0.4,
                )) * col.tree_density
                    * MUSH_FACT
                    * 256.0,
                Some((256.0, 0.25)),
            )
        }),
        (PinkFlower, false, |c, col| {
            (
                close(c.temp, 0.0, 0.7).min(close(c.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((64.0, 0.2)),
            )
        }),
        (PurpleFlower, false, |c, col| {
            (
                close(c.temp, CONFIG.temperate_temp, 0.7).min(close(
                    c.humidity,
                    CONFIG.jungle_hum,
                    0.4,
                )) * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((64.0, 0.2)),
            )
        }),
        (RedFlower, false, |c, col| {
            (
                close(c.temp, CONFIG.tropical_temp, 0.6).min(close(
                    c.humidity,
                    CONFIG.jungle_hum,
                    0.3,
                )) * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((64.0, 0.1)),
            )
        }),
        (WhiteFlower, false, |c, col| {
            (
                close(c.temp, 0.0, 0.7).min(close(c.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((64.0, 0.2)),
            )
        }),
        (YellowFlower, false, |c, col| {
            (
                close(c.temp, 0.0, 0.7).min(close(c.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((64.0, 0.2)),
            )
        }),
        (Sunflower, false, |c, col| {
            (
                close(c.temp, 0.0, 0.7).min(close(c.humidity, CONFIG.jungle_hum, 0.4))
                    * col.tree_density
                    * MUSH_FACT
                    * 350.0,
                Some((300.0, 0.2)),
            )
        }),
        // Herbs and Spices
        (LingonBerry, false, |c, _| {
            (
                close(c.temp, 0.3, 0.4).min(close(c.humidity, CONFIG.jungle_hum, 0.5))
                    * MUSH_FACT
                    * 2.5,
                None,
            )
        }),
        (LeafyPlant, false, |c, _| {
            (
                close(c.temp, 0.3, 0.4).min(close(c.humidity, CONFIG.jungle_hum, 0.3))
                    * MUSH_FACT
                    * 4.0,
                None,
            )
        }),
        (Fern, false, |c, _| {
            (
                close(c.temp, 0.3, 0.4).min(close(c.humidity, CONFIG.forest_hum, 0.5))
                    * MUSH_FACT
                    * 0.5,
                Some((48.0, 0.3)),
            )
        }),
        (Blueberry, false, |c, _| {
            (
                close(c.temp, CONFIG.temperate_temp, 0.5).min(close(
                    c.humidity,
                    CONFIG.forest_hum,
                    0.5,
                )) * MUSH_FACT
                    * 0.3,
                None,
            )
        }),
        // Collectable Objects
        // Only spawn twigs in temperate forests
        (Twigs, false, |c, _| {
            ((c.tree_density - 0.5).max(0.0) * 1.0e-3, None)
        }),
        (Stones, false, |c, _| {
            ((c.rockiness - 0.5).max(0.0) * 1.0e-3, None)
        }),
        // Don't spawn Mushrooms in snowy regions
        (Mushroom, false, |c, _| {
            (
                close(c.temp, 0.3, 0.4).min(close(c.humidity, CONFIG.forest_hum, 0.35)) * MUSH_FACT,
                None,
            )
        }),
        // Grass
        (ShortGrass, false, |c, _| {
            (
                close(c.temp, 0.2, 0.65).min(close(c.humidity, CONFIG.jungle_hum, 0.4)) * 0.03,
                None,
            )
        }),
        (MediumGrass, false, |c, _| {
            (
                close(c.temp, 0.2, 0.6).min(close(c.humidity, CONFIG.jungle_hum, 0.4)) * 0.02,
                None,
            )
        }),
        (LongGrass, false, |c, _| {
            (
                close(c.temp, 0.3, 0.35).min(close(c.humidity, CONFIG.jungle_hum, 0.3)) * 0.15,
                Some((48.0, 0.3)),
            )
        }),
        // Jungle Sprites
        // (LongGrass, false, |c, col| {
        //     (
        //         close(c.temp, CONFIG.tropical_temp, 0.4).min(close(
        //             c.humidity,
        //             CONFIG.jungle_hum,
        //             0.6,
        //         )) * 0.08,
        //         Some((60.0, 5.0)),
        //     )
        // }),
        /*(WheatGreen, false, |c, col| {
            (
                close(c.temp, 0.4, 0.2).min(close(c.humidity, CONFIG.forest_hum, 0.1))
                    * MUSH_FACT
                    * 0.001,
                None,
            )
        }),*/
        (GrassSnow, false, |c, _| {
            (
                close(c.temp, CONFIG.snow_temp - 0.2, 0.4).min(close(
                    c.humidity,
                    CONFIG.forest_hum,
                    0.5,
                )) * 0.01,
                Some((48.0, 0.2)),
            )
        }),
        // Desert Plants
        (DeadBush, false, |c, _| {
            (
                close(c.temp, 1.0, 0.95).min(close(c.humidity, 0.0, 0.3)) * MUSH_FACT * 15.0,
                None,
            )
        }),
        (LargeCactus, false, |c, _| {
            (
                close(c.temp, CONFIG.desert_temp + 0.2, 0.3).min(close(
                    c.humidity,
                    CONFIG.desert_hum,
                    0.2,
                )) * MUSH_FACT
                    * 0.1,
                None,
            )
        }),
        /*(BarrelCactus, false, |c, col| {
            (
                close(c.temp, CONFIG.desert_temp + 0.2, 0.3).min(close(
                    c.humidity,
                    CONFIG.desert_hum,
                    0.2,
                )) * MUSH_FACT
                    * 0.1,
                None,
            )
        }),
        (RoundCactus, false, |c, col| {
            (
                close(c.temp, CONFIG.desert_temp + 0.2, 0.3).min(close(
                    c.humidity,
                    CONFIG.desert_hum,
                    0.2,
                )) * MUSH_FACT
                * 0.1,
                None,
            )
        }),
        (ShortCactus, false, |c, col| {
            (
                close(c.temp, CONFIG.desert_temp + 0.2, 0.3).min(close(
                    c.humidity,
                    CONFIG.desert_hum,
                    0.2,
                )) * MUSH_FACT
                * 0.1,
                None,
            )
        }),
        (MedFlatCactus, false, |c, col| {
            (
                close(c.temp, CONFIG.desert_temp + 0.2, 0.3).min(close(
                    c.humidity,
                    CONFIG.desert_hum,
                    0.2,
                )) * MUSH_FACT
                * 0.1,
                None,
            )
        }),
        (ShortFlatCactus, false, |c, col| {
            (
                close(c.temp, CONFIG.desert_temp + 0.2, 0.3).min(close(
                    c.humidity,
                    CONFIG.desert_hum,
                    0.2,
                )) * MUSH_FACT
                * 0.1,
                None,
            )
        }),*/
        (Reed, false, |c, col| {
            (
                close(c.humidity, CONFIG.jungle_hum, 0.7)
                    * col
                        .water_dist
                        .map(|wd| Lerp::lerp(0.2, 0.0, (wd / 8.0).clamped(0.0, 1.0)))
                        .unwrap_or(0.0),
                Some((128.0, 0.5)),
            )
        }),
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

            let bk = scatter
                .iter()
                .enumerate()
                .find_map(|(i, (bk, is_underwater, f))| {
                    let (density, patch) = f(chunk, col_sample);
                    let is_patch = patch
                        .map(|(wavelen, threshold)| {
                            index
                                .noise
                                .scatter_nz
                                .get(
                                    wpos2d
                                        .map(|e| e as f64 / wavelen as f64 + i as f64 * 43.0)
                                        .into_array(),
                                )
                                .abs()
                                > 1.0 - threshold as f64
                        })
                        .unwrap_or(true);
                    if density > 0.0
                        && is_patch
                        && RandomField::new(i as u32)
                            .chance(Vec3::new(wpos2d.x, wpos2d.y, 0), density)
                        && underwater == *is_underwater
                    {
                        Some(*bk)
                    } else {
                        None
                    }
                });

            if let Some(bk) = bk {
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
                    let _ = vol.set(
                        Vec3::new(offs.x, offs.y, alt + solid_end),
                        Block::new(bk, Rgb::broadcast(0)),
                    );
                }
            }
        }
    }
}
