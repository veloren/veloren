use crate::{
    column::ColumnSample,
    sim::{RiverKind, WorldSim},
    CONFIG,
};
use common::{
    terrain::{
        map::{Connection, ConnectionKind, MapConfig, MapSample},
        vec2_as_uniform_idx, TerrainChunkSize, NEIGHBOR_DELTA,
    },
    vol::RectVolSize,
};
use std::{f32, f64};
use vek::*;

/// A sample function that grabs the connections at a chunk.
///
/// Currently this just supports rivers, but ideally it can be extended past
/// that.
///
/// A sample function that grabs surface altitude at a column.
/// (correctly reflecting settings like is_basement and is_water).
///
/// The altitude produced by this function at a column corresponding to a
/// particular chunk should be identical to the altitude produced by
/// sample_pos at that chunk.
///
/// You should generally pass a closure over this function into generate
/// when constructing a map for the first time.
/// However, if repeated construction is needed, or alternate base colors
/// are to be used for some reason, one should pass a custom function to
/// generate instead (e.g. one that just looks up the height in a cached
/// array).
pub fn sample_wpos(config: &MapConfig, sampler: &WorldSim, wpos: Vec2<i32>) -> f32 {
    let MapConfig {
        focus,
        gain,

        is_basement,
        is_water,
        ..
    } = *config;

    (sampler
        .get_wpos(wpos)
        .map(|s| {
            if is_basement { s.basement } else { s.alt }.max(if is_water {
                s.water_alt
            } else {
                -f32::INFINITY
            })
        })
        .unwrap_or(CONFIG.sea_level)
        - focus.z as f32)
        / gain as f32
}

