pub mod biome;
pub mod block;
pub mod chonk;
pub mod map;
pub mod site;
pub mod sprite;
pub mod structure;

// Reexports
pub use self::{
    biome::BiomeKind,
    block::{Block, BlockKind},
    map::MapSizeLg,
    site::SiteKindMeta,
    sprite::{SpriteCfg, SpriteKind, UnlockKind},
    structure::{Structure, StructuresGroup},
};
use hashbrown::HashMap;
use roots::find_roots_cubic;
use serde::{Deserialize, Serialize};

use crate::{
    vol::{ReadVol, RectVolSize},
    volumes::vol_grid_2d::VolGrid2d,
};
use vek::*;

// TerrainChunkSize

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkSize;

/// Base two logarithm of the number of blocks along either horizontal axis of
/// a chunk.
///
/// NOTE: (1 << CHUNK_SIZE_LG) is guaranteed to fit in a u32.
///
/// NOTE: A lot of code assumes that the two dimensions are equal, so we make it
/// explicit here.
///
/// NOTE: It is highly unlikely that a value greater than 5 will work, as many
/// frontend optimizations rely on being able to pack chunk horizontal
/// dimensions into 5 bits each.
pub const TERRAIN_CHUNK_BLOCKS_LG: u32 = 5;

impl RectVolSize for TerrainChunkSize {
    const RECT_SIZE: Vec2<u32> = Vec2 {
        x: (1 << TERRAIN_CHUNK_BLOCKS_LG),
        y: (1 << TERRAIN_CHUNK_BLOCKS_LG),
    };
}

impl TerrainChunkSize {
    #[inline(always)]
    /// Convert dimensions in terms of chunks into dimensions in terms of blocks
    /// ```
    /// use vek::*;
    /// use veloren_common::terrain::TerrainChunkSize;
    ///
    /// assert_eq!(TerrainChunkSize::blocks(Vec2::new(3, 2)), Vec2::new(96, 64));
    /// ```
    pub fn blocks(chunks: Vec2<u32>) -> Vec2<u32> { chunks * Self::RECT_SIZE }

    /// Calculate the world position (i.e. in blocks) at the center of this
    /// chunk
    /// ```
    /// use vek::*;
    /// use veloren_common::terrain::TerrainChunkSize;
    ///
    /// assert_eq!(
    ///     TerrainChunkSize::center_wpos(Vec2::new(0, 2)),
    ///     Vec2::new(16, 80)
    /// );
    /// ```
    pub fn center_wpos(chunk_pos: Vec2<i32>) -> Vec2<i32> {
        chunk_pos * Self::RECT_SIZE.as_::<i32>() + Self::RECT_SIZE.as_::<i32>() / 2
    }
}

pub trait CoordinateConversions {
    fn wpos_to_cpos(&self) -> Self;
    fn cpos_to_wpos(&self) -> Self;
}

impl CoordinateConversions for Vec2<i32> {
    #[inline]
    fn wpos_to_cpos(&self) -> Self { self.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e / sz as i32) }

    #[inline]
    fn cpos_to_wpos(&self) -> Self { self.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e * sz as i32) }
}

impl CoordinateConversions for Vec2<f32> {
    #[inline]
    fn wpos_to_cpos(&self) -> Self { self.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e / sz as f32) }

    #[inline]
    fn cpos_to_wpos(&self) -> Self { self.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e * sz as f32) }
}

impl CoordinateConversions for Vec2<f64> {
    #[inline]
    fn wpos_to_cpos(&self) -> Self { self.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e / sz as f64) }

    #[inline]
    fn cpos_to_wpos(&self) -> Self { self.map2(TerrainChunkSize::RECT_SIZE, |e, sz| e * sz as f64) }
}

// TerrainChunkMeta

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainChunkMeta {
    name: Option<String>,
    biome: BiomeKind,
    alt: f32,
    tree_density: f32,
    contains_cave: bool,
    contains_river: bool,
    river_velocity: Vec3<f32>,
    temp: f32,
    humidity: f32,
    site: Option<SiteKindMeta>,
    tracks: Vec<CubicBezier3<f32>>,
    debug_points: Vec<Vec3<f32>>,
    debug_lines: Vec<LineSegment3<f32>>,
    sprite_cfgs: HashMap<Vec3<i32>, SpriteCfg>,
}

