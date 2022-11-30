use common::{grid::Grid, terrain::TerrainChunk, trade::Good};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// World map information.  Note that currently, we always send the whole thing
/// in one go, but the structure aims to try to provide information as locally
/// as possible, so that in the future we can split up large maps into multiple
/// WorldMapMsg fragments.
///
/// TODO: Update message format to make fragmentable, allowing us to send more
/// information without running into bandwidth issues.
///
/// TODO: Add information for rivers (currently, we just prerender them on the
/// server, but this is not a great solution for LoD.  The map rendering code is
/// already set up to be able to take advantage of the river rendering being
/// split out, but the format is a little complicated for space reasons and it
/// may take some tweaking to get right, so we avoid sending it for now).
///
/// TODO: measure explicit compression schemes that might save space, e.g.
/// repeating the "small angles" optimization that works well on more detailed
/// shadow maps intended for height maps.
pub struct WorldMapMsg {
    /// Log base 2 of world map dimensions (width × height) in chunks.
    ///
    /// NOTE: Invariant: chunk count fits in a u16.
    pub dimensions_lg: Vec2<u32>,
    /// Max height (used to scale altitudes).
    pub max_height: f32,
    /// RGB+A; the alpha channel is currently unused, but will be used in the
    /// future. Entries are in the usual chunk order.
    pub rgba: Grid<u32>,
    /// Altitudes: bits 2 to 0 are unused, then bits 15 to 3 are used for
    /// altitude. The remainder are currently unused, but we have plans to
    /// use 7 bits for water depth (using an integer f7 encoding), and we
    /// will find other uses for the remaining 12 bits.
    pub alt: Grid<u32>,
    /// Horizon mapping.  This is a variant of shadow mapping that is
    /// specifically designed for height maps; it takes advantage of their
    /// regular structure (e.g. no holes) to compress all information needed
    /// to decide when to cast a sharp shadow into a single nagle, the "horizon
    /// angle."  This is the smallest angle with the ground at which light can
    /// pass through any occluders to reach the chunk, in some chosen
    /// horizontal direction.  This would not be sufficient for a more
    /// complicated 3D structure, but it works for height maps since:
    ///
    /// 1. they have no gaps, so as soon as light can shine through it will
    /// always be able to do    so, and
    /// 2. we only care about lighting from the top, and only from the east and
    /// west    (since at a large scale like this we mostly just want to
    /// handle variable sunlight;    moonlight would present more challenges
    /// but we currently have no plans to try to cast    accurate shadows in
    /// moonlight).
    ///
    /// Our chosen format is two pairs of vectors,
    /// with the first pair representing west-facing light (casting shadows on
    /// the left side)  and the second representing east-facing light
    /// (casting shadows on the east side).
    ///
    /// The pair of vectors consists of (with each vector in the usual chunk
    /// order):
    ///
    /// * Horizon angle pointing east (1 byte, scaled so 1 unit = 255° / 360).
    ///   We might consider switching to tangent if that represents the
    ///   information we care about better.
    /// * Approximate (floor) height of maximal occluder. We currently use this
    ///   to try to deliver some approximation of soft shadows, which isn't that
    ///   big a deal on the world map but is probably needed in order to ensure
    ///   smooth transitions between chunks in LoD view.  Additionally, when we
    ///   start using the shadow information to do local lighting on the world
    ///   map, we'll want a quick way to test where we can go out of shadow at
    ///   arbitrary heights (since the player and other entities cajn find
    ///   themselves far from the ground at times).  While this is only an
    ///   approximation to a proper distance map, hopefully it will give us
    ///   something  that feels reasonable enough for Veloren's style.
    ///
    /// NOTE: On compression.
    ///
    /// Horizon mapping has a lot of advantages for height maps (simple, easy to
    /// understand, doesn't require any fancy math or approximation beyond
    /// precision loss), though it loses a few of them by having to store
    /// distance to occluder as well.  However, just storing tons
    /// and tons of regular shadow maps (153 for a full day cycle, stored at
    /// irregular intervals) combined with clever explicit compression and
    /// avoiding recording sharp local shadows (preferring retracing for
    /// these), yielded a compression rate of under 3 bits per column! Since
    /// we likely want to avoid per-column shadows for worlds of the sizes we
    /// want, we'd still need to store *some* extra information to create
    /// soft shadows, but it would still be nice to try to drive down our
    /// size as much as possible given how compressible shadows of height
    /// maps seem to be in practice.  Therefore, we try to take advantage of the
    /// way existing compression algorithms tend to work to see if we can
    /// achieve significant gains without doing a lot of custom work.
    ///
    /// Specifically, since our rays are cast east/west, we expect that for each
    /// row, the horizon angles in each direction should be sequences of
    /// monotonically increasing values (as chunks approach a tall
    /// occluder), followed by sequences of no shadow, repeated
    /// until the end of the map.  Monotonic sequences and same-byte sequences
    /// are usually easy to compress and existing algorithms are more likely
    /// to be able to deal with them than jumbled data.  If we were to keep
    /// both directions in the same vector, off-the-shelf compression would
    /// probably be less effective.
    ///
    /// For related reasons, rather than storing distances as in a standard
    /// distance map (which would lead to monotonically *decreasing* values
    /// as we approached the occluder from a given direction), we store the
    /// estimated *occluder height.*  The idea here is that we replace the
    /// monotonic sequences with constant sequences, which are extremely
    /// straightforward to compress and mostly handled automatically by anything
    /// that does run-length encoding (i.e. most off-the-shelf compression
    /// algorithms).
    ///
    /// We still need to benchmark this properly, as there's no guarantee our
    /// current compression algorithms will actually work well on this data
    /// in practice.  It's possible that some other permutation (e.g. more
    /// bits reserved for "distance to occluder" in exchange for an even
    /// more predictible sequence) would end up compressing better than storing
    /// angles, or that we don't need as much precision as we currently have
    /// (256 possible angles).
    pub horizons: [(Vec<u8>, Vec<u8>); 2],
    pub sites: Vec<SiteInfo>,
    pub pois: Vec<PoiInfo>,
    /// Default chunk (representing the ocean outside the map bounds).  Sea
    /// level (used to provide a base altitude) is the lower bound of this
    /// chunk.
    pub default_chunk: Arc<TerrainChunk>,
}

pub type SiteId = common::trade::SiteId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteInfo {
    pub id: SiteId,
    pub kind: SiteKind,
    pub wpos: Vec2<i32>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum SiteKind {
    Town,
    Dungeon { difficulty: u32 },
    Castle,
    Cave,
    Tree,
    Gnarling,
    ChapelSite,
    Bridge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyInfo {
    pub id: SiteId,
    pub population: u32,
    pub stock: HashMap<Good, f32>,
    pub labor_values: HashMap<Good, f32>,
    pub values: HashMap<Good, f32>,
    pub labors: Vec<f32>,
    pub last_exports: HashMap<Good, f32>,
    pub resources: HashMap<Good, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoiInfo {
    pub kind: PoiKind,
    pub wpos: Vec2<i32>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u8)]
pub enum PoiKind {
    Peak(u32),
    Lake(u32),
}
