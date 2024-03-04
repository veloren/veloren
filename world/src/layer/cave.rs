use crate::{
    site::SiteKind,
    util::{
        close_fast as close, sampler::Sampler, FastNoise2d, RandomField, RandomPerm, SmallCache,
        StructureGen2d, LOCALITY, SQUARE_4,
    },
    Canvas, CanvasInfo, ColumnSample, Land,
};
use common::{
    generation::EntityInfo,
    terrain::{
        quadratic_nearest_point, river_spline_coeffs, Block, BlockKind, CoordinateConversions,
        SpriteKind, TerrainChunkSize,
    },
    vol::RectVolSize,
};
use itertools::Itertools;
use noise::NoiseFn;
use rand::prelude::*;
use std::{
    cmp::Ordering,
    f64::consts::PI,
    ops::{Add, Mul, Range, Sub},
};
use vek::*;

const CELL_SIZE: i32 = 1536;

#[derive(Copy, Clone)]
pub struct Node {
    pub wpos: Vec2<i32>,
    pub depth: i32,
}

fn to_cell(wpos: Vec2<i32>, level: u32) -> Vec2<i32> {
    (wpos + (level & 1) as i32 * CELL_SIZE / 4).map(|e| e.div_euclid(CELL_SIZE))
}
fn to_wpos(cell: Vec2<i32>, level: u32) -> Vec2<i32> {
    (cell * CELL_SIZE) - (level & 1) as i32 * CELL_SIZE / 4
}

const AVG_LEVEL_DEPTH: i32 = 120;
pub const LAYERS: u32 = 5;
const MIN_RADIUS: f32 = 8.0;
const MAX_RADIUS: f32 = 64.0;

fn node_at(cell: Vec2<i32>, level: u32, land: &Land) -> Option<Node> {
    let rand = RandomField::new(37 + level);

    if rand.chance(cell.with_z(0), 0.75) || level == 0 {
        let dx = RandomField::new(38 + level);
        let dy = RandomField::new(39 + level);
        let wpos = to_wpos(cell, level)
            + CELL_SIZE / 4
            + (Vec2::new(dx.get(cell.with_z(0)), dy.get(cell.with_z(0))) % CELL_SIZE as u32 / 2)
                .map(|e| e as i32);
        land.get_chunk_wpos(wpos).and_then(|chunk| {
            let depth = AVG_LEVEL_DEPTH * level as i32 - 6;

            if level > 0
                || (!chunk.near_cliffs()
                    && !chunk.river.near_water()
                    && chunk.sites.is_empty()
                    && land.get_gradient_approx(wpos) < 0.75)
            {
                Some(Node { wpos, depth })
            } else {
                None
            }
        })
    } else {
        None
    }
}

pub fn surface_entrances<'a>(land: &'a Land) -> impl Iterator<Item = Vec2<i32>> + 'a {
    let sz_cells = to_cell(land.size().as_::<i32>().cpos_to_wpos(), 0);
    (0..sz_cells.x + 1)
        .flat_map(move |x| (0..sz_cells.y + 1).map(move |y| Vec2::new(x, y)))
        .filter_map(|cell| Some(tunnel_below_from_cell(cell, 0, land)?.a.wpos))
}

#[derive(Copy, Clone)]
pub struct Tunnel {
    a: Node,
    b: Node,
    curve: f32,
}

impl Tunnel {
    fn ctrl_offset(&self) -> Vec2<f32> {
        let start = self.a.wpos.map(|e| e as f64 + 0.5);
        let end = self.b.wpos.map(|e| e as f64 + 0.5);

        ((end - start) * 0.5 + ((end - start) * 0.5).rotated_z(PI / 2.0) * 6.0 * self.curve as f64)
            .map(|e| e as f32)
    }

