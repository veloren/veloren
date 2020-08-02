use super::{
    neighbors, quadratic_nearest_point, river_spline_coeffs, uniform_idx_as_vec2,
    vec2_as_uniform_idx, TerrainChunkSize, NEIGHBOR_DELTA, TERRAIN_CHUNK_BLOCKS_LG,
};
use crate::vol::RectVolSize;
use core::{f32, f64, iter, ops::RangeInclusive};
use vek::*;

/// Base two logarithm of the maximum size of the precomputed world, in meters,
/// along the x (E/W) and y (N/S) dimensions.
///
/// NOTE: Each dimension is guaranteed to be a power of 2, so the logarithm is
/// exact. This is so that it is possible (at least in theory) for compiler or
/// runtime optimizations exploiting this are possible.  For example, division
/// by the chunk size can turn into a bit shift.
///
/// NOTE: As an invariant, this value is at least [TERRAIN_CHUNK_BLOCKS_LG].
///
/// NOTE: As an invariant, `(1 << [MAX_WORLD_BLOCKS_LG])` fits in an i32.
///
/// TODO: Add static assertions for the above invariants.
///
/// Currently, we define the maximum to be 19 (corresponding to 2^19 m) for both
/// directions. This value was derived by backwards reasoning from the following
/// conservative estimate of the maximum landmass area (using an approximation
/// of 1024 blocks / km instead of 1000 blocks / km, which will result in an
/// estimate that is strictly lower than the real landmass):
///
/// Max area (km²)
///     ≌ (2^19 blocks * 1 km / 1024 blocks)^2
///     = 2^((19 - 10) * 2) km²
///     = 2^18 km²
///     = 262,144 km²
///
/// which is roughly the same area as the entire United Kingdom, and twice the
/// horizontal extent of Dwarf Fortress's largest map.  Besides the comparison
/// to other games without infinite or near-infinite maps (like Dwarf Fortress),
/// there are other reasons to choose this as a good maximum size:
///
/// * It is large enough to include geological features of fairly realistic
///   scale.  It may be hard to do justice to truly enormous features like the
///   Amazon River, and natural temperature variation not related to altitude
///   would probably not produce climate extremes on an Earth-like planet, but
///   it can comfortably fit enormous river basins, Everest-scale mountains,
///   large islands and inland lakes, vast forests and deserts, and so on.
///
/// * It is large enough that making it from one side of the map to another will
///   take a *very* long time.  We show this with two examples.  In each
///   example, travel is either purely horizontal or purely vertical (to
///   minimize distance traveled) across the whole map, and we assume there are
///   no obstacles or slopes.
///
///   In example 1, a human is walking at the (real-time) speed of the fastest
/// marathon runners   (around 6 blocks / real-time s).  We assume the human can
/// maintain this pace indefinitely   without stopping.  Then crossing the map
/// will take about:
///
///   2^19 blocks * 1 real-time s / 6 blocks * 1 real-time min / 60 real-time s
/// * 1 real-time hr / 60 real-time min * 1 real-time days / 24 hr = 2^19 / 6 /
///   60 / 60 / 24 real-time days ≌ 1 real-time day.
///
///   That's right--it will take a full day of *real* time to cross the map at
/// an apparent speed of   6 m / s.  Moreover, since in-game time passes at a
/// rate of 1 in-game min / 1 in-game s, this   would also take *60 days* of
/// in-game time.
///
///   Still though, this is the rate of an ordinary human.  And besides that, if
/// we instead had a   marathon runner traveling at 6 m / in-game s, it would
/// take just 1 day of in-game time for   the runner to cross the map, or a mere
/// 0.4 hr of real time.   To show that this rate of travel is unrealistic (and
/// perhaps make an eventual argument for   a slower real-time to in-game time
/// conversion rate), our second example will consist of a   high-speed train
/// running at 300 km / real-time h (the fastest real-world high speed train
///   averages under 270 k m / h, with 300 km / h as the designed top speed).
/// For a train   traveling at this apparent speed (in real time), crossing the
/// map would take:
///
///   2^19 blocks * 1 km / 1000 blocks * 1 real-time hr / 300 km
///   = 2^19 / 1000 / 300 real-time hr
///   ≌ 1.75 real-time hr
///
///   = 2^19 / 1000 / 300 real-time hr * 60 in-game hr / real-time hr
///     * 1 in-game days / 24 in-game hr
///   = 2^19 / 1000 / 300 * 60 / 24 in-game days
///   ≌ 4.37 in-game days
///
///   In other words, something faster in real-time than any existing high-speed
/// train would be   over 4 times slower (in real-time) than our hypothetical
/// top marathon runner running at 6 m /   s in in-game speed.  This suggests
/// that the gap between in-game time and real-time is   probably much too large
/// for most purposes; however, what it definitely shows is that even
///   extremely fast in-game transport across the world will not trivialize its
/// size.
///
///   It follows that cities or towns of realistic scale, player housing,
/// fields, and so on, will   all fit comfortably on a map of this size, while
/// at the same time still being reachable by   non-warping, in-game mechanisms
/// (such as high-speed transit).  It also provides plenty of   room for mounts
/// of varying speeds, which can help ensure that players don't feel cramped or
///   deliberately slowed down by their own speed.
///
/// * It is small enough that it is (barely) plausible that we could still
///   generate maps for a world of this size using detailed and realistic
///   erosion algorithms.  At 1/4 of this map size along each dimension,
///   generation currently takes around 5 hours on a good computer, and one
///   could imagine (since the bottleneck step appears to be roughly O(n)) that
///   with a smart implementation generation times of under a week could be
///   achievable.
///
/// * The map extends further than the resolution of human eyesight under
///   Earthlike conditions, even from tall mountains across clear landscapes.
///   According to one calculation, even from Mt. Everest in the absence of
///   cloud cover, you could only see for about 339 km before the Earth's
///   horizon prevented you from seeing further, and other sources suggest that
///   in practice the limit is closer to 160 km under realistic conditions. This
///   implies that making the map much larger in a realistic way would require
///   incorporating curvature, and also implies that any features that cannot
///   fit on the map would not (under realistic atmospheric conditions) be fully
///   visible from any point on Earth.  Therefore, even if we cannot represent
///   features larger than this accurately, nothing should be amiss from a
///   visual perspective, so this should not significantly impact the player
///   experience.
pub const MAX_WORLD_BLOCKS_LG: Vec2<u32> = Vec2 { x: 19, y: 19 };

