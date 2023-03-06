use crate::{
    column::ColumnSample,
    sim::{RiverKind, WorldSim},
    site::SiteKind,
    IndexRef, CONFIG,
};
use common::{
    terrain::{
        map::{Connection, ConnectionKind, MapConfig, MapSample},
        vec2_as_uniform_idx, CoordinateConversions, TerrainChunkSize, NEIGHBOR_DELTA,
    },
    vol::RectVolSize,
};
use std::f64;
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
        / gain
}

/// Samples a MapSample at a chunk.
///
/// You should generally pass a closure over this function into generate
/// when constructing a map for the first time.
/// However, if repeated construction is needed, or alternate base colors
/// are to be used for some reason, one should pass a custom function to
/// generate instead (e.g. one that just looks up the color in a cached
/// array).
// NOTE: Deliberately not putting Rgb colors here in the config file; they
// aren't hot reloaded anyway, and for various reasons they're probably not a
// good idea to update in that way (for example, we currently want water colors
// to match voxygen's).  Eventually we'll fix these sorts of issues in some
// other way.
pub fn sample_pos(
    config: &MapConfig,
    sampler: &WorldSim,
    index: IndexRef,
    samples: Option<&[Option<ColumnSample>]>,
    pos: Vec2<i32>,
) -> MapSample {
    let map_size_lg = config.map_size_lg();
    let MapConfig {
        focus,
        gain,

        is_basement,
        is_water,
        is_ice,
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
        is_bridge,
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
                sample.path.0.is_way(),
                sample
                    .sites
                    .iter()
                    .any(|site| match &index.sites.get(*site).kind {
                        SiteKind::Bridge(bridge) => {
                            if let Some(plot) =
                                bridge.wpos_tile(TerrainChunkSize::center_wpos(pos)).plot
                            {
                                matches!(bridge.plot(plot).kind, crate::site2::PlotKind::Bridge(_))
                            } else {
                                false
                            }
                        },
                        _ => false,
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

    let humidity = humidity.clamp(0.0, 1.0);
    let temperature = temperature.clamp(-1.0, 1.0) * 0.5 + 0.5;
    let wpos = pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
    let column_data = samples
        .and_then(|samples| {
            chunk_idx
                .and_then(|chunk_idx| samples.get(chunk_idx))
                .and_then(Option::as_ref)
        })
        .map(|sample| {
            // TODO: Eliminate the redundancy between this and the block renderer.
            let alt = sample.alt;
            let basement = sample.basement;
            let grass_depth = (1.5 + 2.0 * sample.chaos).min(alt - basement);
            let wposz = if is_basement { basement } else { alt };
            let rgb = if is_basement && wposz < alt - grass_depth {
                Lerp::lerp(
                    sample.sub_surface_color,
                    sample.stone_col.map(|e| e as f32 / 255.0),
                    (alt - grass_depth - wposz) * 0.15,
                )
                .map(|e| e as f64)
            } else {
                Lerp::lerp(
                    sample.sub_surface_color,
                    sample.surface_color,
                    ((wposz - (alt - grass_depth)) / grass_depth).sqrt(),
                )
                .map(|e| e as f64)
            };

            (rgb, alt, sample.ice_depth)
        });

    let downhill_wpos = downhill.unwrap_or(wpos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32));
    let alt = if is_basement {
        basement
    } else {
        column_data.map_or(alt, |(_, alt, _)| alt)
    };

    let true_water_alt = (alt.max(water_alt) as f64 - focus.z) / gain as f64;
    let true_alt = (alt as f64 - focus.z) / gain as f64;
    let water_depth = (true_water_alt - true_alt).clamp(0.0, 1.0);
    let alt = true_alt.clamp(0.0, 1.0);

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
    let column_rgb = column_data.map(|(rgb, _, _)| rgb).unwrap_or(default_rgb);
    let mut connections = [None; 8];
    let mut has_connections = false;
    // TODO: Support non-river connections.
    // TODO: Support multiple connections.
    let river_width = river_kind.map(|river| match river {
        RiverKind::River { cross_section } => cross_section.x,
        RiverKind::Lake { .. } | RiverKind::Ocean => TerrainChunkSize::RECT_SIZE.x as f32,
    });
    if let (Some(river_width), true) = (river_width, is_water) {
        let downhill_pos = downhill_wpos.wpos_to_cpos();
        NEIGHBOR_DELTA
            .iter()
            .zip(connections.iter_mut())
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
    let rgb =
        if is_water && is_ice && column_data.map_or(false, |(_, _, ice_depth)| ice_depth > 0.0) {
            CONFIG.ice_color
        } else {
            match (river_kind, (is_water, true_alt >= true_sea_level)) {
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
                (None | Some(RiverKind::Lake { .. } | RiverKind::Ocean), _) => Rgb::new(
                    0,
                    ((g_water - water_depth * g_water) * 1.0) as u8,
                    ((b_water - water_depth * b_water) * 1.0) as u8,
                ),
            }
        };
    // TODO: Make principled.
    let rgb = if is_bridge {
        Rgb::new(0x80, 0x80, 0x80)
    } else if is_path {
        Rgb::new(0x37, 0x29, 0x23)
    } else {
        rgb
    };

    MapSample {
        rgb: Rgb::new(rgb.r, rgb.g, rgb.b),
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