    fn possibly_near(&self, wposf: Vec2<f64>, threshold: f64) -> Option<(f64, Vec2<f64>, f64)> {
        let start = self.a.wpos.map(|e| e as f64 + 0.5);
        let end = self.b.wpos.map(|e| e as f64 + 0.5);
        if let Some((t, closest, _)) = quadratic_nearest_point(
            &river_spline_coeffs(start, self.ctrl_offset(), end),
            wposf,
            Vec2::new(start, end),
        ) {
            let dist2 = closest.distance_squared(wposf);
            if dist2 < (MAX_RADIUS as f64 + threshold).powi(2) {
                Some((t, closest, dist2.sqrt()))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn z_range_at(
        &self,
        wposf: Vec2<f64>,
        info: CanvasInfo,
    ) -> Option<(Range<i32>, f32, f32, f32)> {
        let _start = self.a.wpos.map(|e| e as f64 + 0.5);
        let _end = self.b.wpos.map(|e| e as f64 + 0.5);
        if let Some((t, closest, dist)) = self.possibly_near(wposf, 1.0) {
            let horizontal = Lerp::lerp_unclamped(
                MIN_RADIUS as f64,
                MAX_RADIUS as f64,
                (info.index().noise.cave_fbm_nz.get(
                    (closest.with_z(info.land().get_alt_approx(self.a.wpos) as f64) / 256.0)
                        .into_array(),
                ) + 0.5)
                    .clamped(0.0, 1.0)
                    .powf(3.0),
            );
            let vertical = Lerp::lerp_unclamped(
                MIN_RADIUS as f64,
                MAX_RADIUS as f64,
                (info.index().noise.cave_fbm_nz.get(
                    (closest.with_z(info.land().get_alt_approx(self.b.wpos) as f64) / 256.0)
                        .into_array(),
                ) + 0.5)
                    .clamped(0.0, 1.0)
                    .powf(3.0),
            );
            let height_here = (1.0 - dist / horizontal).max(0.0).powf(0.3) * vertical;

            if height_here > 0.0 {
                let z_offs = info
                    .index()
                    .noise
                    .cave_fbm_nz
                    .get((wposf / 512.0).into_array())
                    * 96.0
                    * ((1.0 - (t - 0.5).abs() * 2.0) * 8.0).min(1.0);
                let alt_here = info.land().get_alt_approx(closest.map(|e| e as i32));
                let base = (Lerp::lerp_unclamped(
                    alt_here as f64 - self.a.depth as f64,
                    alt_here as f64 - self.b.depth as f64,
                    t,
                ) + z_offs)
                    .min(alt_here as f64);
                Some((
                    (base - height_here * 0.3) as i32..(base + height_here * 1.35) as i32,
                    horizontal as f32,
                    vertical as f32,
                    dist as f32,
                ))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn biome_at(&self, wpos: Vec3<i32>, info: &CanvasInfo) -> Biome {
        let Some(col) = info.col_or_gen(wpos.xy()) else {
            return Biome::default();
        };

        // Below the ground
        let below = ((col.alt - wpos.z as f32) / (AVG_LEVEL_DEPTH as f32 * LAYERS as f32 * 0.5))
            .clamped(0.0, 1.0);
        let depth = (col.alt - wpos.z as f32) / (AVG_LEVEL_DEPTH as f32 * LAYERS as f32);
        let underground = ((col.alt - wpos.z as f32) / 80.0 - 1.0).clamped(0.0, 1.0);

        // TODO think about making rate of change of humidity and temp noise higher to
        // effectively increase biome size
        let humidity = Lerp::lerp_unclamped(
            col.humidity,
            FastNoise2d::new(41)
                .get(wpos.xy().map(|e| e as f64 / 768.0))
                .mul(1.2),
            below,
        );

        let temp = Lerp::lerp_unclamped(
            col.temp * 2.0,
            FastNoise2d::new(42)
                .get(wpos.xy().map(|e| e as f64 / 1536.0))
                .mul(1.15)
                .mul(2.0)
                .sub(1.0)
                .add(
                    ((col.alt - wpos.z as f32) / (AVG_LEVEL_DEPTH as f32 * LAYERS as f32 * 0.6))
                        .clamped(0.0, 2.0),
                ),
            below,
        );

        let mineral = FastNoise2d::new(43)
            .get(wpos.xy().map(|e| e as f64 / 256.0))
            .mul(1.15)
            .mul(0.5)
            .add(
                ((col.alt - wpos.z as f32) / (AVG_LEVEL_DEPTH as f32 * LAYERS as f32))
                    .clamped(0.0, 1.5),
            );

        let [
            barren,
            mushroom,
            fire,
            leafy,
            dusty,
            icy,
            snowy,
            crystal,
            sandy,
        ] = {
            // Default biome, no other conditions apply
            let barren = 0.005;
            // Mushrooms grow underground and thrive in a humid environment with moderate
            // temperatures
            let mushroom = underground
                * close(humidity, 1.0, 0.7, 4)
                * close(temp, 1.5, 1.2, 4)
                * close(depth, 0.9, 0.65, 4);
            // Extremely hot and dry areas deep underground
            let fire = underground
                * close(humidity, 0.0, 0.6, 4)
                * close(temp, 2.0, 1.4, 4)
                * close(depth, 1.0, 0.45, 4);
            // Overgrown with plants that need a moderate climate to survive
            let leafy = underground
                * close(humidity, 0.8, 0.9, 4)
                * close(temp, 0.8, 1.0, 4)
                * close(depth, 0.1, 0.7, 4);
            // Cool temperature, dry and devoid of value
            let dusty = close(humidity, 0.0, 0.5, 4)
                * close(temp, -0.3, 0.7, 4)
                * close(depth, 0.5, 0.5, 4);
            // Deep underground and freezing cold
            let icy = underground
                * close(temp, -1.5, 2.0, 4)
                * close(depth, 0.9, 0.55, 4)
                * close(humidity, 1.0, 0.75, 4);
            // Rocky cold cave that appear near the surface
            let snowy = close(temp, -0.7, 0.4, 4) * close(depth, 0.0, 0.4, 4);
            // Crystals grow deep underground in areas rich with minerals. They are present
            // in areas with colder temperatures and low humidity
            let crystal = underground
                * close(humidity, 0.0, 0.5, 4)
                * close(temp, -0.9, 0.9, 4)
                * close(depth, 1.0, 0.55, 4)
                * close(mineral, 2.0, 1.25, 4);
            // Hot, dry and shallow
            let sandy =
                close(humidity, 0.0, 0.5, 4) * close(temp, 1.0, 0.9, 4) * close(depth, 0.0, 0.6, 4);

            let biomes = [
                barren, mushroom, fire, leafy, dusty, icy, snowy, crystal, sandy,
            ];
            let max = biomes
                .into_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap();
            biomes.map(|e| (e / max).powf(3.0))
        };

        Biome {
            humidity,
            mineral,
            barren,
            mushroom,
            fire,
            leafy,
            dusty,
            icy,
            snowy,
            crystal,
            sandy,
            depth,
        }
    }

    pub fn nodes(&self) -> (&Node, &Node) { (&self.a, &self.b) }
}

pub(crate) fn tunnels_at<'a>(
    wpos: Vec2<i32>,
    level: u32,
    land: &'a Land,
) -> impl Iterator<Item = Tunnel> + 'a {
    let rand = RandomField::new(37 + level);
    let col_cell = to_cell(wpos - CELL_SIZE / 4, level);
    LOCALITY
        .into_iter()
        .filter_map(move |rpos| {
            let current_cell_pos = col_cell + rpos;
            Some(current_cell_pos).zip(node_at(current_cell_pos, level, land))
        })
        .flat_map(move |(current_cell_pos, current_cell)| {
            [Vec2::new(1, 1), Vec2::new(1, -1)]
                .into_iter()
                .filter(move |rpos| {
                    let mid = (current_cell_pos * 2 + rpos) / 2;
                    rand.chance(mid.with_z(0), 0.5) ^ (rpos.y == -1)
                })
                .chain([Vec2::new(1, 0), Vec2::new(0, 1)])
                .filter_map(move |rpos| {
                    let other_cell_pos = current_cell_pos + rpos;
                    Some(other_cell_pos).zip(node_at(other_cell_pos, level, land))
                })
                .filter(move |(other_cell_pos, _)| {
                    rand.chance((current_cell_pos + other_cell_pos).with_z(7), 0.3)
                })
                .map(move |(_other_cell_pos, other_cell)| Tunnel {
                    a: current_cell,
                    b: other_cell,
                    curve: RandomField::new(13)
                        .get_f32(current_cell.wpos.with_z(0))
                        .powf(0.25)
                        .mul(
                            if RandomField::new(14).chance(current_cell.wpos.with_z(0), 0.5) {
                                1.0
                            } else {
                                -1.0
                            },
                        ),
                })
        })
}

fn tunnel_below_from_cell(cell: Vec2<i32>, level: u32, land: &Land) -> Option<Tunnel> {
    let wpos = to_wpos(cell, level);
    Some(Tunnel {
        a: node_at(to_cell(wpos, level), level, land)?,
        b: node_at(to_cell(wpos + CELL_SIZE / 2, level + 1), level + 1, land)?,
        curve: 0.0,
    })
}

fn tunnels_down_from<'a>(
    wpos: Vec2<i32>,
    level: u32,
    land: &'a Land,
) -> impl Iterator<Item = Tunnel> + 'a {
    let col_cell = to_cell(wpos, level);
    LOCALITY
        .into_iter()
        .filter_map(move |rpos| tunnel_below_from_cell(col_cell + rpos, level, land))
}

fn all_tunnels_at<'a>(
    wpos2d: Vec2<i32>,
    _info: &'a CanvasInfo,
    land: &'a Land,
) -> impl Iterator<Item = (u32, Tunnel)> + 'a {
    (1..LAYERS + 1).flat_map(move |level| {
        tunnels_at(wpos2d, level, land)
            .chain(tunnels_down_from(wpos2d, level - 1, land))
            .map(move |tunnel| (level, tunnel))
    })
}

fn tunnel_bounds_at_from<'a>(
    wpos2d: Vec2<i32>,
    info: &'a CanvasInfo,
    _land: &'a Land,
    tunnels: impl Iterator<Item = (u32, Tunnel)> + 'a,
) -> impl Iterator<Item = (u32, Range<i32>, f32, f32, f32, Tunnel)> + 'a {
    let wposf = wpos2d.map(|e| e as f64 + 0.5);
    info.col_or_gen(wpos2d)
        .map(move |col| {
            let col_alt = col.alt;
            let col_water_dist = col.water_dist;
            tunnels.filter_map(move |(level, tunnel)| {
                let (z_range, horizontal, vertical, dist) = tunnel.z_range_at(wposf, *info)?;
                // Avoid cave entrances intersecting water
                let z_range = Lerp::lerp_unclamped(
                    z_range.end,
                    z_range.start,
                    1.0 - (1.0
                        - ((col_water_dist.unwrap_or(1000.0) - 4.0).max(0.0) / 32.0)
                            .clamped(0.0, 1.0))
                        * (1.0 - ((col_alt - z_range.end as f32 - 4.0) / 8.0).clamped(0.0, 1.0)),
                )..z_range.end;
                if z_range.end - z_range.start > 0 {
                    Some((level, z_range, horizontal, vertical, dist, tunnel))
                } else {
                    None
                }
            })
        })
        .into_iter()
        .flatten()
}

pub fn tunnel_bounds_at<'a>(
    wpos2d: Vec2<i32>,
    info: &'a CanvasInfo,
    land: &'a Land,
) -> impl Iterator<Item = (u32, Range<i32>, f32, f32, f32, Tunnel)> + 'a {
    tunnel_bounds_at_from(wpos2d, info, land, all_tunnels_at(wpos2d, info, land))
}

