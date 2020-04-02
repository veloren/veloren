use crate::{
    column::{quadratic_nearest_point, river_spline_coeffs, ColumnSample},
    sim::{
        neighbors, uniform_idx_as_vec2, vec2_as_uniform_idx, Alt, RiverKind, WorldSim,
        NEIGHBOR_DELTA, WORLD_SIZE,
    },
    CONFIG,
};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use std::{f32, f64, iter};
use vek::*;

pub struct MapConfig<'a> {
    /// Dimensions of the window being written to.  Defaults to WORLD_SIZE.
    pub dimensions: Vec2<usize>,
    /// x, y, and z of top left of map (defaults to (0.0, 0.0,
    /// CONFIG.sea_level)).
    pub focus: Vec3<f64>,
    /// Altitude is divided by gain and clamped to [0, 1]; thus, decreasing gain
    /// makes smaller differences in altitude appear larger.
    ///
    /// Defaults to CONFIG.mountain_scale.
    pub gain: f32,
    /// lgain is used for shading purposes and refers to how much impact a
    /// change in the z direction has on the perceived slope relative to the
    /// same change in x and y.
    ///
    /// Defaults to TerrainChunkSize::RECT_SIZE.x.
    pub lgain: f64,
    /// Scale is like gain, but for x and y rather than z.
    ///
    /// Defaults to WORLD_SIZE.x / dimensions.x (NOTE: fractional, not integer,
    /// division!).
    pub scale: f64,
    /// Vector that indicates which direction light is coming from, if shading
    /// is turned on.
    ///
    /// Right-handed coordinate system: light is going left, down, and
    /// "backwards" (i.e. on the map, where we translate the y coordinate on
    /// the world map to z in the coordinate system, the light comes from -y
    /// on the map and points towards +y on the map).  In a right
    /// handed coordinate system, the "camera" points towards -z, so positive z
    /// is backwards "into" the camera.
    ///
    /// "In world space the x-axis will be pointing east, the y-axis up and the
    /// z-axis will be pointing south"
    ///
    /// Defaults to (-0.8, -1.0, 0.3).
    pub light_direction: Vec3<f64>,
    /// If Some, uses the provided horizon map.
    ///
    /// Defaults to None.
    pub horizons: Option<&'a [(Vec<Alt>, Vec<Alt>); 2]>,
    /// If Some, uses the provided column samples to determine surface color.
    ///
    /// Defaults to None.
    pub samples: Option<&'a [Option<ColumnSample<'a>>]>,
    /// If true, only the basement (bedrock) is used for altitude; otherwise,
    /// the surface is used.
    ///
    /// Defaults to false.
    pub is_basement: bool,
    /// If true, water is rendered; otherwise, the surface without water is
    /// rendered, even if it is underwater.
    ///
    /// Defaults to true.
    pub is_water: bool,
    /// If true, 3D lighting and shading are turned on.  Otherwise, a plain
    /// altitude map is used.
    ///
    /// Defaults to true.
    pub is_shaded: bool,
    /// If true, the red component of the image is also used for temperature
    /// (redder is hotter). Defaults to false.
    pub is_temperature: bool,
    /// If true, the blue component of the image is also used for humidity
    /// (bluer is wetter).
    ///
    /// Defaults to false.
    pub is_humidity: bool,
    /// Record debug information.
    ///
    /// Defaults to false.
    pub is_debug: bool,
}

pub const QUADRANTS: usize = 4;

pub struct MapDebug {
    pub quads: [[u32; QUADRANTS]; QUADRANTS],
    pub rivers: u32,
    pub lakes: u32,
    pub oceans: u32,
}

impl<'a> Default for MapConfig<'a> {
    fn default() -> Self {
        let dimensions = WORLD_SIZE;
        Self {
            dimensions,
            focus: Vec3::new(0.0, 0.0, CONFIG.sea_level as f64),
            gain: CONFIG.mountain_scale,
            lgain: TerrainChunkSize::RECT_SIZE.x as f64,
            scale: WORLD_SIZE.x as f64 / dimensions.x as f64,
            light_direction: Vec3::new(-1.2, -1.0, 0.8),
            horizons: None,
            samples: None,

            is_basement: false,
            is_water: true,
            is_shaded: true,
            is_temperature: false,
            is_humidity: false,
            is_debug: false,
        }
    }
}

/// Connection kind (per edge).  Currently just supports rivers, but may be
/// extended to support paths or at least one other kind of connection.
#[derive(Clone, Copy, Debug)]
pub enum ConnectionKind {
    /// Connection forms a visible river.
    River,
}

/// Map connection (per edge).
#[derive(Clone, Copy, Debug)]
pub struct Connection {
    /// The kind of connection this is (e.g. river or path).
    pub kind: ConnectionKind,
    /// Assumed to be the "b" part of a 2d quadratic function.
    pub spline_derivative: Vec2<f32>,
    /// Width of the connection.
    pub width: f32,
}

