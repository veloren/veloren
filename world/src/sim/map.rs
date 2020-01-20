use crate::{
    sim::{self, uniform_idx_as_vec2, RiverKind, WorldSim, WORLD_SIZE},
    util::Sampler,
    CONFIG,
};
use common::{terrain::TerrainChunkSize, vol::RectVolSize};
use std::{f32, f64};
use vek::*;

pub struct MapConfig {
    /// Dimensions of the window being written to.  Defaults to WORLD_SIZE.
    pub dimensions: Vec2<usize>,
    /// x, y, and z of top left of map (defaults to (0.0, 0.0, CONFIG.sea_level)).
    pub focus: Vec3<f64>,
    /// Altitude is divided by gain and clamped to [0, 1]; thus, decreasing gain makes
    /// smaller differences in altitude appear larger.
    ///
    /// Defaults to CONFIG.mountain_scale.
    pub gain: f32,
    /// lgain is used for shading purposes and refers to how much impact a change in the z
    /// direction has on the perceived slope relative to the same change in x and y.
    ///
    /// Defaults to TerrainChunkSize::RECT_SIZE.x.
    pub lgain: f64,
    /// Scale is like gain, but for x and y rather than z.
    ///
    /// Defaults to WORLD_SIZE.x / dimensions.x (NOTE: fractional, not integer, division!).
    pub scale: f64,
    /// Vector that indicates which direction light is coming from, if shading is turned on.
    ///
    /// Right-handed coordinate system: light is going left, down, and "backwards" (i.e. on the
    /// map, where we translate the y coordinate on the world map to z in the coordinate system,
    /// the light comes from -y on the map and points towards +y on the map).  In a right
    /// handed coordinate system, the "camera" points towards -z, so positive z is backwards
    /// "into" the camera.
    ///
    /// "In world space the x-axis will be pointing east, the y-axis up and the z-axis will be pointing south"
    ///
    /// Defaults to (-0.8, -1.0, 0.3).
    pub light_direction: Vec3<f64>,
    /// If true, only the basement (bedrock) is used for altitude; otherwise, the surface is used.
    ///
    /// Defaults to false.
    pub is_basement: bool,
    /// If true, water is rendered; otherwise, the surface without water is rendered, even if it
    /// is underwater.
    ///
    /// Defaults to true.
    pub is_water: bool,
    /// If true, 3D lighting and shading are turned on.  Otherwise, a plain altitude map is used.
    ///
    /// Defaults to true.
    pub is_shaded: bool,
    /// If true, the red component of the image is also used for temperature (redder is hotter).
    /// Defaults to false.
    pub is_temperature: bool,
    /// If true, the blue component of the image is also used for humidity (bluer is wetter).
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

impl Default for MapConfig {
    fn default() -> Self {
        let dimensions = WORLD_SIZE;
        Self {
            dimensions,
            focus: Vec3::new(0.0, 0.0, CONFIG.sea_level as f64),
            gain: CONFIG.mountain_scale,
            lgain: TerrainChunkSize::RECT_SIZE.x as f64,
            scale: WORLD_SIZE.x as f64 / dimensions.x as f64,
            light_direction: Vec3::new(-0.8, -1.0, 0.3),

            is_basement: false,
            is_water: true,
            is_shaded: true,
            is_temperature: false,
            is_humidity: false,
            is_debug: false,
        }
    }
}

impl MapConfig {
    /// Generates a map image using the specified settings.  Note that it will write from left to
    /// write from (0, 0) to dimensions - 1, inclusive, with 4 1-byte color components provided
    /// as (r, g, b, a).  It is up to the caller to provide a function that translates this
    /// information into the correct format for a buffer and writes to it.
    pub fn generate(
        &self,
        sampler: &WorldSim,
        mut write_pixel: impl FnMut(Vec2<usize>, (u8, u8, u8, u8)),
    ) -> MapDebug {
        let MapConfig {
            dimensions,
            focus,
            gain,
            lgain,
            scale,
            light_direction,

            is_basement,
            is_water,
            is_shaded,
            is_temperature,
            is_humidity,
            is_debug,
        } = *self;

        let light = light_direction.normalized();
        let mut quads = [[0u32; QUADRANTS]; QUADRANTS];
        let mut rivers = 0u32;
        let mut lakes = 0u32;
        let mut oceans = 0u32;

        // let water_light = (light_direction.z + 1.0) / 2.0 * 0.8 + 0.2;
        let focus_rect = Vec2::from(focus);
        let true_sea_level = (CONFIG.sea_level as f64 - focus.z) / gain as f64;

        (0..dimensions.y * dimensions.x)
            .into_iter()
            .for_each(|chunk_idx| {
                let i = chunk_idx % dimensions.x as usize;
                let j = chunk_idx / dimensions.x as usize;

                let pos =
                    (focus_rect + Vec2::new(i as f64, j as f64) * scale).map(|e: f64| e as i32);

                let (alt, basement, water_alt, humidity, temperature, downhill, river_kind) =
                    sampler
                        .get(pos)
                        .map(|sample| {
                            (
                                sample.alt,
                                sample.basement,
                                sample.water_alt,
                                sample.humidity,
                                sample.temp,
                                sample.downhill,
                                sample.river.river_kind,
                            )
                        })
                        .unwrap_or((
                            CONFIG.sea_level,
                            CONFIG.sea_level,
                            CONFIG.sea_level,
                            0.0,
                            0.0,
                            None,
                            None,
                        ));
                let humidity = humidity.min(1.0).max(0.0);
                let temperature = temperature.min(1.0).max(-1.0) * 0.5 + 0.5;
                let pos = pos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32);
                let downhill_pos = (downhill
                    .map(|downhill_pos| downhill_pos/*.map2(TerrainChunkSize::RECT_SIZE, |e, sz: u32| e / sz as i32)*/)
                    .unwrap_or(pos + TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
                    - pos)/* * scale*/
                    + pos;
                let downhill_alt = sampler
                    .get_wpos(downhill_pos)
                    .map(|s| if is_basement { s.basement } else { s.alt })
                    .unwrap_or(CONFIG.sea_level);
                let alt = if is_basement { basement } else { alt };
                let cross_pos = pos
                    + ((downhill_pos - pos)
                        .map(|e| e as f32)
                        .rotated_z(f32::consts::FRAC_PI_2)
                        .map(|e| e as i32));
                let cross_alt = sampler
                    .get_wpos(cross_pos)
                    .map(|s| if is_basement { s.basement } else { s.alt })
                    .unwrap_or(CONFIG.sea_level);
                // Pointing downhill, forward
                // (index--note that (0,0,1) is backward right-handed)
                let forward_vec = Vec3::new(
                    (downhill_pos.x - pos.x) as f64,
                    (downhill_alt - alt) as f64 * lgain,
                    (downhill_pos.y - pos.y) as f64,
                );
                // Pointing 90 degrees left (in horizontal xy) of downhill, up
                // (middle--note that (1,0,0), 90 degrees CCW backward, is right right-handed)
                let up_vec = Vec3::new(
                    (cross_pos.x - pos.x) as f64,
                    (cross_alt - alt) as f64 * lgain,
                    (cross_pos.y - pos.y) as f64,
                );
                // Then cross points "to the right" (upwards) on a right-handed coordinate system.
                // (right-handed coordinate system means (0, 0, 1.0) is "forward" into the screen).
                let surface_normal = forward_vec.cross(up_vec).normalized();
                // f = (0, alt_bl - alt_tl, 1) [backward right-handed = (0,0,1)]
                // u = (1, alt_tr - alt_tl, 0) [right (90 degrees CCW backward) = (1,0,0)]
                // (f × u in right-handed coordinate system: pointing up)
                //
                // f × u =
                //   (a.y*b.z - a.z*b.y,
                //    a.z*b.x - a.x*b.z,
                //    a.x*b.y - a.y*b.x,
                //   )
                // =
                //   (-(alt_tr - alt_tl),
                //    1,
                //    -(alt_bl - alt_tl),
                //   )
                // =
                //   (alt_tl - alt_tr,
                //    1,
                //    alt_tl - alt_bl,
                //   )
                //
                // let surface_normal = Vec3::new((alt_tl - alt_tr) as f64, 1.0, (alt_tl - alt_bl) as f64).normalized();
                let light = (surface_normal.dot(light) + 1.0) / 2.0;
                let light = (light * 0.9) + 0.1;

                let true_water_alt = (alt.max(water_alt) as f64 - focus.z) / gain as f64;
                let true_alt = (alt as f64 - focus.z) / gain as f64;
                let water_depth = (true_water_alt - true_alt).min(1.0).max(0.0);
                let water_alt = true_water_alt.min(1.0).max(0.0);
                let alt = true_alt.min(1.0).max(0.0);
                if is_debug {
                    let quad =
                        |x: f32| ((x as f64 * QUADRANTS as f64).floor() as usize).min(QUADRANTS - 1);
                    if river_kind.is_none() || humidity != 0.0 {
                        quads[quad(humidity)][quad(temperature)] += 1;
                    }
                    match river_kind {
                        Some(RiverKind::River { .. }) => {
                            rivers += 1;
                        }
                        Some(RiverKind::Lake { .. }) => {
                            lakes += 1;
                        }
                        Some(RiverKind::Ocean { .. }) => {
                            oceans += 1;
                        }
                        None => {}
                    }
                }

                let rgba = match (river_kind, (is_water, true_alt >= true_sea_level)) {
                    (_, (false, _)) | (None, (_, true)) => {
                        let (r, g, b) = (
                            (if is_shaded { alt } else { alt }
                                * if is_temperature {
                                    temperature as f64
                                } else if is_shaded {
                                    alt
                                } else {
                                    0.0
                                })
                            .sqrt(),
                            if is_shaded { 0.2 + (alt * 0.8) } else { alt },
                            (if is_shaded { alt } else { alt }
                                * if is_humidity {
                                    humidity as f64
                                } else if is_shaded {
                                    alt
                                } else {
                                    0.0
                                })
                            .sqrt(),
                        );
                        let light = if is_shaded { light } else { 1.0 };
                        (
                            (r * light * 255.0) as u8,
                            (g * light * 255.0) as u8,
                            (b * light * 255.0) as u8,
                            255,
                        )
                    }
                    (Some(RiverKind::Ocean), _) => (
                        0,
                        ((32.0 - water_depth * 32.0) * 1.0) as u8,
                        ((64.0 - water_depth * 64.0) * 1.0) as u8,
                        255,
                    ),
                    (Some(RiverKind::River { .. }), _) => (
                        0,
                        32 + (alt * 95.0) as u8,
                        64 + (alt * 191.0) as u8,
                        255,
                    ),
                    (None, _) | (Some(RiverKind::Lake { .. }), _) => (
                        0,
                        (((32.0 + water_alt * 95.0) + (-water_depth * 32.0)) * 1.0) as u8,
                        (((64.0 + water_alt * 191.0) + (-water_depth * 64.0)) * 1.0) as u8,
                        255,
                    ),
                };

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