impl TerrainChunkMeta {
    pub fn new(
        name: Option<String>,
        biome: BiomeKind,
        alt: f32,
        tree_density: f32,
        contains_cave: bool,
        contains_river: bool,
        river_velocity: Vec3<f32>,
        temp: f32,
        humidity: f32,
        site: Option<SiteKindMeta>,
    ) -> Self {
        Self {
            name,
            biome,
            alt,
            tree_density,
            contains_cave,
            contains_river,
            river_velocity,
            temp,
            humidity,
            site,
            tracks: Vec::new(),
            debug_points: Vec::new(),
            debug_lines: Vec::new(),
            sprite_cfgs: HashMap::default(),
        }
    }

    pub fn void() -> Self {
        Self {
            name: None,
            biome: BiomeKind::Void,
            alt: 0.0,
            tree_density: 0.0,
            contains_cave: false,
            contains_river: false,
            river_velocity: Vec3::zero(),
            temp: 0.0,
            humidity: 0.0,
            site: None,
            tracks: Vec::new(),
            debug_points: Vec::new(),
            debug_lines: Vec::new(),
            sprite_cfgs: HashMap::default(),
        }
    }

    pub fn name(&self) -> Option<&str> { self.name.as_deref() }

    pub fn biome(&self) -> BiomeKind { self.biome }

    pub fn alt(&self) -> f32 { self.alt }

    pub fn tree_density(&self) -> f32 { self.tree_density }

    pub fn contains_cave(&self) -> bool { self.contains_cave }

    pub fn contains_river(&self) -> bool { self.contains_river }

    pub fn river_velocity(&self) -> Vec3<f32> { self.river_velocity }

    pub fn site(&self) -> Option<SiteKindMeta> { self.site }

    pub fn temp(&self) -> f32 { self.temp }

    pub fn humidity(&self) -> f32 { self.humidity }

    pub fn tracks(&self) -> &[CubicBezier3<f32>] { &self.tracks }

    pub fn add_track(&mut self, bezier: CubicBezier3<f32>) { self.tracks.push(bezier); }

    pub fn debug_points(&self) -> &[Vec3<f32>] { &self.debug_points }

    pub fn add_debug_point(&mut self, point: Vec3<f32>) { self.debug_points.push(point); }

    pub fn debug_lines(&self) -> &[LineSegment3<f32>] { &self.debug_lines }

    pub fn add_debug_line(&mut self, line: LineSegment3<f32>) { self.debug_lines.push(line); }

    pub fn sprite_cfg_at(&self, rpos: Vec3<i32>) -> Option<&SpriteCfg> {
        self.sprite_cfgs.get(&rpos)
    }

    pub fn set_sprite_cfg_at(&mut self, rpos: Vec3<i32>, sprite_cfg: SpriteCfg) {
        self.sprite_cfgs.insert(rpos, sprite_cfg);
    }
}

// Terrain type aliases

pub type TerrainChunk = chonk::Chonk<Block, TerrainChunkSize, TerrainChunkMeta>;
pub type TerrainGrid = VolGrid2d<TerrainChunk>;

impl TerrainGrid {
    /// Find a location suitable for spawning an entity near the given
    /// position (but in the same chunk).
    pub fn find_space(&self, pos: Vec3<i32>) -> Vec3<i32> {
        self.try_find_space(pos).unwrap_or(pos)
    }

    pub fn is_space(&self, pos: Vec3<i32>) -> bool {
        (0..2).all(|z| {
            self.get(pos + Vec3::unit_z() * z)
                .map_or(true, |b| !b.is_solid())
        })
    }

    pub fn try_find_space(&self, pos: Vec3<i32>) -> Option<Vec3<i32>> {
        const SEARCH_DIST: i32 = 63;
        (0..SEARCH_DIST * 2 + 1)
            .map(|i| if i % 2 == 0 { i } else { -i } / 2)
            .map(|z_diff| pos + Vec3::unit_z() * z_diff)
            .find(|pos| {
                self.get(pos - Vec3::unit_z())
                    .map_or(false, |b| b.is_filled())
                    && self.is_space(*pos)
            })
    }
}

impl TerrainChunk {
    /// Generate an all-water chunk at a specific sea level.
    pub fn water(sea_level: i32) -> TerrainChunk {
        TerrainChunk::new(
            sea_level,
            Block::new(BlockKind::Water, Rgb::zero()),
            Block::air(SpriteKind::Empty),
            TerrainChunkMeta::void(),
        )
    }

