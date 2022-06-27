use super::scatter::close;
use crate::{
    util::{sampler::Sampler, RandomField, LOCALITY},
    Canvas, ColumnSample, Land,
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

fn node_at(cell: Vec2<i32>, level: u32, land: &Land) -> Option<Node> {
    let rand = RandomField::new(37 + level);

    if rand.chance(cell.with_z(0), 0.5) || level == 0 {
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
            let tunnel = tunnels_below_from_cell(cell, 0, land)?;
            // Hacky, moves the entrance position closer to the actual entrance
            Some(Lerp::lerp(tunnel.a.wpos.xy(), tunnel.b.wpos.xy(), 0.25))
        })
}

struct Tunnel {
    a: Node,
    b: Node,
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
                .map(move |(other_cell_pos, other_cell)| Tunnel {
                    a: current_cell,
                    b: other_cell,
                })
        })
}

fn tunnels_below_from_cell(cell: Vec2<i32>, level: u32, land: &Land) -> Option<Tunnel> {
    let wpos = to_wpos(cell, level);
    Some(Tunnel {
        a: node_at(to_cell(wpos, level), level, land)?,
        b: node_at(
            to_cell(wpos + CELL_SIZE as i32 / 2, level + 1),
            level + 1,
            land,
        )?,
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
        .filter_map(move |rpos| tunnels_below_from_cell(col_cell + rpos, level, land))
}

pub fn apply_caves_to(canvas: &mut Canvas, rng: &mut impl Rng) {
    let nz = Fbm::new();
    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let wposf = wpos2d.map(|e| e as f64 + 0.5);
        let land = info.land();

        for level in 1..4 {
            let rand = RandomField::new(37 + level);
            let tunnel_bounds = tunnels_at(wpos2d, level, &land)
                .chain(tunnels_down_from(wpos2d, level, &land))
                .filter_map(|tunnel| {
                    let start = tunnel.a.wpos.xy().map(|e| e as f64 + 0.5);
                    let end = tunnel.b.wpos.xy().map(|e| e as f64 + 0.5);
                    let dist = LineSegment2 { start, end }
                        .distance_to_point(wpos2d.map(|e| e as f64 + 0.5));

                    let curve = (
                        RandomField::new(13)
                            .get_f32(tunnel.a.wpos.xy().with_z(0))
                            .powf(0.25) as f64,
                        (RandomField::new(14).get_f32(tunnel.a.wpos.xy().with_z(0)) as f64 - 0.5)
                            .signum(),
                    );

                    if let Some((t, closest, _)) = quadratic_nearest_point(
                        &river_spline_coeffs(
                            start,
                            ((end - start) * 0.5
                                + ((end - start) * 0.5).rotated_z(PI / 2.0)
                                    * 4.0
                                    * curve.0
                                    * curve.1)
                                .map(|e| e as f32),
                            end,
                        ),
                        wposf,
                        Vec2::new(start, end),
                    ) {
                        let dist = closest.distance(wposf);
                        if dist < 64.0 {
                            let tunnel_len = tunnel
                                .a
                                .wpos
                                .map(|e| e as f64)
                                .distance(tunnel.b.wpos.map(|e| e as f64));
                            let radius = Lerp::lerp(
                                6.0,
                                32.0,
                                (nz.get((wposf / 200.0).into_array()) * 2.0 * 0.5 + 0.5)
                                    .clamped(0.0, 1.0),
                            ); // Lerp::lerp(8.0, 24.0, (t * 0.075 * tunnel_len).sin() * 0.5 + 0.5);
                            let height_here = (1.0 - dist / radius).max(0.0).powf(0.3) * radius;
                            if height_here > 0.0 {
                                let z_offs = nz.get((wposf / 512.0).into_array())
                                    * 48.0
                                    * ((1.0 - (t - 0.5).abs() * 2.0) * 8.0).min(1.0);
                                let depth =
                                    Lerp::lerp(tunnel.a.wpos.z as f64, tunnel.b.wpos.z as f64, t)
                                        + z_offs;
                                Some((
                                    (depth - height_here * 0.3) as i32,
                                    (depth + height_here * 1.35) as i32,
                                    z_offs,
                                ))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });

            for (min, max, z_offs) in tunnel_bounds {
                // Avoid cave entrances intersecting water
                let z_range = Lerp::lerp(
                    max,
                    min,
                    1.0 - (1.0 - ((col.alt - col.water_level) / 4.0).clamped(0.0, 1.0))
                        * (1.0 - ((col.alt - max as f32) / 8.0).clamped(0.0, 1.0)),
                )..max;
                write_column(canvas, col, level, wpos2d, z_range, z_offs, rng);
            }
        }
    });
}

struct Biome {
    humidity: f32,
    temp: f32,
    mineral: f32,
}

fn write_column(
    canvas: &mut Canvas,
    col: &ColumnSample,
    level: u32,
    wpos2d: Vec2<i32>,
    z_range: Range<i32>,
    z_offs: f64,
    rng: &mut impl Rng,
) {
    let info = canvas.info();

    let below = ((col.alt - z_range.start as f32) / 50.0).clamped(0.0, 1.0);
    let biome = Biome {
        humidity: Lerp::lerp(
            col.humidity,
            info.index()
                .noise
                .cave_nz
                .get(wpos2d.map(|e| e as f64 / 1024.0).into_array())
                .mul(0.5)
                .add(0.5) as f32,
            below,
        ),
        temp: Lerp::lerp(
            col.temp,
            info.index()
                .noise
                .cave_nz
                .get(wpos2d.map(|e| e as f64 / 2048.0).into_array())
                .add(
                    ((col.alt as f64 - z_range.start as f64) / (AVG_LEVEL_DEPTH as f64 * 2.75))
                        .clamped(0.0, 2.0),
                ) as f32,
            below,
        ),
        mineral: info
            .index()
            .noise
            .cave_nz
            .get(wpos2d.map(|e| e as f64 / 256.0).into_array())
            .mul(0.5)
            .add(0.5) as f32,
    };

    // Exposed to the sky
    let exposed = z_range.end as f32 > col.alt;

    let rand = RandomField::new(37 + level);

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
            .mul((biome.temp as f64 - 1.5).mul(30.0).clamped(0.0, 1.0))
            .mul(64.0)
            .max(-32.0)
    };

    let underground = ((col.alt as f32 - z_range.end as f32) / 80.0).clamped(0.0, 1.0);
    let mushroom_glow =
        underground * close(biome.humidity, 1.0, 0.6) * close(biome.temp, 0.25, 0.7);

    let dirt = if exposed { 0 } else { 1 };
    let bedrock = z_range.start + lava as i32;
    let base = bedrock + (stalactite * 0.4) as i32;
    let floor = base + dirt;
    let ceiling = z_range.end - stalactite as i32;
    for z in bedrock..z_range.end {
        let wpos = wpos2d.with_z(z);
        canvas.map(wpos, |block| {
            if !block.is_filled() {
                block.into_vacant()
            } else if z < z_range.start - 4 {
                Block::new(BlockKind::Lava, Rgb::new(255, 100, 0))
            } else if z < base || z >= ceiling {
                let stalactite: Rgb<i16> =
                    Lerp::lerp(Rgb::new(80, 100, 150), Rgb::new(0, 75, 200), mushroom_glow);
                Block::new(
                    if rand.chance(wpos, mushroom_glow * biome.mineral) {
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
                    Lerp::lerp(Rgb::new(100, 20, 50), Rgb::new(80, 80, 100), col.marble_mid);
                let surf_color: Rgb<i16> = Lerp::lerp(
                    Lerp::lerp(dry_mud, mycelium, mushroom_glow),
                    fire_rock,
                    (biome.temp - 1.0).mul(4.0).clamped(0.0, 1.0),
                );

                Block::new(BlockKind::Sand, surf_color.map(|e| e as u8))
            } else if let Some(sprite) = (z == floor && !exposed)
                .then(|| {
                    if rand.chance(wpos2d.with_z(0), mushroom_glow * 0.02) {
                        Some(SpriteKind::CaveMushroom)
                    } else if rand.chance(wpos2d.with_z(1), mushroom_glow * 0.1) {
                        Some(
                            *[
                                SpriteKind::CavernGrassBlueShort,
                                SpriteKind::CavernGrassBlueMedium,
                                SpriteKind::CavernGrassBlueLong,
                            ]
                            .choose(rng)
                            .unwrap(),
                        )
                    } else if rand.chance(
                        wpos2d.with_z(2),
                        close(biome.humidity, 0.0, 0.5) * biome.mineral * 0.005,
                    ) {
                        Some(SpriteKind::CrystalLow)
                    } else {
                        None
                    }
                })
                .flatten()
            {
                Block::air(sprite)
            } else if let Some(sprite) = (z == ceiling - 1)
                .then(|| {
                    if rand.chance(wpos2d.with_z(3), mushroom_glow * 0.02) {
                        Some(
                            *[SpriteKind::CavernMycelBlue, SpriteKind::CeilingMushroom]
                                .choose(rng)
                                .unwrap(),
                        )
                    } else if rand.chance(wpos2d.with_z(4), 0.0075) {
                        Some(
                            *[SpriteKind::CrystalHigh, SpriteKind::Orb]
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
                Block::air(SpriteKind::Empty)
            }
        });
    }
}
