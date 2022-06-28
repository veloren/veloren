use super::scatter::close;
use crate::{
    util::{sampler::Sampler, FastNoise, RandomField, RandomPerm, StructureGen2d, LOCALITY},
    Canvas, CanvasInfo, ColumnSample, Land,
};
use common::{
    terrain::{
        quadratic_nearest_point, river_spline_coeffs, Block, BlockKind, SpriteKind,
        TerrainChunkSize,
    },
    vol::RectVolSize,
};
use noise::{Fbm, NoiseFn};
use rand::prelude::*;
use std::{
    cmp::Ordering,
    collections::HashMap,
    f64::consts::PI,
    ops::{Add, Mul, Range, Sub},
};
use vek::*;

const CELL_SIZE: i32 = 1024;

#[derive(Copy, Clone)]
pub struct Node {
    pub wpos: Vec3<i32>,
}

fn to_cell(wpos: Vec2<i32>, level: u32) -> Vec2<i32> {
    (wpos + (level & 1) as i32 * CELL_SIZE / 2).map(|e| e.div_euclid(CELL_SIZE))
}
fn to_wpos(cell: Vec2<i32>, level: u32) -> Vec2<i32> {
    (cell * CELL_SIZE) - (level & 1) as i32 * CELL_SIZE / 2
}

const AVG_LEVEL_DEPTH: i32 = 120;
const LAYERS: u32 = 4;

fn node_at(cell: Vec2<i32>, level: u32, land: &Land) -> Option<Node> {
    let rand = RandomField::new(37 + level);

    if rand.chance(cell.with_z(0), 0.85) || level == 0 {
        let dx = RandomField::new(38 + level);
        let dy = RandomField::new(39 + level);
        let wpos = to_wpos(cell, level)
            + CELL_SIZE as i32 / 4
            + (Vec2::new(dx.get(cell.with_z(0)), dy.get(cell.with_z(0))) % CELL_SIZE as u32 / 2)
                .map(|e| e as i32);
        land.get_chunk_wpos(wpos).and_then(|chunk| {
            let alt = chunk.alt as i32 + 8 - AVG_LEVEL_DEPTH * level as i32;

            if level > 0
                || (!chunk.near_cliffs()
                    && !chunk.river.near_water()
                    && chunk.sites.is_empty()
                    && land.get_gradient_approx(wpos) < 0.75)
            {
                Some(Node {
                    wpos: wpos.with_z(alt),
                })
            } else {
                None
            }
        })
    } else {
        None
    }
}

pub fn surface_entrances<'a>(land: &'a Land) -> impl Iterator<Item = Vec2<i32>> + 'a {
    let sz_cells = to_cell(
        land.size()
            .map2(TerrainChunkSize::RECT_SIZE, |e, sz| (e * sz) as i32),
        0,
    );
    (0..sz_cells.x + 1)
        .flat_map(move |x| (0..sz_cells.y + 1).map(move |y| Vec2::new(x, y)))
        .filter_map(|cell| {
            let tunnel = tunnel_below_from_cell(cell, 0, land)?;
            // Hacky, moves the entrance position closer to the actual entrance
            Some(Lerp::lerp(tunnel.a.wpos.xy(), tunnel.b.wpos.xy(), 0.125))
        })
}

struct Tunnel {
    a: Node,
    b: Node,
    curve: f32,
}