    /// Find the highest or lowest accessible position within the chunk
    pub fn find_accessible_pos(&self, spawn_wpos: Vec2<i32>, ascending: bool) -> Vec3<f32> {
        let min_z = self.get_min_z();
        let max_z = self.get_max_z();

        let pos = Vec3::new(
            spawn_wpos.x,
            spawn_wpos.y,
            if ascending { min_z } else { max_z },
        );
        (0..(max_z - min_z))
            .map(|z_diff| {
                if ascending {
                    pos + Vec3::unit_z() * z_diff
                } else {
                    pos - Vec3::unit_z() * z_diff
                }
            })
            .find(|test_pos| {
                let chunk_relative_xy = test_pos
                    .xy()
                    .map2(TerrainChunkSize::RECT_SIZE, |e, sz| e.rem_euclid(sz as i32));
                self.get(
                    Vec3::new(chunk_relative_xy.x, chunk_relative_xy.y, test_pos.z)
                        - Vec3::unit_z(),
                )
                .map_or(false, |b| b.is_filled())
                    && (0..3).all(|z| {
                        self.get(
                            Vec3::new(chunk_relative_xy.x, chunk_relative_xy.y, test_pos.z)
                                + Vec3::unit_z() * z,
                        )
                        .map_or(true, |b| !b.is_solid())
                    })
            })
            .unwrap_or(pos)
            .map(|e| e as f32)
            + 0.5
    }
}

// Terrain helper functions used across multiple crates.

/// Computes the position Vec2 of a SimChunk from an index, where the index was
/// generated by uniform_noise.
///
/// NOTE: Dimensions obey constraints on [map::MapConfig::map_size_lg].
#[inline(always)]
pub fn uniform_idx_as_vec2(map_size_lg: MapSizeLg, idx: usize) -> Vec2<i32> {
    let x_mask = (1 << map_size_lg.vec().x) - 1;
    Vec2::new((idx & x_mask) as i32, (idx >> map_size_lg.vec().x) as i32)
}

/// Computes the index of a Vec2 of a SimChunk from a position, where the index
/// is generated by uniform_noise.  NOTE: Both components of idx should be
/// in-bounds!
#[inline(always)]
pub fn vec2_as_uniform_idx(map_size_lg: MapSizeLg, idx: Vec2<i32>) -> usize {
    ((idx.y as usize) << map_size_lg.vec().x) | idx.x as usize
}

// NOTE: want to keep this such that the chunk index is in ascending order!
pub const NEIGHBOR_DELTA: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Iterate through all cells adjacent to a chunk.
#[inline(always)]
pub fn neighbors(map_size_lg: MapSizeLg, posi: usize) -> impl Clone + Iterator<Item = usize> {
    let pos = uniform_idx_as_vec2(map_size_lg, posi);
    let world_size = map_size_lg.chunks();
    NEIGHBOR_DELTA
        .iter()
        .map(move |&(x, y)| Vec2::new(pos.x + x, pos.y + y))
        .filter(move |pos| {
            pos.x >= 0 && pos.y >= 0 && pos.x < world_size.x as i32 && pos.y < world_size.y as i32
        })
        .map(move |pos| vec2_as_uniform_idx(map_size_lg, pos))
}

pub fn river_spline_coeffs(
    // _sim: &WorldSim,
    chunk_pos: Vec2<f64>,
    spline_derivative: Vec2<f32>,
    downhill_pos: Vec2<f64>,
) -> Vec3<Vec2<f64>> {
    let dxy = downhill_pos - chunk_pos;
    // Since all splines have been precomputed, we don't have to do that much work
    // to evaluate the spline.  The spline is just ax^2 + bx + c = 0, where
    //
    // a = dxy - chunk.river.spline_derivative
    // b = chunk.river.spline_derivative
    // c = chunk_pos
    let spline_derivative = spline_derivative.map(|e| e as f64);
    Vec3::new(dxy - spline_derivative, spline_derivative, chunk_pos)
}