/// Base two logarithm of a world size, in chunks, per dimension
/// (each dimension must be a power of 2, so the logarithm is exact).
///
/// NOTE: As an invariant, each dimension must be between 0 and
/// `[MAX_WORLD_BLOCKS_LG] - [TERRAIN_CHUNK_BLOCKS_LG]`.
///
/// NOTE: As an invariant, `(1 << ([DEFAULT_WORLD_CHUNKS_LG] +
/// [TERRAIN_CHUNK_BLOCKS_LG]))` fits in an i32 (derived from the invariant
/// on [MAX_WORLD_BLOCKS_LG]).
///
/// NOTE: As an invariant, each dimension (in chunks) must fit in a u16.
///
/// NOTE: As an invariant, the product of dimensions (in chunks) must fit in a
/// usize.
///
/// These invariants are all checked on construction of a `MapSizeLg`.
#[derive(Clone, Copy, Debug)]
pub struct MapSizeLg(Vec2<u32>);

impl MapSizeLg {
    // FIXME: We cannot use is_some() here because it is not currently marked as a
    // `const fn`.  Since being able to use conditionals in constant expressions has
    // not technically been stabilized yet, Clippy probably doesn't check for this
    // case yet.  When it can, or when is_some() is stabilized as a `const fn`,
    // we should deal with this.
    #[allow(clippy::redundant_pattern_matching)]
    /// Construct a new `MapSizeLg`, returning an error if the needed invariants
    /// do not hold and the vector otherwise.
    ///
    /// TODO: In the future, we may use unsafe code to assert to the compiler
    /// that these invariants indeed hold, safely opening up optimizations
    /// that might not otherwise be available at runtime.
    #[inline(always)]
    pub const fn new(map_size_lg: Vec2<u32>) -> Result<Self, ()> {
        // Assertion on dimensions: must be between
        // 0 and MAX_WORLD_BLOCKS_LG] - [TERRAIN_CHUNK_BLOCKS_LG
        let is_le_max = map_size_lg.x <= MAX_WORLD_BLOCKS_LG.x - TERRAIN_CHUNK_BLOCKS_LG
            && map_size_lg.y <= MAX_WORLD_BLOCKS_LG.y - TERRAIN_CHUNK_BLOCKS_LG;
        // Assertion on dimensions: chunks must fit in a u16.
        let chunks_in_range =
            /* 1u16.checked_shl(map_size_lg.x).is_some() &&
            1u16.checked_shl(map_size_lg.y).is_some(); */
            map_size_lg.x <= 16 &&
            map_size_lg.y <= 16;
        if is_le_max && chunks_in_range {
            // Assertion on dimensions: blocks must fit in a i32.
            let blocks_in_range =
                /* 1i32.checked_shl(map_size_lg.x + TERRAIN_CHUNK_BLOCKS_LG).is_some() &&
                1i32.checked_shl(map_size_lg.y + TERRAIN_CHUNK_BLOCKS_LG).is_some(); */
                map_size_lg.x + TERRAIN_CHUNK_BLOCKS_LG < 32 &&
                map_size_lg.y + TERRAIN_CHUNK_BLOCKS_LG < 32;
            // Assertion on dimensions: product of dimensions must fit in a usize.
            let chunks_product_in_range =
                if let Some(_) = 1usize.checked_shl(map_size_lg.x + map_size_lg.y) {
                    true
                } else {
                    false
                };
            if blocks_in_range && chunks_product_in_range {
                // Cleared all invariants.
                Ok(MapSizeLg(map_size_lg))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    #[inline(always)]
    /// Acquire the `MapSizeLg`'s inner vector.
    pub const fn vec(self) -> Vec2<u32> { self.0 }

    #[inline(always)]
    /// Get the size of this map in chunks.
    pub const fn chunks(self) -> Vec2<u16> { Vec2::new(1 << self.0.x, 1 << self.0.y) }

    /// Get the size of an array of the correct size to hold all chunks.
    pub const fn chunks_len(self) -> usize { 1 << (self.0.x + self.0.y) }
}

impl From<MapSizeLg> for Vec2<u32> {
    #[inline(always)]
    fn from(size: MapSizeLg) -> Self { size.vec() }
}

pub struct MapConfig<'a> {
    /// Base two logarithm of the chunk dimensions of the base map.
    /// Has no default; set explicitly during initial orthographic projection.
    pub map_size_lg: MapSizeLg,
    /// Dimensions of the window being written to.
    ///
    /// Defaults to `1 << [MapConfig::map_size_lg]`.
    pub dimensions: Vec2<usize>,
    /// x, y, and z of top left of map.
    ///
    /// Default x and y are 0.0; no reasonable default for z, so set during
    /// initial orthographic projection.
    pub focus: Vec3<f64>,
    /// Altitude is divided by gain and clamped to [0, 1]; thus, decreasing gain
    /// makes smaller differences in altitude appear larger.
    ///
    /// No reasonable default for z; set during initial orthographic projection.
    pub gain: f32,
    /// `fov` is used for shading purposes and refers to how much impact a
    /// change in the z direction has on the perceived slope relative to the
    /// same change in x and y.
    ///
    /// It is stored as cos θ in the range (0, 1\] where θ is the FOV
    /// "half-angle" used for perspective projection.  At 1.0, we treat it
    /// as the limit value for θ = 90 degrees, and use an orthographic
    /// projection.
    ///
    /// Defaults to 1.0.
    ///
    /// FIXME: This is a hack that tries to incorrectly implement a variant of
    /// perspective projection (which generates ∂P/∂x and ∂P/∂y for screen
    /// coordinate P by using the hyperbolic function \[assuming frustum of
    /// \[l, r, b, t, n, f\], rh coordinates, and output from -1 to 1 in
    /// s/t, 0 to 1 in r, and NDC is left-handed \[so visible z ranges from
    /// -n to -f\]\]):
    ///
    /// P.s(x, y, z) = -1 +  2(-n/z x -  l) / ( r -  l)
    /// P.t(x, y, z) = -1 +  2(-n/z y -  b) / ( t -  b)
    /// P.r(x, y, z) =  0 + -f(-n/z   -  1) / ( f -  n)
    ///
    /// Then arbitrarily using W_e_x = (r - l) as the width of the projected
    /// image, we have W_e_x = 2 n_e tan θ ⇒ tan Θ = (r - l) / (2n_e), for a
    /// perspective projection
    ///
    /// (where θ is the half-angle of the FOV).
    ///
    /// Taking the limit as θ → 90, we show that this degenrates to an
    /// orthogonal projection:
    ///
    /// lim{n → ∞}(-f(-n / z - 1) / (f - n)) = -(z - -n) / (f - n).
    ///
    /// (Proof not currently included, but has been formalized for the P.r case
    /// in Coq-tactic notation; the proof can be added on request, but is
    /// large and probably not well-suited to Rust documentation).
    ///
    /// For this reason, we feel free to store `fov` as cos θ in the range (0,
    /// 1\].
    ///
    /// However, `fov` does not actually work properly yet, so for now we just
    /// treat it as a visual gimmick.
    pub fov: f64,
    /// Scale is like gain, but for x and y rather than z.
    ///
    /// Defaults to (1 << world_size_lg).x / dimensions.x (NOTE: fractional, not
    /// integer, division!).
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
    pub horizons: Option<&'a [(Vec<f32>, Vec<f32>); 2]>,
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
    /// Constructs the configuration settings for an orthographic projection of
    /// a map from the top down, rendering (by default) the complete map to
    /// an image such that the chunk:pixel ratio is 1:1.
    ///
    /// Takes two arguments: the base two logarithm of the horizontal map extent
    /// (in chunks), and the z bounds of the projection.
    pub fn orthographic(map_size_lg: MapSizeLg, z_bounds: RangeInclusive<f32>) -> Self {
        assert!(z_bounds.start() <= z_bounds.end());
        // NOTE: Safe cast since map_size_lg is restricted by the prior assert.
        let dimensions = map_size_lg.chunks().map(usize::from);
        Self {
            map_size_lg,
            dimensions,
            focus: Vec3::new(0.0, 0.0, f64::from(*z_bounds.start())),
            gain: z_bounds.end() - z_bounds.start(),
            fov: 1.0,
            scale: 1.0,
            light_direction: Vec3::new(-1.2, -1.0, 0.8),
            horizons: None,

            is_basement: false,
            is_water: true,
            is_shaded: true,
            is_temperature: false,
            is_humidity: false,
            is_debug: false,
        }
    }

    /// Get the base 2 logarithm of the underlying map size.
    pub fn map_size_lg(&self) -> MapSizeLg { self.map_size_lg }

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
    #[allow(clippy::if_same_then_else)] // TODO: Pending review in #587
    #[allow(clippy::unnested_or_patterns)] // TODO: Pending review in #587
    #[allow(clippy::many_single_char_names)]
    pub fn generate(
        &self,
        sample_pos: impl Fn(Vec2<i32>) -> MapSample,
        sample_wpos: impl Fn(Vec2<i32>) -> f32,
        mut write_pixel: impl FnMut(Vec2<usize>, (u8, u8, u8, u8)),
    ) -> MapDebug {
        let MapConfig {
            map_size_lg,
            dimensions,
            focus,
            gain,
            fov,
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

        /* // NOTE: Asserting this to enable LLVM optimizations.  Ideally we should come up
        // with a principled way to do this (especially one with no runtime
        // cost).
        assert!(
            map_size_lg
                .vec()
                .cmple(&(MAX_WORLD_BLOCKS_LG - TERRAIN_CHUNK_BLOCKS_LG))
                .reduce_and()
        ); */
        let world_size = map_size_lg.chunks();

        (0..dimensions.y * dimensions.x).for_each(|chunk_idx| {
            let i = chunk_idx % dimensions.x as usize;
            let j = chunk_idx / dimensions.x as usize;

            let wposf = focus_rect + Vec2::new(i as f64, j as f64) * scale;
            let pos = wposf.map(|e: f64| e as i32);
            let wposf = wposf * chunk_size;

            let chunk_idx = if pos.reduce_partial_min() >= 0
                && pos.x < world_size.x as i32
                && pos.y < world_size.y as i32
            {
                Some(vec2_as_uniform_idx(map_size_lg, pos))
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
                .into_iter()
                .flat_map(|chunk_idx| {
                    neighbors(map_size_lg, chunk_idx).chain(iter::once(chunk_idx))
                })
                .for_each(|neighbor_posi| {
                    let neighbor_pos = uniform_idx_as_vec2(map_size_lg, neighbor_posi);
                    let neighbor_wpos = neighbor_pos.map(|e| e as f64) * chunk_size;
                    let MapSample { connections, .. } = sample_pos(neighbor_pos);
                    NEIGHBOR_DELTA
                        .iter()
                        .zip(connections.iter().flatten())
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
                            let connection_dist =
                                (dist.sqrt() - (connection.width as f64 * 0.5).max(1.0)).max(0.0);
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
            // TODO: Fix use of fov to match proper perspective projection, as described in
            // the doc comment.
            // Pointing downhill, forward
            // (index--note that (0,0,1) is backward right-handed)
            let forward_vec = Vec3::new(
                (downhill_wpos.x - wposi.x) as f64,
                ((downhill_alt - alt) * gain) as f64 * fov,
                (downhill_wpos.y - wposi.y) as f64,
            );
            // Pointing 90 degrees left (in horizontal xy) of downhill, up
            // (middle--note that (1,0,0), 90 degrees CCW backward, is right right-handed)
            let up_vec = Vec3::new(
                (cross_pos.x - wposi.x) as f64,
                ((cross_alt - alt) * gain) as f64 * fov,
                (cross_pos.y - wposi.y) as f64,
            );
            // let surface_normal = Vec3::new(fov* (f.y * u.z - f.z * u.y), -(f.x * u.z -
            // f.z * u.x), fov* (f.x * u.y - f.y * u.x)).normalized();
            // Then cross points "to the right" (upwards) on a right-handed coordinate
            // system. (right-handed coordinate system means (0, 0, 1.0) is
            // "forward" into the screen).
            let surface_normal = forward_vec.cross(up_vec).normalized();

            // TODO: Figure out if we can reimplement debugging.
            /* if is_debug {
                let quad =
                    |x: f32| ((x as f64 * QUADRANTS as f64).floor() as usize).min(QUADRANTS - 1);
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
                    let height = (height - f64::from(alt * gain)).max(0.0);
                    if angle != 0.0 && light_direction.x != 0.0 && height != 0.0 {
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
                .unwrap_or(1.0);

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
                let i_m_d = Rgb::new(1.0, 1.0, 1.0);
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