/// Per-chunk data the map needs to be able to sample in order to correctly
/// render.
#[derive(Clone, Debug)]
pub struct MapSample {
    /// the base RGB color for a particular map pixel using the current settings
    /// (i.e. the color *without* lighting).
    pub rgb: Rgb<u8>,
    /// Surface altitude information
    /// (correctly reflecting settings like is_basement and is_water)
    pub alt: f64,
    /// Downhill chunk (may not be meaningful on ocean tiles, or at least edge
    /// tiles)
    pub downhill_wpos: Vec2<i32>,
    /// Connection information about any connections to/from this chunk (e.g.
    /// rivers).
    ///
    /// Connections at each index correspond to the same index in
    /// NEIGHBOR_DELTA.
    pub connections: Option<[Option<Connection>; 8]>,
}

impl<'a> MapConfig<'a> {
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
    pub fn sample_wpos(&self, sampler: &WorldSim, wpos: Vec2<i32>) -> f32 {
        let MapConfig {
            focus,
            gain,

            is_basement,
            is_water,
            ..
        } = *self;

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
    pub fn sample_pos(&self, sampler: &WorldSim, pos: Vec2<i32>) -> MapSample {
        let MapConfig {
            focus,
            gain,
            samples,

            is_basement,
            is_water,
            is_shaded,
            is_temperature,
            is_humidity,
            // is_debug,
            ..
        } = *self;

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
        ) = sampler
            .get(pos)
            .map(|sample| {
                (
                    Some(vec2_as_uniform_idx(pos)),
                    sample.alt,
                    sample.basement,
                    sample.water_alt,
                    sample.humidity,
                    sample.temp,
                    sample.downhill,
                    sample.river.river_kind,
                    sample.river.spline_derivative,
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
        let column_rgb = column_rgb.unwrap_or(Rgb::new(
            if is_shaded || is_temperature {
                1.0
            } else {
                0.0
            },
            if is_shaded { 1.0 } else { alt },
            if is_shaded || is_humidity { 1.0 } else { 0.0 },
        ));
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
            (None, _) | (Some(RiverKind::Lake { .. }), _) | (Some(RiverKind::Ocean), _) => {
                Rgb::new(
                    0,
                    ((g_water - water_depth * g_water) * 1.0) as u8,
                    ((b_water - water_depth * b_water) * 1.0) as u8,
                )
            },
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

    /// Generates a map image using the specified settings.  Note that it will
    /// write from left to write from (0, 0) to dimensions - 1, inclusive,
    /// with 4 1-byte color components provided as (r, g, b, a).  It is up
    /// to the caller to provide a function that translates this information
    /// into the correct format for a buffer and writes to it.
    ///
    /// sample_pos is a function that, given a chunk position, returns enough
    /// information about the chunk to attempt to render it on the map.
    /// When in doubt, try using `MapConfig::sample_pos` for this.
    ///
    /// sample_wpos is a simple function that, given a *column* position,
    /// returns the approximate altitude at that column.  When in doubt, try
    /// using `MapConfig::sample_wpos` for this.
    pub fn generate(
        &self,
        sample_pos: impl Fn(Vec2<i32>) -> MapSample,
        sample_wpos: impl Fn(Vec2<i32>) -> f32,
        // sampler: &WorldSim,
        mut write_pixel: impl FnMut(Vec2<usize>, (u8, u8, u8, u8)),
    ) -> MapDebug {
        let MapConfig {
            dimensions,
            focus,
            gain,
            lgain,
            scale,
            light_direction,
            horizons,

            is_shaded,
            // is_debug,
            ..
        } = *self;

        let light_direction = Vec3::new(
            light_direction.x,
            light_direction.y,
            0.0, // we currently ignore light_direction.z.
        );
        let light_shadow_dir = if light_direction.x >= 0.0 { 0 } else { 1 };
        let horizon_map = horizons.map(|horizons| &horizons[light_shadow_dir]);
        let light = light_direction.normalized();
        let /*mut */quads = [[0u32; QUADRANTS]; QUADRANTS];
        let /*mut */rivers = 0u32;
        let /*mut */lakes = 0u32;
        let /*mut */oceans = 0u32;

        let focus_rect = Vec2::from(focus);

        let chunk_size = TerrainChunkSize::RECT_SIZE.map(|e| e as f64);

        (0..dimensions.y * dimensions.x)
            .into_iter()
            .for_each(|chunk_idx| {
                let i = chunk_idx % dimensions.x as usize;
                let j = chunk_idx / dimensions.x as usize;

                let wposf = focus_rect + Vec2::new(i as f64, j as f64) * scale;
                let pos = wposf.map(|e: f64| e as i32);
                let wposf = wposf * chunk_size;

                let chunk_idx = if pos.reduce_partial_min() >= 0
                    && pos.x < WORLD_SIZE.x as i32
                    && pos.y < WORLD_SIZE.y as i32
                {
                    Some(vec2_as_uniform_idx(pos))
                } else {
                    None
                };

                let MapSample {
                    rgb,
                    alt,
                    downhill_wpos,
                    ..
                } = sample_pos(pos);

                let alt = alt as f32;
                let wposi = pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
                let mut rgb = rgb.map(|e| e as f64 / 255.0);

                // Material properties:
                //
                // For each material in the scene,
                //  k_s = (RGB) specular reflection constant
                let mut k_s = Rgb::new(1.0, 1.0, 1.0);
                //  k_d = (RGB) diffuse reflection constant
                let mut k_d = rgb;
                //  k_a = (RGB) ambient reflection constant
                let mut k_a = rgb;
                //  α = (per-material) shininess constant
                let mut alpha = 4.0; // 4.0;

                // Compute connections
                let mut has_river = false;
                // NOTE: consider replacing neighbors with local_cells, since it is more
                // accurate (though I'm not sure if it can matter for these
                // purposes).
                chunk_idx
                    .map(|chunk_idx| neighbors(chunk_idx).chain(iter::once(chunk_idx)))
                    .into_iter()
                    .flatten()
                    .for_each(|neighbor_posi| {
                        let neighbor_pos = uniform_idx_as_vec2(neighbor_posi);
                        let neighbor_wpos = neighbor_pos.map(|e| e as f64) * chunk_size;
                        let MapSample { connections, .. } = sample_pos(neighbor_pos);
                        NEIGHBOR_DELTA
                            .iter()
                            .zip(
                                connections
                                    .as_ref()
                                    .map(|e| e.iter())
                                    .into_iter()
                                    .flatten()
                                    .into_iter(),
                            )
                            .for_each(|(&delta, connection)| {
                                let connection = if let Some(connection) = connection {
                                    connection
                                } else {
                                    return;
                                };
                                let downhill_wpos = neighbor_wpos
                                    + Vec2::from(delta).map(|e: i32| e as f64) * chunk_size;
                                let coeffs = river_spline_coeffs(
                                    neighbor_wpos,
                                    connection.spline_derivative,
                                    downhill_wpos,
                                );
                                let (_t, _pt, dist) = if let Some((t, pt, dist)) =
                                    quadratic_nearest_point(&coeffs, wposf)
                                {
                                    (t, pt, dist)
                                } else {
                                    let ndist = wposf.distance_squared(neighbor_wpos);
                                    let ddist = wposf.distance_squared(downhill_wpos);
                                    if ndist <= ddist {
                                        (0.0, neighbor_wpos, ndist)
                                    } else {
                                        (1.0, downhill_wpos, ddist)
                                    }
                                };
                                let connection_dist = (dist.sqrt()
                                    - (connection.width as f64 * 0.5).max(1.0))
                                .max(0.0);
                                if connection_dist == 0.0 {
                                    match connection.kind {
                                        ConnectionKind::River => {
                                            has_river = true;
                                        },
                                    }
                                }
                            });
                    });

                // Color in connectins.
                let water_color_factor = 2.0;
                let g_water = 32.0 * water_color_factor;
                let b_water = 64.0 * water_color_factor;
                if has_river {
                    let water_rgb = Rgb::new(0, ((g_water) * 1.0) as u8, ((b_water) * 1.0) as u8)
                        .map(|e| e as f64 / 255.0);
                    rgb = water_rgb;
                    k_s = Rgb::new(1.0, 1.0, 1.0);
                    k_d = water_rgb;
                    k_a = water_rgb;
                    alpha = 0.255;
                }

                let downhill_alt = sample_wpos(downhill_wpos);
                let cross_pos = wposi
                    + ((downhill_wpos - wposi)
                        .map(|e| e as f32)
                        .rotated_z(f32::consts::FRAC_PI_2)
                        .map(|e| e as i32));
                let cross_alt = sample_wpos(cross_pos);
                // Pointing downhill, forward
                // (index--note that (0,0,1) is backward right-handed)
                let forward_vec = Vec3::new(
                    (downhill_wpos.x - wposi.x) as f64,
                    ((downhill_alt - alt) * gain) as f64 * lgain,
                    (downhill_wpos.y - wposi.y) as f64,
                );
                // Pointing 90 degrees left (in horizontal xy) of downhill, up
                // (middle--note that (1,0,0), 90 degrees CCW backward, is right right-handed)
                let up_vec = Vec3::new(
                    (cross_pos.x - wposi.x) as f64,
                    ((cross_alt - alt) * gain) as f64 * lgain,
                    (cross_pos.y - wposi.y) as f64,
                );
                // let surface_normal = Vec3::new(lgain * (f.y * u.z - f.z * u.y), -(f.x * u.z -
                // f.z * u.x), lgain * (f.x * u.y - f.y * u.x)).normalized();
                // Then cross points "to the right" (upwards) on a right-handed coordinate
                // system. (right-handed coordinate system means (0, 0, 1.0) is
                // "forward" into the screen).
                let surface_normal = forward_vec.cross(up_vec).normalized();

                // TODO: Figure out if we can reimplement debugging.
                /* if is_debug {
                    let quad = |x: f32| {
                        ((x as f64 * QUADRANTS as f64).floor() as usize).min(QUADRANTS - 1)
                    };
                    if river_kind.is_none() || humidity != 0.0 {
                        quads[quad(humidity)][quad(temperature)] += 1;
                    }
                    match river_kind {
                        Some(RiverKind::River { .. }) => {
                            rivers += 1;
                        },
                        Some(RiverKind::Lake { .. }) => {
                            lakes += 1;
                        },
                        Some(RiverKind::Ocean { .. }) => {
                            oceans += 1;
                        },
                        None => {},
                    }
                } */

                let shade_frac = horizon_map
                    .and_then(|(angles, heights)| {
                        chunk_idx
                            .and_then(|chunk_idx| angles.get(chunk_idx))
                            .map(|&e| (e as f64, heights))
                    })
                    .and_then(|(e, heights)| {
                        chunk_idx
                            .and_then(|chunk_idx| heights.get(chunk_idx))
                            .map(|&f| (e, f as f64))
                    })
                    .map(|(angle, height)| {
                        let w = 0.1;
                        if angle != 0.0 && light_direction.x != 0.0 {
                            let deltax = height / angle;
                            let lighty = (light_direction.y / light_direction.x * deltax).abs();
                            let deltay = lighty - height;
                            let s = (deltay / deltax / w).min(1.0).max(0.0);
                            // Smoothstep
                            s * s * (3.0 - 2.0 * s)
                        } else {
                            1.0
                        }
                    })
                    .unwrap_or(alt as f64);

                let rgb = if is_shaded {
                    // Phong reflection model with shadows:
                    //
                    // I_p = k_a i_a + shadow * Σ {m ∈ lights} (k_d (L_m ⋅ N) i_m,d + k_s (R_m ⋅
                    // V)^α i_m,s)
                    //
                    // where for the whole scene,
                    //  i_a = (RGB) intensity of ambient lighting component
                    let i_a = Rgb::new(0.1, 0.1, 0.1);
                    //  V = direction pointing towards the viewer (e.g. virtual camera).
                    let v = Vec3::new(0.0, 0.0, -1.0).normalized();
                    // let v = Vec3::new(0.0, -1.0, 0.0).normalized();
                    //
                    // for each light m,
                    //  i_m,d = (RGB) intensity of diffuse component of light source m
                    let i_m_d = Rgb::new(0.45, 0.45, 0.45);
                    //  i_m,s = (RGB) intensity of specular component of light source m
                    let i_m_s = Rgb::new(0.45, 0.45, 0.45);
                    // let i_m_s = Rgb::new(0.45, 0.45, 0.45);

                    // for each light m and point p,
                    //  L_m = (normalized) direction vector from point on surface to light source m
                    let l_m = light;
                    //  N = (normalized) normal at this point on the surface,
                    let n = surface_normal;
                    //  R_m = (normalized) direction a perfectly reflected ray of light from m would
                    // take from point p      = 2(L_m ⋅ N)N - L_m
                    let r_m = (-l_m).reflected(n); // 2 * (l_m.dot(n)) * n - l_m;
                    //
                    // and for each point p in the scene,
                    //  shadow = computed shadow factor at point p
                    // FIXME: Should really just be shade_frac, but with only ambient light we lose
                    // all local lighting detail... some sort of global illumination (e.g.
                    // radiosity) is of course the "right" solution, but maybe we can find
                    // something cheaper?
                    let shadow = 0.2 + 0.8 * shade_frac;

                    let lambertian = l_m.dot(n).max(0.0);
                    let spec_angle = r_m.dot(v).max(0.0);

                    let ambient = k_a * i_a;
                    let diffuse = k_d * lambertian * i_m_d;
                    let specular = k_s * spec_angle.powf(alpha) * i_m_s;
                    (ambient + shadow * (diffuse + specular)).map(|e| e.min(1.0))
                } else {
                    rgb
                }
                .map(|e| (e * 255.0) as u8);

                let rgba = (rgb.r, rgb.g, rgb.b, 255);
                write_pixel(Vec2::new(i, j), rgba);
            });

        MapDebug {
            quads,
            rivers,
            lakes,
            oceans,
        }
    }
}