/// Find the nearest point from a quadratic spline to this point (in terms of t,
/// the "distance along the curve" by which our spline is parameterized).  Note
/// that if t < 0.0 or t >= 1.0, we probably shouldn't be considered "on the
/// curve"... hopefully this works out okay and gives us what we want (a
/// river that extends outwards tangent to a quadratic curve, with width
/// configured by distance along the line).
pub fn quadratic_nearest_point(
    spline: &Vec3<Vec2<f64>>,
    point: Vec2<f64>,
    _line: Vec2<Vec2<f64>>, // Used for alternative distance functions below
) -> Option<(f64, Vec2<f64>, f64)> {
    //let eval_at = |t: f64| spline.x * t * t + spline.y * t + spline.z;

    // Linear

    // let line = LineSegment2 {
    //     start: line.x,
    //     end: line.y,
    // };
    // let len_sq = line.start.distance_squared(line.end);
    // let t = ((point - line.start).dot(line.end - line.start) /
    // len_sq).clamped(0.0, 1.0); let pos = line.start + (line.end - line.start)
    // * t; return Some((t, pos, pos.distance_squared(point)));

    // Quadratic

    // let curve = QuadraticBezier2 {
    //     start: line.x,
    //     ctrl: eval_at(0.5),
    //     end: line.y,
    // };
    // let (t, pos) = curve.binary_search_point_by_steps(point, 16, 0.001);
    // let t = t.clamped(0.0, 1.0);
    // let pos = curve.evaluate(t);
    // return Some((t, pos, pos.distance_squared(point)));

    // Cubic

    // let ctrl_at = |t: f64, end: f64| {
    //     let a = eval_at(end);
    //     let b = eval_at(Lerp::lerp(end, t, 0.1));
    //     let dir = (b - a).normalized();
    //     let exact = eval_at(t);
    //     a + dir * exact.distance(a)
    // };
    // let curve = CubicBezier2 {
    //     start: line.x,
    //     ctrl0: ctrl_at(0.33, 0.0),
    //     ctrl1: ctrl_at(0.66, 1.0),
    //     end: line.y,
    // };
    // let (t, pos) = curve.binary_search_point_by_steps(point, 12, 0.01);
    // let t = t.clamped(0.0, 1.0);
    // let pos = curve.evaluate(t);
    // return Some((t, pos, pos.distance_squared(point)));

    let a = spline.z.x;
    let b = spline.y.x;
    let c = spline.x.x;
    let d = point.x;
    let e = spline.z.y;
    let f = spline.y.y;
    let g = spline.x.y;
    let h = point.y;
    // This is equivalent to solving the following cubic equation (derivation is a
    // bit annoying):
    //
    // A = 2(c^2 + g^2)
    // B = 3(b * c + g * f)
    // C = ((a - d) * 2 * c + b^2 + (e - h) * 2 * g + f^2)
    // D = ((a - d) * b + (e - h) * f)
    //
    // Ax³ + Bx² + Cx + D = 0
    //
    // Once solved, this yield up to three possible values for t (reflecting minimal
    // and maximal values).  We should choose the minimal such real value with t
    // between 0.0 and 1.0.  If we fall outside those bounds, then we are
    // outside the spline and return None.
    let a_ = (c * c + g * g) * 2.0;
    let b_ = (b * c + g * f) * 3.0;
    let a_d = a - d;
    let e_h = e - h;
    let c_ = a_d * c * 2.0 + b * b + e_h * g * 2.0 + f * f;
    let d_ = a_d * b + e_h * f;
    let roots = find_roots_cubic(a_, b_, c_, d_);
    let roots = roots.as_ref();

    let min_root = roots
        .iter()
        .copied()
        .map(|root| {
            let river_point = spline.x * root * root + spline.y * root + spline.z;
            if root > 0.0 && root < 1.0 {
                (root, river_point)
            } else {
                let root = root.clamped(0.0, 1.0);
                let river_point = spline.x * root * root + spline.y * root + spline.z;
                (root, river_point)
            }
        })
        .map(|(root, river_point)| {
            let river_distance = river_point.distance_squared(point);
            (root, river_point, river_distance)
        })
        // In the (unlikely?) case that distances are equal, prefer the earliest point along the
        // river.
        .min_by(|&(ap, _, a), &(bp, _, b)| {
            (a, !(0.0..=1.0).contains(&ap), ap)
                .partial_cmp(&(b, !(0.0..=1.0).contains(&bp), bp))
                .unwrap()
        });
    min_root
}