pub fn apply_caves_to(canvas: &mut Canvas, rng: &mut impl Rng) {
    let info = canvas.info();
    let land = info.land();

    let diagonal = (TerrainChunkSize::RECT_SIZE.map(|e| e * e).sum() as f32).sqrt() as f64;
    let tunnels = all_tunnels_at(
        info.wpos() + TerrainChunkSize::RECT_SIZE.map(|e| e as i32) / 2,
        &info,
        &land,
    )
    .filter(|(_, tunnel)| {
        SQUARE_4
            .into_iter()
            .map(|rpos| info.wpos() + rpos * TerrainChunkSize::RECT_SIZE.map(|e| e as i32))
            .any(|wpos| {
                tunnel
                    .possibly_near(wpos.map(|e| e as f64), diagonal + 1.0)
                    .is_some()
            })
    })
    .collect::<Vec<_>>();

    if !tunnels.is_empty() {
        let giant_tree_dist = info
            .chunk
            .sites
            .iter()
            .filter_map(|site| {
                if let SiteKind::GiantTree(t) = &info.index.sites.get(*site).kind {
                    Some(t.origin.distance_squared(info.wpos) as f32 / t.radius().powi(2))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        let mut structure_cache = SmallCache::default();
        canvas.foreach_col(|canvas, wpos2d, col| {
            let tunnel_bounds =
                tunnel_bounds_at_from(wpos2d, &info, &land, tunnels.iter().copied())
                    .collect::<Vec<_>>();

            // First, clear out tunnels
            for (_, z_range, _, _, _, _) in &tunnel_bounds {
                for z in z_range.start..z_range.end.min(col.alt as i32 + 1) {
                    canvas.set(wpos2d.with_z(z), Block::empty());
                }
            }

            let z_ranges = &tunnel_bounds
                .iter()
                .map(|(_, z_range, _, _, _, _)| z_range.clone())
                .collect_vec();

            let structure_seeds = StructureGen2d::new(34537, 24, 8).get(wpos2d);
            for (level, z_range, horizontal, vertical, dist, tunnel) in tunnel_bounds {
                write_column(
                    canvas,
                    col,
                    level,
                    wpos2d,
                    z_range.clone(),
                    z_ranges,
                    tunnel,
                    (horizontal, vertical, dist),
                    giant_tree_dist,
                    &mut structure_cache,
                    &structure_seeds,
                    rng,
                );
            }
        });
    }
}

#[allow(dead_code)]
#[derive(Default, Clone)]
pub struct Biome {
    humidity: f32,
    pub barren: f32,
    mineral: f32,
    pub mushroom: f32,
    pub fire: f32,
    pub leafy: f32,
    pub dusty: f32,
    pub icy: f32,
    pub snowy: f32,
    pub crystal: f32,
    pub sandy: f32,
    depth: f32,
}

#[derive(Clone)]
enum CaveStructure {
    Mushroom(Mushroom),
    Crystal(CrystalCluster),
    Flower(Flower),
    GiantRoot {
        pos: Vec3<i32>,
        radius: f32,
        height: f32,
    },
}

#[derive(Clone)]
struct Mushroom {
    pos: Vec3<i32>,
    stalk: f32,
    head_color: Rgb<u8>,
}

#[derive(Clone)]
struct Crystal {
    dir: Vec3<f32>,
    length: f32,
    radius: f32,
}

#[derive(Clone)]
struct CrystalCluster {
    pos: Vec3<i32>,
    crystals: Vec<Crystal>,
    color: Rgb<u8>,
}

#[derive(Clone)]
struct Flower {
    pos: Vec3<i32>,
    stalk: f32,
    petals: usize,
    petal_height: f32,
    petal_radius: f32,
    // rotation: Mat3<f32>,
}

fn write_column<R: Rng>(
    canvas: &mut Canvas,
    col: &ColumnSample,
    level: u32,
    wpos2d: Vec2<i32>,
    z_range: Range<i32>,
    z_ranges: &[Range<i32>],
    tunnel: Tunnel,
    dimensions: (f32, f32, f32),
    giant_tree_dist: f32,
    structure_cache: &mut SmallCache<Vec3<i32>, Option<CaveStructure>>,
    structure_seeds: &[(Vec2<i32>, u32); 9],
    rng: &mut R,
) {
    let info = canvas.info();

    // Exposed to the sky, or some other void above
    let void_above = !canvas.get(wpos2d.with_z(z_range.end)).is_filled();
    let void_below = !canvas.get(wpos2d.with_z(z_range.start - 1)).is_filled();
    // Exposed to the sky
    let sky_above = z_range.end as f32 > col.alt;
    let cavern_height = (z_range.end - z_range.start) as f32;
    let (cave_width, max_height, dist_cave_center) = dimensions;
    let biome = tunnel.biome_at(wpos2d.with_z(z_range.start), &info);

    // Get the range, if there is any, where the current cave overlaps with other
    // caves. Right now this is only used to prevent ceiling cover from being
    // place
    let overlap = z_ranges.iter().find_map(|other_z_range| {
        if *other_z_range == z_range {
            return None;
        }
        let start = z_range.start.max(other_z_range.start);
        let end = z_range.end.min(other_z_range.end);
        let min = z_range.start.min(other_z_range.start);
        if start < end { Some(min..end) } else { None }
    });

    let stalactite = {
        FastNoise2d::new(35)
            .get(wpos2d.map(|e| e as f64 / 8.0))
            .sub(0.5)
            .max(0.0)
            .mul(2.0)
            .powi(if biome.icy > 0.7 {2} else {1})
            // No stalactites near entrances
            .mul(((col.alt - z_range.end as f32) / 32.0).clamped(0.0, 1.0))
            // Prevent stalactites from blocking cave
            .mul(((cave_width + max_height) / 40.0).clamped(0.0, 1.0))
            // Scale with cavern height, sandy and icy biomes have bigger stalactites
            .mul(8.0 + cavern_height * (0.4 + (biome.sandy - 0.5).max(biome.icy - 0.6).max(0.0)))
    };

    let ceiling_cover = if biome.leafy > 0.3
        || biome.mushroom > 0.5
        || biome.icy > 0.6
        || biome.sandy > 0.4
        || biome.fire > 0.4
        || biome.crystal > 0.75
    {
        // 1.0 because at some point we maybe want to use some noise value here instead
        1.0.mul(((col.alt - z_range.end as f32) / 32.0).clamped(0.0, 1.0))
            .mul(max_height * (dist_cave_center / cave_width).powf(3.33))
            .max(1.0)
            .sub(
                if col.marble_mid
                    > biome
                        .fire
                        .max(biome.icy - 0.6)
                        .max(biome.sandy - 0.3)
                        .max(biome.leafy - 0.4)
                        .max(biome.mushroom - 0.4)
                        .max(biome.crystal - 0.5)
                {
                    max_height * col.marble_mid
                } else {
                    0.0
                },
            )
            .max(0.0)
    } else {
        0.0
    };

    let basalt = if biome.fire > 0.5 {
        FastNoise2d::new(36)
            .get(wpos2d.map(|e| e as f64 / 40.0))
            .mul(1.1)
            .sub(0.5)
            .max(0.0)
            .mul(((cave_width + max_height) / 48.0).clamped(0.0, 1.0))
            .mul(6.0 + cavern_height * 0.6)
            .mul((biome.fire - 0.5).powi(3) * 8.0)
    } else {
        0.0
    };

    let lava = if biome.fire > 0.5 {
        FastNoise2d::new(37)
            .get(wpos2d.map(|e| e as f64 / 32.0))
            .mul(0.5)
            .abs()
            .sub(0.2)
            .min(0.0)
            // .mul((biome.temp as f64 - 1.5).mul(30.0).clamped(0.0, 1.0))
            .mul((cave_width / 16.0).clamped(0.5, 1.0))
            .mul((cave_width / (MAX_RADIUS - 16.0)).clamped(1.0, 1.25))
            .mul((biome.fire - 0.5).mul(30.0).clamped(0.0, 1.0))
            .mul(64.0)
            .max(-32.0)
    } else {
        0.0
    };

    let bump = if biome
        .sandy
        .max(biome.dusty)
        .max(biome.leafy)
        .max(biome.barren)
        > 0.5
    {
        FastNoise2d::new(38)
            .get(wpos2d.map(|e| e as f64 / 4.0))
            .mul(1.15)
            .add(1.0)
            .mul(0.5)
            .mul(((col.alt - z_range.end as f32) / 16.0).clamped(0.0, 1.0))
            .mul({
                let (val, total) = [
                    (biome.sandy, 0.9),
                    (biome.dusty, 0.5),
                    (biome.leafy, 0.6),
                    (biome.barren, 0.6),
                ]
                .into_iter()
                .fold((0.0, 0.0), |a, x| (a.0 + x.0.max(0.0) * x.1, a.1 + x.1));
                val / total
            })
            .mul(cavern_height * 0.2)
            .clamped(0.0, 4.0)
    } else {
        0.0
    };

    let rand = RandomField::new(37 + level);

    let is_ice = biome.icy + col.marble * 0.2 > 0.5 && col.marble > 0.6;
    let is_snow = biome.snowy + col.marble_mid * 0.2 > 0.5 && col.marble_mid > 0.6;

    let (has_stalagmite, has_stalactite) = if biome.icy > 0.5 {
        (col.marble_mid > 0.6, col.marble_mid < 0.5)
    } else {
        (true, true)
    };

    let stalagmite_only = if has_stalagmite && !has_stalactite {
        0.6f32
    } else {
        0.0f32
    };

    // Don't cover stalgmites in ice biome with surface block
    let dirt = if biome.icy > 0.5 && has_stalagmite && stalactite > 3.0 {
        0
    } else {
        1 + (!is_ice) as i32 + is_snow as i32
    };
    let bedrock = z_range.start + lava as i32;
    let base = bedrock + (stalactite * (0.4 + stalagmite_only)) as i32;
    let floor = base + dirt + bump as i32;
    let ceiling =
        z_range.end - (stalactite * has_stalactite as i32 as f32).max(ceiling_cover) as i32;

    let get_ceiling_drip = |wpos: Vec2<i32>, freq: f64, length: f32| {
        let wposf = wpos.map(|e| e as f32);
        let wposf = wposf + wposf.yx() * 1.1;
        let dims = Vec2::new(11.0, 32.0);
        let posf = wposf + Vec2::unit_y() * (wposf.x / dims.x).floor() * 89.0 / dims;
        let pos = posf.map(|e| e.floor() as i32);
        if rand.chance(pos.with_z(73), freq as f32) {
            let drip_length = ((posf.y.fract() - 0.5).abs() * 2.0 * dims.y)
                .mul(length * 0.01)
                .powf(2.0)
                .min(length);

            if (posf.x.fract() * 2.0 - 1.0).powi(2) < 1.0 {
                drip_length
            } else {
                0.0
            }
        } else {
            0.0
        }
    };

    let ceiling_drip = ceiling
        - if !void_above && !sky_above {
            let c = if biome.mushroom > 0.9 && ceiling_cover > 0.0 {
                Some((0.07, 7.0))
            } else if biome.icy > 0.9 {
                Some((0.1 * col.marble_mid as f64, 9.0))
            } else {
                None
            };
            if let Some((freq, length)) = c {
                get_ceiling_drip(wpos2d, freq, length).max(get_ceiling_drip(
                    wpos2d.yx(),
                    freq,
                    length,
                )) as i32
            } else {
                0
            }
        } else {
            0
        };

    let structures = structure_seeds
        .iter()
        .filter_map(|(wpos2d, seed)| {
            let structure = structure_cache.get(wpos2d.with_z(tunnel.a.depth), |_| {
                let mut rng = RandomPerm::new(*seed);
                let (z_range, horizontal, vertical, _) =
                    tunnel.z_range_at(wpos2d.map(|e| e as f64 + 0.5), info)?;
                let pos = wpos2d.with_z(z_range.start);
                let biome = tunnel.biome_at(pos, &info);
                let tunnel_intersection = || {
                    tunnel_bounds_at(pos.xy(), &info, &info.land())
                        .any(|(_, z_range, _, _, _, _)| z_range.contains(&(z_range.start - 1)))
                };

                if biome.mushroom > 0.7
                    && vertical > 16.0
                    && rng.gen_bool(
                        0.5 * close(vertical, MAX_RADIUS, MAX_RADIUS - 16.0, 2) as f64
                            * close(biome.mushroom, 1.0, 0.7, 1) as f64,
                    )
                {
                    if tunnel_intersection() {
                        return None;
                    }
                    let purp = rng.gen_range(0..50);
                    Some(CaveStructure::Mushroom(Mushroom {
                        pos,
                        stalk: 8.0
                            + rng.gen::<f32>().powf(2.0)
                                * (z_range.end - z_range.start - 8) as f32
                                * 0.75,
                        head_color: Rgb::new(
                            40 + purp,
                            rng.gen_range(60..120),
                            rng.gen_range(80..200) + purp,
                        ),
                    }))
                } else if biome.crystal > 0.5
                    && rng.gen_bool(0.4 * close(biome.crystal, 1.0, 0.7, 2) as f64)
                {
                    if tunnel_intersection() {
                        return None;
                    }
                    let on_ground = rng.gen_bool(0.6);
                    let pos = wpos2d.with_z(if on_ground {
                        z_range.start
                    } else {
                        z_range.end
                    });

                    let mut crystals: Vec<Crystal> = Vec::new();
                    let max_length = (48.0 * close(vertical, MAX_RADIUS, MAX_RADIUS, 1)).max(12.0);
                    let length = rng.gen_range(8.0..max_length);
                    let radius =
                        Lerp::lerp(2.0, 4.5, length / max_length + rng.gen_range(-0.1..0.1));
                    let dir = Vec3::new(
                        rng.gen_range(-3.0..3.0),
                        rng.gen_range(-3.0..3.0),
                        rng.gen_range(0.5..10.0) * if on_ground { 1.0 } else { -1.0 },
                    )
                    .normalized();

                    crystals.push(Crystal {
                        dir,
                        length,
                        radius,
                    });

                    (0..4).for_each(|_| {
                        crystals.push(Crystal {
                            dir: Vec3::new(
                                rng.gen_range(-1.0..1.0),
                                rng.gen_range(-1.0..1.0),
                                (dir.z + rng.gen_range(-0.2..0.2)).clamped(0.0, 1.0),
                            ),
                            length: length * rng.gen_range(0.3..0.8),
                            radius: (radius * rng.gen_range(0.5..0.8)).max(1.0),
                        });
                    });

                    let purple = rng.gen_range(25..75);
                    let blue = (rng.gen_range(45.0..75.0) * biome.icy) as u8;

                    Some(CaveStructure::Crystal(CrystalCluster {
                        pos,
                        crystals,
                        color: Rgb::new(
                            255 - blue * 2,
                            255 - blue - purple,
                            200 + rng.gen_range(25..55),
                        ),
                    }))
                } else if biome.leafy > 0.8
                    && vertical > 16.0
                    && horizontal > 16.0
                    && rng.gen_bool(
                        0.125
                            * (close(vertical, MAX_RADIUS, MAX_RADIUS - 16.0, 2)
                                * close(horizontal, MAX_RADIUS, MAX_RADIUS - 16.0, 2)
                                * biome.leafy) as f64,
                    )
                {
                    if tunnel_intersection() {
                        return None;
                    }
                    let petal_radius = rng.gen_range(8.0..16.0);
                    Some(CaveStructure::Flower(Flower {
                        pos,
                        stalk: 6.0
                            + rng.gen::<f32>().powf(2.0)
                                * (z_range.end - z_range.start - 8) as f32
                                * 0.75,
                        petals: rng.gen_range(1..5) * 2 + 1,
                        petal_height: 0.4 * petal_radius * (1.0 + rng.gen::<f32>().powf(2.0)),
                        petal_radius,
                    }))
                } else if (biome.leafy > 0.7 || giant_tree_dist > 0.0)
                    && rng.gen_bool(
                        (0.5 * close(biome.leafy, 1.0, 0.5, 1).max(1.0 + giant_tree_dist) as f64)
                            .clamped(0.0, 1.0),
                    )
                {
                    if tunnel_intersection() {
                        return None;
                    }
                    Some(CaveStructure::GiantRoot {
                        pos,
                        radius: rng.gen_range(
                            2.5..(3.5
                                + close(vertical, MAX_RADIUS, MAX_RADIUS / 2.0, 2) * 3.0
                                + close(horizontal, MAX_RADIUS, MAX_RADIUS / 2.0, 2) * 3.0),
                        ),
                        height: (z_range.end - z_range.start) as f32,
                    })
                } else {
                    None
                }
            });

            structure
                .as_ref()
                .map(|structure| (*seed, structure.clone()))
        })
        .collect_vec();
    let get_structure = |wpos: Vec3<i32>, dynamic_rng: &mut R| {
        let warp = |wposf: Vec3<f64>, freq: f64, amp: Vec3<f32>, seed: u32| -> Option<Vec3<f32>> {
            let xy = wposf.xy();
            let xz = Vec2::new(wposf.x, wposf.z);
            let yz = Vec2::new(wposf.y, wposf.z);
            Some(
                Vec3::new(
                    FastNoise2d::new(seed).get(yz * freq),
                    FastNoise2d::new(seed).get(xz * freq),
                    FastNoise2d::new(seed).get(xy * freq),
                ) * amp,
            )
        };
        for (seed, structure) in structures.iter() {
            let seed = *seed;
            match structure {
                CaveStructure::Mushroom(mushroom) => {
                    let wposf = wpos.map(|e| e as f64);
                    let warp_freq = 1.0 / 32.0;
                    let warp_amp = Vec3::new(12.0, 12.0, 12.0);
                    let warp_offset = warp(wposf, warp_freq, warp_amp, seed)?;
                    let wposf_warped = wposf.map(|e| e as f32)
                        + warp_offset
                            * (wposf.z as f32 - mushroom.pos.z as f32)
                                .mul(0.1)
                                .clamped(0.0, 1.0);

                    let rpos = wposf_warped - mushroom.pos.map(|e| e as f32);

                    let stalk_radius = 2.5f32;
                    let head_radius = 12.0f32;
                    let head_height = 14.0;

                    let dist_sq = rpos.xy().magnitude_squared();
                    if dist_sq < head_radius.powi(2) {
                        let dist = dist_sq.sqrt();
                        let head_dist = ((rpos - Vec3::unit_z() * mushroom.stalk)
                            / Vec2::broadcast(head_radius).with_z(head_height))
                        .magnitude();

                        let stalk = mushroom.stalk
                            + Lerp::lerp_unclamped(head_height * 0.5, 0.0, dist / head_radius);

                        // Head
                        if rpos.z > stalk
                            && rpos.z <= mushroom.stalk + head_height
                            && dist
                                < head_radius
                                    * (1.0 - (rpos.z - mushroom.stalk) / head_height).powf(0.125)
                        {
                            if head_dist < 0.85 {
                                let radial = (rpos.x.atan2(rpos.y) * 10.0).sin() * 0.5 + 0.5;
                                let block_kind = if dynamic_rng.gen_bool(0.1) {
                                    BlockKind::GlowingMushroom
                                } else {
                                    BlockKind::Rock
                                };
                                return Some(Block::new(
                                    block_kind,
                                    Rgb::new(
                                        30,
                                        120 + (radial * 40.0) as u8,
                                        180 - (radial * 40.0) as u8,
                                    ),
                                ));
                            } else if head_dist < 1.0 {
                                return Some(Block::new(BlockKind::Wood, mushroom.head_color));
                            }
                        }

                        if rpos.z <= mushroom.stalk + head_height - 1.0
                            && dist_sq
                                < (stalk_radius * Lerp::lerp(1.5, 0.75, rpos.z / mushroom.stalk))
                                    .powi(2)
                        {
                            // Stalk
                            return Some(Block::new(BlockKind::Wood, Rgb::new(25, 60, 90)));
                        } else if ((mushroom.stalk - 0.1)..(mushroom.stalk + 0.9)).contains(&rpos.z) // Hanging orbs
                    && dist > head_radius * 0.85
                    && dynamic_rng.gen_bool(0.1)
                        {
                            use SpriteKind::*;
                            let sprites = if dynamic_rng.gen_bool(0.1) {
                                &[Beehive, Lantern] as &[_]
                            } else {
                                &[MycelBlue, MycelBlue] as &[_]
                            };
                            return Some(Block::air(*sprites.choose(dynamic_rng).unwrap()));
                        }
                    }
                },
                CaveStructure::Crystal(cluster) => {
                    let wposf = wpos.map(|e| e as f32);
                    let cluster_pos = cluster.pos.map(|e| e as f32);
                    for crystal in &cluster.crystals {
                        let line = LineSegment3 {
                            start: cluster_pos,
                            end: cluster_pos + crystal.dir * crystal.length,
                        };

                        let projected = line.projected_point(wposf);
                        let dist_sq = projected.distance_squared(wposf);
                        if dist_sq < crystal.radius.powi(2) {
                            let rpos = wposf - cluster_pos;
                            let line_length = line.start.distance_squared(line.end);
                            let taper = if line_length < 0.001 {
                                0.0
                            } else {
                                rpos.dot(line.end - line.start) / line_length
                            };

                            let peak_cutoff = 0.8;
                            let taper_factor = 0.55;
                            let peak_taper = 0.3;

                            let crystal_radius = if taper > peak_cutoff {
                                let taper = (taper - peak_cutoff) * 5.0;
                                Lerp::lerp_unclamped(
                                    crystal.radius * taper_factor,
                                    crystal.radius * peak_taper,
                                    taper,
                                )
                            } else {
                                let taper = taper * 1.25;
                                Lerp::lerp_unclamped(
                                    crystal.radius,
                                    crystal.radius * taper_factor,
                                    taper,
                                )
                            };

                            if dist_sq < crystal_radius.powi(2) {
                                if dist_sq / crystal_radius.powi(2) > 0.75 {
                                    return Some(Block::new(BlockKind::GlowingRock, cluster.color));
                                } else {
                                    return Some(Block::new(BlockKind::Rock, cluster.color));
                                }
                            }
                        }
                    }
                },
                CaveStructure::Flower(flower) => {
                    let wposf = wpos.map(|e| e as f64);
                    let warp_freq = 1.0 / 16.0;
                    let warp_amp = Vec3::new(8.0, 8.0, 8.0);
                    let warp_offset = warp(wposf, warp_freq, warp_amp, seed)?;
                    let wposf_warped = wposf.map(|e| e as f32)
                        + warp_offset
                            * (wposf.z as f32 - flower.pos.z as f32)
                                .mul(1.0 / flower.stalk)
                                .sub(1.0)
                                .min(0.0)
                                .abs()
                                .clamped(0.0, 1.0);
                    let rpos = wposf_warped - flower.pos.map(|e| e as f32);

                    let stalk_radius = 2.5f32;
                    let petal_thickness = 2.5;
                    let dist_sq = rpos.xy().magnitude_squared();
                    if rpos.z < flower.stalk
                        && dist_sq
                            < (stalk_radius
                                * Lerp::lerp_unclamped(1.0, 0.75, rpos.z / flower.stalk))
                            .powi(2)
                    {
                        return Some(Block::new(BlockKind::Wood, Rgb::new(0, 108, 0)));
                    }

                    let rpos = rpos - Vec3::unit_z() * flower.stalk;
                    let dist_sq = rpos.xy().magnitude_squared();
                    let petal_radius_sq = flower.petal_radius.powi(2);
                    if dist_sq < petal_radius_sq {
                        let petal_height_at = (dist_sq / petal_radius_sq) * flower.petal_height;
                        if rpos.z > petal_height_at - 1.0
                            && rpos.z <= petal_height_at + petal_thickness
                        {
                            let dist_ratio = dist_sq / petal_radius_sq;
                            let yellow = (60.0 * dist_ratio) as u8;
                            let near = rpos
                                .x
                                .atan2(rpos.y)
                                .rem_euclid(std::f32::consts::TAU / flower.petals as f32);
                            if dist_ratio < 0.175 {
                                let red = close(near, 0.0, 0.5, 1);
                                let purple = close(near, 0.0, 0.35, 1);
                                if dist_ratio > red || rpos.z < petal_height_at {
                                    return Some(Block::new(
                                        BlockKind::ArtLeaves,
                                        Rgb::new(240, 80 - yellow, 80 - yellow),
                                    ));
                                } else if dist_ratio > purple {
                                    return Some(Block::new(
                                        BlockKind::ArtLeaves,
                                        Rgb::new(200, 14, 132),
                                    ));
                                } else {
                                    return Some(Block::new(
                                        BlockKind::ArtLeaves,
                                        Rgb::new(249, 156, 218),
                                    ));
                                }
                            } else {
                                let inset = close(near, -1.0, 1.0, 2).max(close(near, 1.0, 1.0, 2));
                                if dist_ratio < inset {
                                    return Some(Block::new(
                                        BlockKind::ArtLeaves,
                                        Rgb::new(240, 80 - yellow, 80 - yellow),
                                    ));
                                }
                            }
                        }

                        // pollen
                        let pollen_height = 5.0;
                        if rpos.z > 0.0
                            && rpos.z < pollen_height
                            && dist_sq
                                < (stalk_radius
                                    * Lerp::lerp_unclamped(0.5, 1.25, rpos.z / pollen_height))
                                .powi(2)
                        {
                            return Some(Block::new(
                                BlockKind::GlowingWeakRock,
                                Rgb::new(239, 192, 0),
                            ));
                        }
                    }
                },
                CaveStructure::GiantRoot {
                    pos,
                    radius,
                    height,
                } => {
                    let wposf = wpos.map(|e| e as f64);
                    let warp_freq = 1.0 / 16.0;
                    let warp_amp = Vec3::new(8.0, 8.0, 8.0);
                    let warp_offset = warp(wposf, warp_freq, warp_amp, seed)?;
                    let wposf_warped = wposf.map(|e| e as f32) + warp_offset;
                    let rpos = wposf_warped - pos.map(|e| e as f32);
                    let dist_sq = rpos.xy().magnitude_squared();
                    if dist_sq < radius.powi(2) {
                        // Moss
                        if col.marble_mid
                            > (std::f32::consts::PI * rpos.z / *height)
                                .sin()
                                .powf(2.0)
                                .mul(0.25)
                                .add(col.marble_small)
                        {
                            return Some(Block::new(BlockKind::Wood, Rgb::new(48, 70, 25)));
                        }
                        return Some(Block::new(BlockKind::Wood, Rgb::new(66, 41, 26)));
                    }
                },
            }
        }
        None
    };

    for z in bedrock..z_range.end {
        let wpos = wpos2d.with_z(z);
        let mut try_spawn_entity = false;
        canvas.set(wpos, {
            if z < z_range.start - 4 && !void_below {
                Block::new(BlockKind::Lava, Rgb::new(255, 65, 0))
            } else if basalt > 0.0
                && z < bedrock / 6 * 6
                    + 2
                    + basalt as i32 / 4 * 4
                    + (RandomField::new(77)
                        .get_f32(((wpos2d + Vec2::new(wpos2d.y, -wpos2d.x) / 2) / 4).with_z(0))
                        * 6.0)
                        .floor() as i32
                && !void_below
            {
                Block::new(BlockKind::Rock, Rgb::new(50, 35, 75))
            } else if (z < base && !void_below)
                || ((z >= ceiling && !void_above)
                    && !(ceiling_cover > 0.0 && overlap.as_ref().map_or(false, |o| o.contains(&z))))
            {
                let stalactite: Rgb<i16> = Lerp::lerp_unclamped(
                    Lerp::lerp_unclamped(
                        Lerp::lerp_unclamped(
                            Lerp::lerp_unclamped(
                                Lerp::lerp_unclamped(
                                    Lerp::lerp_unclamped(
                                        Rgb::new(80, 100, 150),
                                        Rgb::new(23, 44, 88),
                                        biome.mushroom,
                                    ),
                                    Lerp::lerp_unclamped(
                                        Rgb::new(100, 40, 40),
                                        Rgb::new(100, 75, 100),
                                        col.marble_small,
                                    ),
                                    biome.fire,
                                ),
                                Lerp::lerp_unclamped(
                                    Rgb::new(238, 198, 139),
                                    Rgb::new(111, 99, 64),
                                    col.marble_mid,
                                ),
                                biome.sandy,
                            ),
                            Lerp::lerp_unclamped(
                                Rgb::new(0, 73, 12),
                                Rgb::new(49, 63, 12),
                                col.marble_small,
                            ),
                            biome.leafy,
                        ),
                        Lerp::lerp_unclamped(
                            Rgb::new(100, 150, 255),
                            Rgb::new(100, 120, 255),
                            col.marble,
                        ),
                        biome.icy,
                    ),
                    Lerp::lerp_unclamped(
                        Rgb::new(105, 25, 131),
                        Rgb::new(251, 238, 255),
                        col.marble_mid,
                    ),
                    biome.crystal,
                );
                Block::new(
                    if rand.chance(
                        wpos,
                        (biome.mushroom * 0.01)
                            .max(biome.icy * 0.1)
                            .max(biome.crystal * 0.005),
                    ) {
                        BlockKind::GlowingWeakRock
                    } else if rand.chance(wpos, biome.sandy) {
                        BlockKind::Sand
                    } else if rand.chance(wpos, biome.leafy) {
                        BlockKind::ArtLeaves
                    } else if ceiling_cover > 0.0 {
                        BlockKind::Rock
                    } else {
                        BlockKind::WeakRock
                    },
                    stalactite.map(|e| e as u8),
                )
            } else if z < ceiling && z >= ceiling_drip {
                if biome.mushroom > 0.9 {
                    let block = if rand.chance(wpos2d.with_z(89), 0.05) {
                        BlockKind::GlowingMushroom
                    } else {
                        BlockKind::GlowingWeakRock
                    };
                    Block::new(block, Rgb::new(10, 70, 148))
                } else if biome.icy > 0.9 {
                    Block::new(BlockKind::GlowingWeakRock, Rgb::new(120, 140, 255))
                } else {
                    Block::new(BlockKind::WeakRock, Rgb::new(80, 100, 150))
                }
            } else if z >= base && z < floor && !void_below && !sky_above {
                let (net_col, total) = [
                    (
                        Lerp::lerp_unclamped(
                            Rgb::new(68, 62, 58),
                            Rgb::new(97, 95, 85),
                            col.marble_small,
                        ),
                        0.05,
                    ),
                    (
                        Lerp::lerp_unclamped(
                            Rgb::new(66, 37, 30),
                            Rgb::new(88, 62, 45),
                            col.marble_mid,
                        ),
                        biome.dusty,
                    ),
                    (
                        Lerp::lerp_unclamped(
                            Rgb::new(20, 65, 175),
                            Rgb::new(20, 100, 80),
                            col.marble_mid,
                        ),
                        biome.mushroom,
                    ),
                    (
                        Lerp::lerp_unclamped(
                            Rgb::new(120, 50, 20),
                            Rgb::new(50, 5, 40),
                            col.marble_small,
                        ),
                        biome.fire,
                    ),
                    (
                        Lerp::lerp_unclamped(
                            Rgb::new(0, 100, 50),
                            Rgb::new(80, 100, 20),
                            col.marble_small,
                        ),
                        biome.leafy,
                    ),
                    (Rgb::new(170, 195, 255), biome.icy),
                    (
                        Lerp::lerp_unclamped(
                            Rgb::new(105, 25, 131),
                            Rgb::new(251, 238, 255),
                            col.marble_mid,
                        ),
                        biome.crystal,
                    ),
                    (
                        Lerp::lerp_unclamped(
                            Rgb::new(201, 174, 116),
                            Rgb::new(244, 239, 227),
                            col.marble_small,
                        ),
                        biome.sandy,
                    ),
                    (
                        // Same as barren
                        Lerp::lerp_unclamped(
                            Rgb::new(68, 62, 58),
                            Rgb::new(97, 95, 85),
                            col.marble_small,
                        ),
                        biome.snowy,
                    ),
                ]
                .into_iter()
                .fold((Rgb::<f32>::zero(), 0.0), |a, x| {
                    (a.0 + x.0.map(|e| e as f32) * x.1, a.1 + x.1)
                });
                let surf_color = net_col.map(|e| (e / total) as u8);

                if is_ice {
                    Block::new(BlockKind::Ice, Rgb::new(120, 160, 255))
                } else if is_snow {
                    Block::new(BlockKind::ArtSnow, Rgb::new(170, 195, 255))
                } else {
                    Block::new(
                        if biome.mushroom.max(biome.leafy) > 0.5 {
                            BlockKind::Grass
                        } else if biome.icy > 0.5 {
                            BlockKind::ArtSnow
                        } else if biome.fire.max(biome.snowy) > 0.5 {
                            BlockKind::Rock
                        } else if biome.crystal > 0.5 {
                            if rand.chance(wpos, biome.crystal * 0.02) {
                                BlockKind::GlowingRock
                            } else {
                                BlockKind::Rock
                            }
                        } else {
                            BlockKind::Sand
                        },
                        surf_color,
                    )
                }
            } else if let Some(sprite) = (z == floor && !void_below && !sky_above)
                .then(|| {
                    if col.marble_mid > 0.55
                        && biome.mushroom > 0.6
                        && rand.chance(
                            wpos2d.with_z(1),
                            biome.mushroom.powi(2) * 0.2 * col.marble_mid,
                        )
                    {
                        [
                            (SpriteKind::GlowMushroom, 0.5),
                            (SpriteKind::Mushroom, 0.25),
                            (SpriteKind::GrassBlue, 0.0),
                            (SpriteKind::GrassBlueMedium, 1.5),
                            (SpriteKind::GrassBlueLong, 2.0),
                            (SpriteKind::Moonbell, 0.01),
                            (SpriteKind::SporeReed, 2.5),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .map(|s| s.0)
                    } else if col.marble_mid > 0.6
                        && biome.leafy > 0.4
                        && rand.chance(
                            wpos2d.with_z(15),
                            biome.leafy.powi(2) * 0.25 * col.marble_mid,
                        )
                    {
                        let mixed = col.marble.add(col.marble_small.sub(0.5).mul(0.25));
                        if (0.25..0.45).contains(&mixed) || (0.55..0.75).contains(&mixed) {
                            return [
                                (SpriteKind::LongGrass, 1.0),
                                (SpriteKind::MediumGrass, 2.0),
                                (SpriteKind::ShortGrass, 0.0),
                                (SpriteKind::JungleFern, 0.5),
                                (SpriteKind::JungleRedGrass, 0.35),
                                (SpriteKind::Fern, 0.75),
                                (SpriteKind::LeafyPlant, 0.8),
                                (SpriteKind::JungleLeafyPlant, 0.5),
                                (SpriteKind::LanternPlant, 0.1),
                                (SpriteKind::LanternFlower, 0.1),
                                (SpriteKind::LushFlower, 0.2),
                            ]
                            .choose_weighted(rng, |(_, w)| *w)
                            .ok()
                            .map(|s| s.0);
                        } else if (0.0..0.25).contains(&mixed) {
                            return [(Some(SpriteKind::LanternPlant), 0.5), (None, 0.5)]
                                .choose_weighted(rng, |(_, w)| *w)
                                .ok()
                                .and_then(|s| s.0);
                        } else if (0.75..1.0).contains(&mixed) {
                            return [(Some(SpriteKind::LushFlower), 0.6), (None, 0.4)]
                                .choose_weighted(rng, |(_, w)| *w)
                                .ok()
                                .and_then(|s| s.0);
                        } else {
                            return [
                                (SpriteKind::LongGrass, 1.0),
                                (SpriteKind::MediumGrass, 2.0),
                                (SpriteKind::ShortGrass, 0.0),
                                (SpriteKind::JungleFern, 0.5),
                                (SpriteKind::JungleLeafyPlant, 0.5),
                                (SpriteKind::JungleRedGrass, 0.35),
                                (SpriteKind::Mushroom, 0.15),
                                (SpriteKind::EnsnaringVines, 0.2),
                                (SpriteKind::Fern, 0.75),
                                (SpriteKind::LeafyPlant, 0.8),
                                (SpriteKind::Twigs, 0.07),
                                (SpriteKind::Wood, 0.03),
                                (SpriteKind::LanternPlant, 0.3),
                                (SpriteKind::LanternFlower, 0.3),
                                (SpriteKind::LushFlower, 0.5),
                                (SpriteKind::LushMushroom, 1.0),
                            ]
                            .choose_weighted(rng, |(_, w)| *w)
                            .ok()
                            .map(|s| s.0);
                        }
                    } else if rand.chance(wpos2d.with_z(2), biome.dusty.max(biome.sandy) * 0.01) {
                        [
                            (SpriteKind::Bones, 0.5),
                            (SpriteKind::Stones, 1.5),
                            (SpriteKind::DeadBush, 1.0),
                            (SpriteKind::DeadPlant, 1.5),
                            (SpriteKind::EnsnaringWeb, 0.5),
                            (SpriteKind::Mud, 0.025),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .map(|s| s.0)
                    } else if rand.chance(wpos2d.with_z(14), biome.barren * 0.003) {
                        [
                            (SpriteKind::Bones, 0.5),
                            (SpriteKind::Welwitch, 0.5),
                            (SpriteKind::DeadBush, 1.5),
                            (SpriteKind::DeadPlant, 1.5),
                            (SpriteKind::RockyMushroom, 1.5),
                            (SpriteKind::Crate, 0.005),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .map(|s| s.0)
                    } else if rand.chance(wpos2d.with_z(3), biome.crystal * 0.005) {
                        Some(SpriteKind::CrystalLow)
                    } else if rand.chance(wpos2d.with_z(13), biome.fire * 0.001) {
                        [
                            (SpriteKind::Pyrebloom, 0.3),
                            (SpriteKind::Bloodstone, 0.3),
                            (SpriteKind::Gold, 0.15),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .map(|s| s.0)
                    } else if biome.icy > 0.5 && rand.chance(wpos2d.with_z(23), biome.icy * 0.005) {
                        Some(SpriteKind::IceCrystal)
                    } else if biome.icy > 0.5
                        && rand.chance(wpos2d.with_z(31), biome.icy * biome.mineral * 0.005)
                    {
                        Some(SpriteKind::GlowIceCrystal)
                    } else if rand.chance(wpos2d.with_z(5), 0.0025) {
                        [
                            (Some(SpriteKind::VeloriteFrag), 0.3),
                            (Some(SpriteKind::AmethystSmall), 0.3),
                            (Some(SpriteKind::TopazSmall), 0.3),
                            (Some(SpriteKind::DiamondSmall), 0.04),
                            (Some(SpriteKind::RubySmall), 0.1),
                            (Some(SpriteKind::EmeraldSmall), 0.08),
                            (Some(SpriteKind::SapphireSmall), 0.08),
                            (Some(SpriteKind::Velorite), 0.15),
                            (Some(SpriteKind::Amethyst), 0.15),
                            (Some(SpriteKind::Topaz), 0.15),
                            (Some(SpriteKind::Diamond), 0.02),
                            (Some(SpriteKind::Ruby), 0.05),
                            (Some(SpriteKind::Emerald), 0.04),
                            (Some(SpriteKind::Sapphire), 0.04),
                            (None, 10.0),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .and_then(|s| s.0)
                    } else if rand.chance(wpos2d.with_z(6), 0.0002) {
                        [
                            (Some(SpriteKind::DungeonChest0), 1.0),
                            (Some(SpriteKind::DungeonChest1), 0.3),
                            (Some(SpriteKind::DungeonChest2), 0.1),
                            (Some(SpriteKind::DungeonChest3), 0.03),
                            (Some(SpriteKind::DungeonChest4), 0.01),
                            (Some(SpriteKind::DungeonChest5), 0.003),
                            (None, 1.0),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .and_then(|s| s.0)
                    } else if rand.chance(wpos2d.with_z(7), 0.01) {
                        let shallow = close(biome.depth, 0.0, 0.4, 3);
                        let middle = close(biome.depth, 0.5, 0.4, 3);
                        //let deep = close(biome.depth, 1.0, 0.4); // TODO: Use this for deep only
                        // things
                        [
                            (Some(SpriteKind::Stones), 1.5),
                            (Some(SpriteKind::Copper), shallow),
                            (Some(SpriteKind::Tin), shallow),
                            (Some(SpriteKind::Iron), shallow * 0.5),
                            (Some(SpriteKind::Coal), middle * 0.25),
                            (Some(SpriteKind::Cobalt), middle * 0.1),
                            (Some(SpriteKind::Silver), middle * 0.05),
                            (None, 10.0),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .and_then(|s| s.0)
                    } else {
                        try_spawn_entity = true;
                        None
                    }
                })
                .flatten()
            {
                Block::air(sprite)
            } else if let Some(sprite) = (z == ceiling - 1 && !void_above)
                .then(|| {
                    if biome.mushroom > 0.5 && rand.chance(wpos2d.with_z(3), biome.mushroom * 0.01)
                    {
                        [(SpriteKind::MycelBlue, 0.75), (SpriteKind::Mold, 1.0)]
                            .choose_weighted(rng, |(_, w)| *w)
                            .ok()
                            .map(|s| s.0)
                    } else if biome.leafy > 0.4
                        && rand.chance(wpos2d.with_z(4), biome.leafy * 0.015)
                    {
                        [
                            (SpriteKind::Liana, 1.5),
                            (SpriteKind::CeilingLanternPlant, 1.25),
                            (SpriteKind::CeilingLanternFlower, 1.0),
                            (SpriteKind::CeilingJungleLeafyPlant, 1.5),
                        ]
                        .choose_weighted(rng, |(_, w)| *w)
                        .ok()
                        .map(|s| s.0)
                    } else if rand.chance(wpos2d.with_z(5), biome.barren * 0.015) {
                        Some(SpriteKind::Root)
                    } else if rand.chance(wpos2d.with_z(5), biome.crystal * 0.005) {
                        Some(SpriteKind::CrystalHigh)
                    } else {
                        None
                    }
                })
                .flatten()
            {
                Block::air(sprite)
            } else if let Some(structure_block) = get_structure(wpos, rng) {
                structure_block
            } else {
                Block::empty()
            }
        });

        if try_spawn_entity {
            apply_entity_spawns(canvas, wpos, &biome, rng);
        }
    }
}

fn apply_entity_spawns<R: Rng>(canvas: &mut Canvas, wpos: Vec3<i32>, biome: &Biome, rng: &mut R) {
    if RandomField::new(canvas.info().index().seed).chance(wpos, 0.035) {
        if let Some(entity_asset) = [
            // Mushroom biome
            (
                Some("common.entity.wild.peaceful.truffler"),
                biome.mushroom + 0.02,
                0.35,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.fungome"),
                biome.mushroom + 0.02,
                0.5,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.bat"),
                biome.mushroom + 0.1,
                0.25,
                0.5,
            ),
            // Leafy biome
            (
                Some("common.entity.wild.peaceful.holladon"),
                biome.leafy.max(biome.dusty) + 0.05,
                0.25,
                0.5,
            ),
            (
                Some("common.entity.dungeon.gnarling.mandragora"),
                biome.leafy + 0.05,
                0.2,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.rootsnapper"),
                biome.leafy + 0.05,
                0.075,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.maneater"),
                biome.leafy + 0.05,
                0.075,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.batfox"),
                biome
                    .leafy
                    .max(biome.barren)
                    .max(biome.sandy)
                    .max(biome.snowy)
                    + 0.3,
                0.25,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.crawler_moss"),
                biome.leafy + 0.05,
                0.25,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.asp"),
                biome.leafy.max(biome.sandy) + 0.1,
                0.2,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.swamp_troll"),
                biome.leafy + 0.0,
                0.05,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.bat"),
                biome.leafy + 0.1,
                0.25,
                0.5,
            ),
            // Dusty biome
            (
                Some("common.entity.wild.aggressive.dodarock"),
                biome
                    .dusty
                    .max(biome.barren)
                    .max(biome.crystal)
                    .max(biome.snowy)
                    + 0.05,
                0.05,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.cave_spider"),
                biome.dusty + 0.0,
                0.05,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.cave_troll"),
                biome.dusty + 0.1,
                0.05,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.rat"),
                biome.dusty + 0.1,
                0.3,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.bat"),
                biome.dusty.max(biome.sandy).max(biome.snowy) + 0.1,
                0.25,
                0.5,
            ),
            // Icy biome
            (
                Some("common.entity.wild.aggressive.icedrake"),
                biome.icy + 0.0,
                0.1,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.wendigo"),
                biome.icy.min(biome.depth) + 0.0,
                0.02,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.frostfang"),
                biome.icy + 0.0,
                0.25,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.tursus"),
                biome.icy + 0.0,
                0.03,
                0.5,
            ),
            // Lava biome
            (
                Some("common.entity.wild.aggressive.lavadrake"),
                biome.fire + 0.0,
                0.15,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.crawler_molten"),
                biome.fire + 0.0,
                0.2,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.cave_salamander"),
                biome.fire + 0.0,
                0.4,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.red_oni"),
                biome.fire + 0.0,
                0.03,
                0.5,
            ),
            // Crystal biome
            (
                Some("common.entity.wild.aggressive.basilisk"),
                biome.crystal + 0.1,
                0.1,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.blue_oni"),
                biome.crystal + 0.0,
                0.03,
                0.5,
            ),
            // Sandy biome
            (
                Some("common.entity.wild.aggressive.antlion"),
                biome.sandy.max(biome.dusty) + 0.1,
                0.025,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.sandshark"),
                biome.sandy + 0.1,
                0.025,
                0.5,
            ),
            (
                Some("common.entity.wild.peaceful.crawler_sand"),
                biome.sandy + 0.1,
                0.25,
                0.5,
            ),
            // Snowy biome
            (
                Some("common.entity.wild.aggressive.akhlut"),
                (biome.snowy.max(biome.icy) + 0.1),
                0.01,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.rocksnapper"),
                biome.barren.max(biome.crystal).max(biome.snowy) + 0.1,
                0.1,
                0.5,
            ),
            // With depth
            (
                Some("common.entity.wild.aggressive.black_widow"),
                biome.depth + 0.0,
                0.02,
                0.5,
            ),
            (
                Some("common.entity.wild.aggressive.ogre"),
                biome.depth + 0.0,
                0.02,
                0.5,
            ),
            (None, 100.0, 0.0, 0.0),
        ]
        .iter()
        .filter_map(|(entity, biome_modifier, chance, cutoff)| {
            if let Some(entity) = entity {
                if *biome_modifier > *cutoff {
                    let close = close(1.0, *biome_modifier, *cutoff, 2);
                    (close > 0.0).then(|| (Some(entity), close * chance))
                } else {
                    None
                }
            } else {
                Some((None, 100.0))
            }
        })
        .collect_vec()
        .choose_weighted(rng, |(_, w)| *w)
        .ok()
        .and_then(|s| s.0)
        {
            canvas.spawn(EntityInfo::at(wpos.map(|e| e as f32)).with_asset_expect(
                entity_asset,
                rng,
                None,
            ));
        }
    }

    // FIXME: Add back waypoints once caves are not impossible to escape.
    /* // Occasionally place down a waypoint
    if RandomField::new(canvas.info().index().seed).chance(wpos, 0.000005) {
        canvas.spawn(EntityInfo::at(wpos.map(|e| e as f32)).into_waypoint());
    } */
}