impl Tunnel {
    fn z_range_at(&self, wposf: Vec2<f64>, nz: &Fbm) -> Option<Range<i32>> {
        let start = self.a.wpos.xy().map(|e| e as f64 + 0.5);
        let end = self.b.wpos.xy().map(|e| e as f64 + 0.5);

        if let Some((t, closest, _)) = quadratic_nearest_point(
            &river_spline_coeffs(
                start,
                ((end - start) * 0.5
                    + ((end - start) * 0.5).rotated_z(PI / 2.0) * 6.0 * self.curve as f64)
                    .map(|e| e as f32),
                end,
            ),
            wposf,
            Vec2::new(start, end),
        ) {
            let dist = closest.distance(wposf);
            let radius = 8.0..64.0;
            if dist < radius.end + 1.0 {
                let tunnel_len = self
                    .a
                    .wpos
                    .map(|e| e as f64)
                    .distance(self.b.wpos.map(|e| e as f64));
                let radius = Lerp::lerp(
                    radius.start,
                    radius.end,
                    (nz.get((wposf / 200.0).into_array()) * 2.0 * 0.5 + 0.5)
                        .clamped(0.0, 1.0)
                        .powf(3.0),
                ); // Lerp::lerp(8.0, 24.0, (t * 0.075 * tunnel_len).sin() * 0.5 + 0.5);
                let height_here = (1.0 - dist / radius).max(0.0).powf(0.3) * radius;
                if height_here > 0.0 {
                    let z_offs = nz.get((wposf / 512.0).into_array())
                        * 48.0
                        * ((1.0 - (t - 0.5).abs() * 2.0) * 8.0).min(1.0);
                    let depth = Lerp::lerp(self.a.wpos.z as f64, self.b.wpos.z as f64, t) + z_offs;
                    Some((depth - height_here * 0.3) as i32..(depth + height_here * 1.35) as i32)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn biome_at(&self, wpos: Vec3<i32>, info: &CanvasInfo) -> Biome {
        let Some(col) = info.col_or_gen(wpos.xy()) else { return Biome::default() };

        // Below the ground
        let below = ((col.alt - wpos.z as f32) / 50.0).clamped(0.0, 1.0);

        let humidity = Lerp::lerp(
            col.humidity,
            info.index()
                .noise
                .cave_nz
                .get(wpos.xy().map(|e| e as f64 / 1024.0).into_array()) as f32,
            below,
        );
        let temp = Lerp::lerp(
            col.temp,
            info.index()
                .noise
                .cave_nz
                .get(wpos.xy().map(|e| e as f64 / 2048.0).into_array())
                .add(
                    ((col.alt as f64 - wpos.z as f64)
                        / (AVG_LEVEL_DEPTH as f64 * LAYERS as f64 * 0.8))
                        .clamped(0.0, 2.0),
                ) as f32,
            below,
        );
        let mineral = info
            .index()
            .noise
            .cave_nz
            .get(wpos.xy().map(|e| e as f64 / 256.0).into_array())
            .mul(0.5)
            .add(0.5) as f32;

        let underground = ((col.alt as f32 - wpos.z as f32) / 80.0 - 1.0).clamped(0.0, 1.0);

        let [_, mushroom, fire, leafy, dusty] = {
            let barren = 0.01;
            let mushroom = underground * close(humidity, 1.0, 0.75) * close(temp, 0.25, 1.2);
            let fire = underground * close(humidity, 0.0, 0.75) * close(temp, 2.0, 0.65);
            let leafy = underground * close(humidity, 1.0, 0.75) * close(temp, -0.1, 0.75);
            let dusty = underground * close(humidity, 0.0, 0.5) * close(temp, -0.3, 0.65);

            let biomes = [barren, mushroom, fire, leafy, dusty];
            let max = biomes
                .into_iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal))
                .unwrap();
            biomes.map(|e| (e / max).powf(4.0))
        };

        Biome {
            humidity,
            temp,
            mineral,
            mushroom,
            fire,
            leafy,
            dusty,
        }
    }
}

fn tunnels_at<'a>(
    wpos: Vec2<i32>,
    level: u32,
    land: &'a Land,
) -> impl Iterator<Item = Tunnel> + 'a {
    let rand = RandomField::new(37 + level);
    let col_cell = to_cell(wpos, level);
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
                .map(move |(other_cell_pos, other_cell)| Tunnel {
                    a: current_cell,
                    b: other_cell,
                    curve: RandomField::new(13)
                        .get_f32(current_cell.wpos.xy().with_z(0))
                        .powf(0.25)
                        .mul(
                            if RandomField::new(14).chance(current_cell.wpos.xy().with_z(0), 0.5) {
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
        b: node_at(
            to_cell(wpos + CELL_SIZE as i32 / 2, level + 1),
            level + 1,
            land,
        )?,
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

pub fn apply_caves_to(canvas: &mut Canvas, rng: &mut impl Rng) {
    let nz = Fbm::new();
    let info = canvas.info();
    let mut mushroom_cache = HashMap::new();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let wposf = wpos2d.map(|e| e as f64 + 0.5);
        let land = info.land();

        for level in 1..LAYERS + 1 {
            let rand = RandomField::new(37 + level);
            let tunnel_bounds = tunnels_at(wpos2d, level, &land)
                .chain(tunnels_down_from(wpos2d, level - 1, &land))
                .filter_map(|tunnel| Some((tunnel.z_range_at(wposf, &nz)?, tunnel)));

            for (z_range, tunnel) in tunnel_bounds {
                // Avoid cave entrances intersecting water
                let z_range = Lerp::lerp(
                    z_range.end,
                    z_range.start,
                    1.0 - (1.0 - ((col.alt - col.water_level) / 4.0).clamped(0.0, 1.0))
                        * (1.0 - ((col.alt - z_range.end as f32) / 8.0).clamped(0.0, 1.0)),
                )..z_range.end;
                write_column(
                    canvas,
                    col,
                    level,
                    wpos2d,
                    z_range,
                    tunnel,
                    &nz,
                    &mut mushroom_cache,
                    rng,
                );
            }
        }
    });
}

#[derive(Default)]
struct Biome {
    humidity: f32,
    temp: f32,
    mineral: f32,
    mushroom: f32,
    fire: f32,
    leafy: f32,
    dusty: f32,
}

struct Mushroom {
    pos: Vec3<i32>,
    stalk: f32,
    head_color: Rgb<u8>,
}

fn write_column<R: Rng>(
    canvas: &mut Canvas,
    col: &ColumnSample,
    level: u32,
    wpos2d: Vec2<i32>,
    z_range: Range<i32>,
    tunnel: Tunnel,
    nz: &Fbm,
    mushroom_cache: &mut HashMap<(Vec3<i32>, Vec2<i32>), Option<Mushroom>>,
    rng: &mut R,
) {
    mushroom_cache.clear();
    let info = canvas.info();

    // Exposed to the sky
    let exposed = z_range.end as f32 > col.alt;

    let biome = tunnel.biome_at(wpos2d.with_z(z_range.start), &info);

    let stalactite = {
        let cavern_height = (z_range.end - z_range.start) as f64;
        info
            .index()
            .noise
            .cave_nz
            .get(wpos2d.map(|e| e as f64 / 16.0).into_array())
            .sub(0.5)
            .max(0.0)
            .mul(2.0)
            // No stalactites near entrances
            .mul(((col.alt as f64 - z_range.end as f64) / 8.0).clamped(0.0, 1.0))
            .mul(8.0 + cavern_height * 0.4)
    };

    let lava = {
        info.index()
            .noise
            .cave_nz
            .get(wpos2d.map(|e| e as f64 / 64.0).into_array())
            .sub(0.5)
            .abs()
            .sub(0.2)
            .min(0.0)
            // .mul((biome.temp as f64 - 1.5).mul(30.0).clamped(0.0, 1.0))
            .mul((biome.fire as f64 - 0.5).mul(30.0).clamped(0.0, 1.0))
            .mul(64.0)
            .max(-32.0)
    };

    let rand = RandomField::new(37 + level);

    let dirt = if exposed { 0 } else { 1 };
    let bedrock = z_range.start + lava as i32;
    let base = bedrock + (stalactite * 0.4) as i32;
    let floor = base + dirt;
    let ceiling = z_range.end - stalactite as i32;

    // Get mushroom block, if any, at a position
    let mut get_mushroom = |wpos: Vec3<i32>, dynamic_rng: &mut R| {
        for (wpos2d, seed) in StructureGen2d::new(34537, 24, 8).get(wpos.xy()) {
            let mushroom = if let Some(mushroom) = mushroom_cache
                .entry((tunnel.a.wpos, wpos2d))
                .or_insert_with(|| {
                    let mut rng = RandomPerm::new(seed);
                    let z_range = tunnel.z_range_at(wpos2d.map(|e| e as f64 + 0.5), nz)?;
                    let (cavern_bottom, cavern_top, floor, water_level) = (
                        z_range.start,
                        z_range.end,
                        0, //(stalactite * 0.4) as i32,
                        0,
                    );
                    let pos = wpos2d.with_z(cavern_bottom + floor);
                    if rng.gen_bool(0.75)
                        && cavern_top - cavern_bottom > 15
                        && tunnel.biome_at(pos, &info).mushroom > 0.5
                    // && pos.z as i32 > water_level - 2
                    {
                        Some(Mushroom {
                            pos,
                            stalk: 8.0
                                + rng.gen::<f32>().powf(2.0)
                                    * (cavern_top - cavern_bottom - 8) as f32
                                    * 0.85,
                            head_color: Rgb::new(
                                50,
                                rng.gen_range(70..110),
                                rng.gen_range(100..200),
                            ),
                        })
                    } else {
                        None
                    }
                }) {
                mushroom
            } else {
                continue;
            };

            let wposf = wpos.map(|e| e as f64);
            let warp_freq = 1.0 / 32.0;
            let warp_amp = Vec3::new(12.0, 12.0, 12.0);
            let wposf_warped = wposf.map(|e| e as f32)
                + Vec3::new(
                    FastNoise::new(seed).get(wposf * warp_freq) as f32,
                    FastNoise::new(seed + 1).get(wposf * warp_freq) as f32,
                    FastNoise::new(seed + 2).get(wposf * warp_freq) as f32,
                ) * warp_amp
                    * (wposf.z as f32 - mushroom.pos.z as f32)
                        .mul(0.1)
                        .clamped(0.0, 1.0);

            let rpos = wposf_warped - mushroom.pos.map(|e| e as f32).map(|e| e as f32);

            let stalk_radius = 2.5f32;
            let head_radius = 12.0f32;
            let head_height = 14.0;

            let dist_sq = rpos.xy().magnitude_squared();
            if dist_sq < head_radius.powi(2) {
                let dist = dist_sq.sqrt();
                let head_dist = ((rpos - Vec3::unit_z() * mushroom.stalk)
                    / Vec2::broadcast(head_radius).with_z(head_height))
                .magnitude();

                let stalk = mushroom.stalk + Lerp::lerp(head_height * 0.5, 0.0, dist / head_radius);

                // Head
                if rpos.z > stalk
                    && rpos.z <= mushroom.stalk + head_height
                    && dist
                        < head_radius * (1.0 - (rpos.z - mushroom.stalk) / head_height).powf(0.125)
                {
                    if head_dist < 0.85 {
                        let radial = (rpos.x.atan2(rpos.y) * 10.0).sin() * 0.5 + 0.5;
                        return Some(Block::new(
                            BlockKind::GlowingMushroom,
                            Rgb::new(30, 50 + (radial * 100.0) as u8, 100 - (radial * 50.0) as u8),
                        ));
                    } else if head_dist < 1.0 {
                        return Some(Block::new(BlockKind::Wood, mushroom.head_color));
                    }
                }

                if rpos.z <= mushroom.stalk + head_height - 1.0
                    && dist_sq
                        < (stalk_radius * Lerp::lerp(1.5, 0.75, rpos.z / mushroom.stalk)).powi(2)
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
                        &[Orb, CavernMycelBlue, CavernMycelBlue] as &[_]
                    };
                    return Some(Block::air(*sprites.choose(dynamic_rng).unwrap()));
                }
            }
        }

        None
    };

    for z in bedrock..z_range.end {
        let wpos = wpos2d.with_z(z);
        canvas.map(wpos, |block| {
            if !block.is_filled() {
                block.into_vacant()
            } else if z < z_range.start - 4 {
                Block::new(BlockKind::Lava, Rgb::new(255, 65, 0))
            } else if z < base || z >= ceiling {
                let stalactite: Rgb<i16> =
                    Lerp::lerp(Rgb::new(80, 100, 150), Rgb::new(0, 75, 200), biome.mushroom);
                Block::new(
                    if rand.chance(wpos, biome.mushroom * biome.mineral) {
                        BlockKind::GlowingWeakRock
                    } else {
                        BlockKind::WeakRock
                    },
                    stalactite.map(|e| e as u8),
                )
            } else if z >= base && z < floor {
                let dry_mud =
                    Lerp::lerp(Rgb::new(40, 20, 0), Rgb::new(80, 80, 30), col.marble_small);
                let mycelium =
                    Lerp::lerp(Rgb::new(20, 65, 175), Rgb::new(20, 100, 80), col.marble_mid);
                let fire_rock =
                    Lerp::lerp(Rgb::new(120, 50, 20), Rgb::new(50, 5, 40), col.marble_small);
                let grassy = Lerp::lerp(
                    Rgb::new(0, 100, 50),
                    Rgb::new(80, 100, 20),
                    col.marble_small,
                );
                let dusty = Lerp::lerp(Rgb::new(50, 50, 75), Rgb::new(75, 75, 50), col.marble_mid);
                let surf_color: Rgb<i16> = Lerp::lerp(
                    Lerp::lerp(
                        Lerp::lerp(
                            Lerp::lerp(dry_mud, dusty, biome.dusty),
                            mycelium,
                            biome.mushroom,
                        ),
                        grassy,
                        biome.leafy,
                    ),
                    fire_rock,
                    biome.fire,
                );

                Block::new(BlockKind::Sand, surf_color.map(|e| e as u8))
            } else if let Some(sprite) = (z == floor && !exposed)
                .then(|| {
                    if rand.chance(wpos2d.with_z(1), biome.mushroom * 0.1) {
                        Some(
                            [
                                (SpriteKind::CaveMushroom, 0.3),
                                (SpriteKind::Mushroom, 0.3),
                                (SpriteKind::GrassBlue, 1.0),
                                (SpriteKind::CavernGrassBlueShort, 1.0),
                                (SpriteKind::CavernGrassBlueMedium, 1.0),
                                (SpriteKind::CavernGrassBlueLong, 1.0),
                            ]
                            .choose_weighted(rng, |(_, w)| *w)
                            .unwrap()
                            .0,
                        )
                    } else if rand.chance(wpos2d.with_z(1), biome.leafy * 0.25) {
                        Some(
                            [
                                (SpriteKind::LongGrass, 1.0),
                                (SpriteKind::MediumGrass, 2.0),
                                (SpriteKind::ShortGrass, 2.0),
                                (SpriteKind::JungleFern, 0.5),
                                (SpriteKind::JungleLeafyPlant, 0.5),
                                (SpriteKind::JungleRedGrass, 0.35),
                                (SpriteKind::Mushroom, 0.3),
                                (SpriteKind::EnsnaringVines, 0.2),
                                (SpriteKind::Fern, 0.75),
                                (SpriteKind::LeafyPlant, 0.8),
                            ]
                            .choose_weighted(rng, |(_, w)| *w)
                            .unwrap()
                            .0,
                        )
                    } else if rand.chance(wpos2d.with_z(2), biome.dusty * 0.01) {
                        Some(
                            [
                                (SpriteKind::Bones, 0.5),
                                (SpriteKind::Stones, 1.5),
                                (SpriteKind::DeadBush, 1.0),
                                (SpriteKind::EnsnaringWeb, 0.5),
                                (SpriteKind::Mud, 0.025),
                            ]
                            .choose_weighted(rng, |(_, w)| *w)
                            .unwrap()
                            .0,
                        )
                    } else if rand.chance(
                        wpos2d.with_z(3),
                        close(biome.humidity, 0.0, 0.5) * biome.mineral * 0.005,
                    ) {
                        Some(SpriteKind::CrystalLow)
                    } else if rand.chance(wpos2d.with_z(4), biome.fire * 0.0003) {
                        Some(SpriteKind::Pyrebloom)
                    } else if rand.chance(wpos2d.with_z(5), close(biome.mineral, 1.0, 0.5) * 0.001)
                    {
                        Some(
                            *[
                                SpriteKind::Velorite,
                                SpriteKind::VeloriteFrag,
                                SpriteKind::AmethystSmall,
                                SpriteKind::TopazSmall,
                                SpriteKind::DiamondSmall,
                                SpriteKind::RubySmall,
                                SpriteKind::EmeraldSmall,
                                SpriteKind::SapphireSmall,
                            ]
                            .choose(rng)
                            .unwrap(),
                        )
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
                        .unwrap()
                        .0
                    } else {
                        None
                    }
                })
                .flatten()
            {
                Block::air(sprite)
            } else if let Some(sprite) = (z == ceiling - 1)
                .then(|| {
                    if rand.chance(wpos2d.with_z(3), biome.mushroom * 0.02) {
                        Some(
                            *[
                                SpriteKind::CavernMycelBlue,
                                SpriteKind::CeilingMushroom,
                                SpriteKind::Orb,
                            ]
                            .choose(rng)
                            .unwrap(),
                        )
                    } else if rand.chance(wpos2d.with_z(4), 0.0075) {
                        Some(
                            *[SpriteKind::CrystalHigh, SpriteKind::Liana]
                                .choose(rng)
                                .unwrap(),
                        )
                    } else {
                        None
                    }
                })
                .flatten()
            {
                Block::air(sprite)
            } else {
                get_mushroom(wpos, rng).unwrap_or(Block::air(SpriteKind::Empty))
            }
        });
    }
}