/// Samples a MapSample at a chunk.
///
/// You should generally pass a closure over this function into generate
/// when constructing a map for the first time.
/// However, if repeated construction is needed, or alternate base colors
/// are to be used for some reason, one should pass a custom function to
/// generate instead (e.g. one that just looks up the color in a cached
/// array).
pub fn sample_pos(
    config: &MapConfig,
    sampler: &WorldSim,
    samples: Option<&[Option<ColumnSample>]>,
    pos: Vec2<i32>,
) -> MapSample {
    let map_size_lg = config.map_size_lg();
    let MapConfig {
        focus,
        gain,

        is_basement,
        is_water,
        is_shaded,
        is_temperature,
        is_humidity,
        // is_debug,
        ..
    } = *config;

    let true_sea_level = (CONFIG.sea_level as f64 - focus.z) / gain as f64;

    let (
        chunk_idx,
        alt,
        basement,
        water_alt,
        humidity,
        temperature,
        downhill,
        river_kind,
        spline_derivative,
        is_path,
        near_site,
    ) = sampler
        .get(pos)
        .map(|sample| {
            (
                Some(vec2_as_uniform_idx(map_size_lg, pos)),
                sample.alt,
                sample.basement,
                sample.water_alt,
                sample.humidity,
                sample.temp,
                sample.downhill,
                sample.river.river_kind,
                sample.river.spline_derivative,
                sample.path.is_path(),
                sample.sites.iter().any(|site| {
                    site.get_origin()
                        .distance_squared(pos * TerrainChunkSize::RECT_SIZE.x as i32)
                        < 64i32.pow(2)
                }),
            )
        })
        .unwrap_or((
            None,
            CONFIG.sea_level,
            CONFIG.sea_level,
            CONFIG.sea_level,
            0.0,
            0.0,
            None,
            None,
            Vec2::zero(),
            false,
            false,
        ));

    let humidity = humidity.min(1.0).max(0.0);
    let temperature = temperature.min(1.0).max(-1.0) * 0.5 + 0.5;
    let wpos = pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
    let column_rgb = samples
        .and_then(|samples| {
            chunk_idx
                .and_then(|chunk_idx| samples.get(chunk_idx))
                .map(Option::as_ref)
                .flatten()
        })
        .map(|sample| {
            // TODO: Eliminate the redundancy between this and the block renderer.
            let alt = sample.alt;
            let basement = sample.basement;
            let grass_depth = (1.5 + 2.0 * sample.chaos).min(alt - basement);
            let wposz = if is_basement { basement } else { alt };
            if is_basement && wposz < alt - grass_depth {
                Lerp::lerp(
                    sample.sub_surface_color,
                    sample.stone_col.map(|e| e as f32 / 255.0),
                    (alt - grass_depth - wposz as f32) * 0.15,
                )
                .map(|e| e as f64)
            } else {
                Lerp::lerp(
                    sample.sub_surface_color,
                    sample.surface_color,
                    ((wposz as f32 - (alt - grass_depth)) / grass_depth).powf(0.5),
                )
                .map(|e| e as f64)
            }
        });

    let downhill_wpos = downhill
        .map(|downhill_pos| downhill_pos)
        .unwrap_or(wpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32));
    let alt = if is_basement { basement } else { alt };

    let true_water_alt = (alt.max(water_alt) as f64 - focus.z) / gain as f64;
    let true_alt = (alt as f64 - focus.z) / gain as f64;
    let water_depth = (true_water_alt - true_alt).min(1.0).max(0.0);
    let alt = true_alt.min(1.0).max(0.0);

    let water_color_factor = 2.0;
    let g_water = 32.0 * water_color_factor;
    let b_water = 64.0 * water_color_factor;
    let default_rgb = Rgb::new(
        if is_shaded || is_temperature {
            1.0
        } else {
            0.0
        },
        if is_shaded { 1.0 } else { alt },
        if is_shaded || is_humidity { 1.0 } else { 0.0 },
    );
    let column_rgb = column_rgb.unwrap_or(default_rgb);
    let mut connections = [None; 8];
    let mut has_connections = false;
    // TODO: Support non-river connections.
    // TODO: Support multiple connections.
    let river_width = river_kind.map(|river| match river {
        RiverKind::River { cross_section } => cross_section.x,
        RiverKind::Lake { .. } | RiverKind::Ocean => TerrainChunkSize::RECT_SIZE.x as f32,
    });
    if let (Some(river_width), true) = (river_width, is_water) {
        let downhill_pos = downhill_wpos.map2(TerrainChunkSize::RECT_SIZE, |e, f| e / f as i32);
        NEIGHBOR_DELTA
            .iter()
            .zip((&mut connections).iter_mut())
            .filter(|&(&offset, _)| downhill_pos - pos == Vec2::from(offset))
            .for_each(|(_, connection)| {
                has_connections = true;
                *connection = Some(Connection {
                    kind: ConnectionKind::River,
                    spline_derivative,
                    width: river_width,
                });
            });
    };
    let rgb = match (river_kind, (is_water, true_alt >= true_sea_level)) {
        (_, (false, _)) | (None, (_, true)) | (Some(RiverKind::River { .. }), _) => {
            let (r, g, b) = (
                (column_rgb.r
                    * if is_temperature {
                        temperature as f64
                    } else {
                        column_rgb.r
                    })
                .sqrt(),
                column_rgb.g,
                (column_rgb.b
                    * if is_humidity {
                        humidity as f64
                    } else {
                        column_rgb.b
                    })
                .sqrt(),
            );
            Rgb::new((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
        },
        (None, _) | (Some(RiverKind::Lake { .. }), _) | (Some(RiverKind::Ocean), _) => Rgb::new(
            0,
            ((g_water - water_depth * g_water) * 1.0) as u8,
            ((b_water - water_depth * b_water) * 1.0) as u8,
        ),
    };
    // TODO: Make principled.
    let rgb = if near_site {
        Rgb::new(0x57, 0x39, 0x33)
    } else if is_path {
        Rgb::new(0x37, 0x29, 0x23)
    } else {
        rgb
    };

    MapSample {
        rgb,
        alt: if is_water {
            true_alt.max(true_water_alt)
        } else {
            true_alt
        },
        downhill_wpos,
        connections: if has_connections {
            Some(connections)
        } else {
            None
        },
    }
}
