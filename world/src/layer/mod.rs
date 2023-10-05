pub mod cave;
pub mod rock;
pub mod scatter;
pub mod shrub;
pub mod spot;
pub mod tree;
pub mod wildlife;

pub use self::{
    cave::apply_caves_to as apply_caves2_to, rock::apply_rocks_to, scatter::apply_scatter_to,
    shrub::apply_shrubs_to, spot::apply_spots_to, tree::apply_trees_to,
};

use crate::{
    column::ColumnSample,
    config::CONFIG,
    sim,
    util::{FastNoise, RandomField, RandomPerm, Sampler},
    Canvas, CanvasInfo, IndexRef,
};
use common::{
    assets::AssetExt,
    generation::{ChunkSupplement, EntityInfo},
    lottery::Lottery,
    terrain::{Block, BlockKind, SpriteKind},
    vol::{BaseVol, ReadVol, RectSizedVol, WriteVol},
};
use hashbrown::HashMap;
use noise::NoiseFn;
use rand::prelude::*;
use serde::Deserialize;
use std::{
    f32,
    ops::{Add, Mul, Range, Sub},
};
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    pub bridge: (u8, u8, u8),
    pub stalactite: (u8, u8, u8),
    pub cave_floor: (u8, u8, u8),
    pub cave_roof: (u8, u8, u8),
    pub dirt: (u8, u8, u8),
    pub scaffold: (u8, u8, u8),
    pub lava: (u8, u8, u8),
    pub vein: (u8, u8, u8),
}

const EMPTY_AIR: Block = Block::air(SpriteKind::Empty);

pub struct PathLocals {
    pub riverless_alt: f32,
    pub alt: f32,
    pub water_dist: f32,
    pub bridge_offset: f32,
    pub depth: i32,
}

impl PathLocals {
    pub fn new(info: &CanvasInfo, col: &ColumnSample, path_nearest: Vec2<f32>) -> PathLocals {
        // Try to use the column at the centre of the path for sampling to make them
        // flatter
        let col_pos = -info.wpos().map(|e| e as f32) + path_nearest;
        let col00 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 0));
        let col10 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 0));
        let col01 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(0, 1));
        let col11 = info.col(info.wpos() + col_pos.map(|e| e.floor() as i32) + Vec2::new(1, 1));
        let col_attr = |col: &ColumnSample| {
            Vec3::new(col.riverless_alt, col.alt, col.water_dist.unwrap_or(1000.0))
        };
        let [riverless_alt, alt, water_dist] = match (col00, col10, col01, col11) {
            (Some(col00), Some(col10), Some(col01), Some(col11)) => Lerp::lerp(
                Lerp::lerp(col_attr(col00), col_attr(col10), path_nearest.x.fract()),
                Lerp::lerp(col_attr(col01), col_attr(col11), path_nearest.x.fract()),
                path_nearest.y.fract(),
            ),
            _ => col_attr(col),
        }
        .into_array();
        let (bridge_offset, depth) = (
            ((water_dist.max(0.0) * 0.2).min(f32::consts::PI).cos() + 1.0) * 5.0,
            ((1.0 - ((water_dist + 2.0) * 0.3).min(0.0).cos().abs())
                * (riverless_alt + 5.0 - alt).max(0.0)
                * 1.75
                + 3.0) as i32,
        );
        PathLocals {
            riverless_alt,
            alt,
            water_dist,
            bridge_offset,
            depth,
        }
    }
}

pub fn apply_paths_to(canvas: &mut Canvas) {
    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let surface_z = col.riverless_alt.floor() as i32;

        let noisy_color = |color: Rgb<u8>, factor: u32| {
            let nz = RandomField::new(0).get(Vec3::new(wpos2d.x, wpos2d.y, surface_z));
            color.map(|e| {
                (e as u32 + nz % (factor * 2))
                    .saturating_sub(factor)
                    .min(255) as u8
            })
        };

        if let Some((path_dist, path_nearest, path, _)) =
            col.path.filter(|(dist, _, path, _)| *dist < path.width)
        {
            let inset = 0;

            let PathLocals {
                riverless_alt,
                alt: _,
                water_dist: _,
                bridge_offset,
                depth,
            } = PathLocals::new(&canvas.info(), col, path_nearest);
            let surface_z = (riverless_alt + bridge_offset).floor() as i32;

            for z in inset - depth..inset {
                canvas.set(
                    Vec3::new(wpos2d.x, wpos2d.y, surface_z + z),
                    if bridge_offset >= 2.0 && path_dist >= 3.0 || z < inset - 1 {
                        Block::new(
                            BlockKind::Rock,
                            noisy_color(info.index().colors.layer.bridge.into(), 8),
                        )
                    } else {
                        let path_color =
                            path.surface_color(col.sub_surface_color.map(|e| (e * 255.0) as u8));
                        Block::new(BlockKind::Earth, noisy_color(path_color, 8))
                    },
                );
            }
            let head_space = path.head_space(path_dist);
            for z in inset..inset + head_space {
                let pos = Vec3::new(wpos2d.x, wpos2d.y, surface_z + z);
                if canvas.get(pos).kind() != BlockKind::Water {
                    canvas.set(pos, EMPTY_AIR);
                }
            }
        }
    });
}

pub fn apply_trains_to(
    canvas: &mut Canvas,
    sim: &sim::WorldSim,
    sim_chunk: &sim::SimChunk,
    chunk_center_wpos2d: Vec2<i32>,
) {
    let mut splines = Vec::new();
    let g = |v: Vec2<f32>| -> Vec3<f32> {
        let path_nearest = sim
            .get_nearest_path(v.as_::<i32>())
            .map(|x| x.1)
            .unwrap_or(v.as_::<f32>());
        let alt = if let Some(c) = canvas.col_or_gen(v.as_::<i32>()) {
            let pl = PathLocals::new(canvas, &c, path_nearest);
            pl.riverless_alt + pl.bridge_offset + 0.75
        } else {
            sim_chunk.alt
        };
        v.with_z(alt)
    };
    fn hermite_to_bezier(
        p0: Vec3<f32>,
        m0: Vec3<f32>,
        p3: Vec3<f32>,
        m3: Vec3<f32>,
    ) -> CubicBezier3<f32> {
        let hermite = Vec4::new(p0, p3, m0, m3);
        let hermite = hermite.map(|v| v.with_w(0.0));
        let hermite: [[f32; 4]; 4] = hermite.map(|v: Vec4<f32>| v.into_array()).into_array();
        // https://courses.engr.illinois.edu/cs418/sp2009/notes/12-MoreSplines.pdf
        let mut m = Mat4::from_row_arrays([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
            [-3.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, -3.0, 3.0],
        ]);
        m.invert();
        let bezier = m * Mat4::from_row_arrays(hermite);
        let bezier: Vec4<Vec4<f32>> =
            Vec4::<[f32; 4]>::from(bezier.into_row_arrays()).map(Vec4::from);
        let bezier = bezier.map(Vec3::from);
        CubicBezier3::from(bezier)
    }
    for sim::NearestWaysData { bezier: bez, .. } in
        sim.get_nearest_ways(chunk_center_wpos2d, &|chunk| Some(chunk.path))
    {
        if bez.length_by_discretization(16) < 0.125 {
            continue;
        }
        let a = 0.0;
        let b = 1.0;
        for bez in bez.split((a + b) / 2.0) {
            let p0 = g(bez.evaluate(a));
            let p1 = g(bez.evaluate(a + (b - a) / 3.0));
            let p2 = g(bez.evaluate(a + 2.0 * (b - a) / 3.0));
            let p3 = g(bez.evaluate(b));
            splines.push(hermite_to_bezier(p0, 3.0 * (p1 - p0), p3, 3.0 * (p3 - p2)));
        }
    }
    for spline in splines.into_iter() {
        canvas.chunk.meta_mut().add_track(spline);
    }
}

pub fn apply_caves_to(canvas: &mut Canvas, rng: &mut impl Rng) {
    let info = canvas.info();
    canvas.foreach_col(|canvas, wpos2d, col| {
        let surface_z = col.alt.floor() as i32;

        if let Some((cave_dist, _, cave, _)) =
            col.cave.filter(|(dist, _, cave, _)| *dist < cave.width)
        {
            let cave_x = (cave_dist / cave.width).min(1.0);

            // Relative units
            let cave_depth = (col.alt - cave.alt).max(0.0);

            let cave_floor = 0.0 - 0.5 * (1.0 - cave_x.powi(2)).max(0.0).sqrt() * cave.width;
            let cave_height = (1.0 - cave_x.powi(2)).max(0.0).sqrt() * cave.width;

            let t = cave.water_dist.min(1.0);
            // Abs units
            let cave_base = Lerp::lerp(
                cave.alt + cave_floor,
                (cave.water_alt as f32).max(cave.alt + cave_floor),
                t,
            ) as i32;
            let cave_roof = (cave.alt + cave_height) as i32;

            for z in cave_base..cave_roof {
                if cave_x < 0.95
                    || info.index().noise.cave_nz.get(
                        Vec3::new(wpos2d.x, wpos2d.y, z)
                            .map(|e| e as f64 * 0.15)
                            .into_array(),
                    ) < 0.0
                {
                    // If the block a little above is liquid, and the water level is lower, we
                    // should stop carving out the cave in order to leave a
                    // ceiling, and not floating water.
                    if z >= cave.water_alt
                        && canvas.get(Vec3::new(wpos2d.x, wpos2d.y, z + 2)).is_liquid()
                    {
                        break;
                    }

                    let empty_block = if z < cave.water_alt {
                        Block::water(SpriteKind::Empty)
                    } else {
                        EMPTY_AIR
                    };

                    canvas.map(Vec3::new(wpos2d.x, wpos2d.y, z), |b| {
                        if b.is_liquid() { b } else { empty_block }
                    });
                }
            }
            let noisy_color = |color: Rgb<u8>, factor: u32| {
                let nz = RandomField::new(0).get(Vec3::new(wpos2d.x, wpos2d.y, surface_z));
                color.map(|e| {
                    (e as u32 + nz % (factor * 2))
                        .saturating_sub(factor)
                        .min(255) as u8
                })
            };

            let ridge_condition = cave_depth % 10.0 > 8.0 && cave_depth > 10.0;
            let pit_condition = cave_depth % 42.0 > 37.0 && cave_x > 0.6 && cave_depth > 200.0;
            let pit_depth = 30;
            let floor_dist = pit_condition as i32 * pit_depth;
            let vein_condition =
                cave_depth % 12.0 > 11.5 && cave_x > 0.1 && cave_x < 0.6 && cave_depth > 200.0;
            let stalactite_condition = cave_depth > 150.0;
            let vein_depth = 3;
            let vein_floor = cave_base - vein_depth;
            // Stalagtites
            let stalactites = info
                .index()
                .noise
                .cave_nz
                .get(wpos2d.map(|e| e as f64 * 0.18).into_array())
                .sub(0.5)
                .max(0.0)
                .mul(
                    (col.alt - cave_roof as f32 - 5.0)
                        .mul(0.15)
                        .clamped(0.0, 1.0) as f64,
                )
                .mul(45.0) as i32;

            // Generate stalactites if there's something for them to hold on to
            if canvas
                .get(Vec3::new(wpos2d.x, wpos2d.y, cave_roof))
                .is_filled()
                && stalactite_condition
            {
                for z in cave_roof - stalactites..cave_roof {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::WeakRock,
                            noisy_color(info.index().colors.layer.stalactite.into(), 8),
                        ),
                    );
                }
            }

            let ground_colors = if cave_roof - cave_base > 23 {
                noisy_color(info.index().colors.layer.cave_floor.into(), 8)
            } else {
                noisy_color(info.index().colors.layer.dirt.into(), 8)
            };

            //make pits
            for z in cave_base - pit_depth..cave_base {
                if pit_condition && (cave_roof - cave_base) > 10 {
                    let kind = if z < cave.water_alt {
                        BlockKind::Water
                    } else if z < (cave_base - pit_depth) + (3 * pit_depth / 4) {
                        BlockKind::Lava
                    } else {
                        BlockKind::Air
                    };
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(kind, noisy_color(info.index().colors.layer.lava.into(), 8)),
                    );
                }
            }
            //fill bottom of pits
            for z in cave_base - pit_depth
                ..cave_base - pit_depth + ((cave_x.powf(4.0) * (pit_depth as f32 + 3.0)) as i32) + 1
            {
                if (cave_roof - cave_base) > 10 && pit_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(BlockKind::WeakRock, ground_colors),
                    );
                }
            }
            //empty veins
            for z in cave_base - vein_depth..cave_base {
                if vein_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::Air,
                            noisy_color(info.index().colors.layer.scaffold.into(), 8),
                        ),
                    );
                }
            }

            //fill veins except bottom later
            for z in cave_base - vein_depth + 1..cave_base {
                if vein_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::GlowingWeakRock,
                            noisy_color(info.index().colors.layer.vein.into(), 16),
                        ),
                    );
                }
            }
            //fill some of bottom
            for z in cave_base - vein_depth..cave_base - vein_depth + 1 {
                if rng.gen::<f32>() < 0.5 && vein_condition {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::GlowingWeakRock,
                            noisy_color(info.index().colors.layer.vein.into(), 16),
                        ),
                    );
                }
            }
            if vein_condition && rng.gen::<f32>() > 0.7 {
                let kind = *Lottery::<SpriteKind>::load_expect("common.cave_scatter.vein")
                    .read()
                    .choose();
                canvas.map(Vec3::new(wpos2d.x, wpos2d.y, vein_floor), |block| {
                    block.with_sprite(kind)
                });
            }

            //fill normal floor
            for z in cave_base..cave_base + 1 {
                if cave_depth > 15.0
                    && (cave_roof - cave_base) > 10
                    && !pit_condition
                    && !vein_condition
                {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(BlockKind::WeakRock, ground_colors),
                    );
                }
            }
            //fill roof
            for z in cave_roof - 1..cave_roof {
                if cave_depth > 30.0 && (cave_roof - cave_base) > 10 {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::WeakRock,
                            noisy_color(info.index().colors.layer.cave_roof.into(), 8),
                        ),
                    );
                }
            }
            //add ridges
            for z in cave_roof - 4..cave_roof {
                if ridge_condition && (cave_roof - cave_base) > 10 {
                    canvas.set(
                        Vec3::new(wpos2d.x, wpos2d.y, z),
                        Block::new(
                            BlockKind::WeakRock,
                            noisy_color(info.index().colors.layer.scaffold.into(), 8),
                        ),
                    );
                }
            }

            let cave_roof_adjusted = if (cave_roof - cave_base) > 10 {
                cave_roof - 1
            } else {
                cave_roof
            };

            let cave_floor_adjusted = if (cave_roof - cave_base) > 10 {
                cave_base + 1 - floor_dist
            } else {
                cave_base - floor_dist
            };
            // Scatter things on cave floors
            if cave_floor_adjusted + 1 < cave.water_alt {
                if cave_depth > 40.0 && cave_depth < 80.0 {
                    if rng.gen::<f32>() < 0.14 * (cave_x.max(0.5).powf(4.0)) && !vein_condition {
                        let kind = *Lottery::<SpriteKind>::load_expect(
                            "common.cave_scatter.shallow_water_floor",
                        )
                        .read()
                        .choose();
                        canvas.map(
                            Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                            |block| block.with_sprite(kind),
                        );
                    }
                } else if rng.gen::<f32>() < 0.065 * (cave_x.max(0.5).powf(4.0))
                    && !vein_condition
                    && cave_depth > 40.0
                {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.deep_water_floor")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                        |block| block.with_sprite(kind),
                    );
                };
            } else if cave_depth > 40.0 && cave_depth < 80.0 {
                if rng.gen::<f32>() < 0.14 * (cave_x.max(0.5).powf(4.0)) && !vein_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.shallow_floor")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                        |block| block.with_sprite(kind),
                    );
                }
            } else if cave_depth < 200.0 && cave_depth > 80.0 {
                if rng.gen::<f32>() < 0.065 * (cave_x.max(0.5).powf(4.0)) && !vein_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.deep_floor")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                        |block| block.with_sprite(kind),
                    );
                }
            } else if rng.gen::<f32>() < 0.08 * (cave_x.max(0.5).powf(4.0))
                && cave_depth > 40.0
                && !vein_condition
            {
                let kind = *Lottery::<SpriteKind>::load_expect("common.cave_scatter.dark_floor")
                    .read()
                    .choose();
                canvas.map(
                    Vec3::new(wpos2d.x, wpos2d.y, cave_floor_adjusted),
                    |block| block.with_sprite(kind),
                );
            };

            // Scatter things on cave ceilings
            if cave_roof_adjusted - 1 < cave.water_alt {
                if cave_depth > 40.0 && cave_depth < 80.0 {
                    if rng.gen::<f32>() < 0.02 * (cave_x.max(0.5).powf(4.0)) && !ridge_condition {
                        let kind = *Lottery::<SpriteKind>::load_expect(
                            "common.cave_scatter.shallow_water_ceiling",
                        )
                        .read()
                        .choose();
                        canvas.map(
                            Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                            |block| block.with_sprite(kind),
                        );
                    }
                } else if rng.gen::<f32>() < 0.1 * (cave_x.max(0.5).powf(4.0))
                    && !ridge_condition
                    && cave_depth > 40.0
                {
                    let kind = *Lottery::<SpriteKind>::load_expect(
                        "common.cave_scatter.deep_water_ceiling",
                    )
                    .read()
                    .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                        |block| block.with_sprite(kind),
                    );
                };
            } else if cave_depth > 40.0 && cave_depth < 80.0 {
                if rng.gen::<f32>() < 0.3 * (cave_x.max(0.5).powf(4.0)) && !ridge_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.shallow_ceiling")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                        |block| block.with_sprite(kind),
                    );
                }
            } else if cave_depth < 200.0 && cave_depth > 80.0 {
                if rng.gen::<f32>() < 0.3 * (cave_x.max(0.5).powf(4.0)) && !ridge_condition {
                    let kind =
                        *Lottery::<SpriteKind>::load_expect("common.cave_scatter.deep_ceiling")
                            .read()
                            .choose();
                    canvas.map(
                        Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                        |block| block.with_sprite(kind),
                    );
                }
            } else if rng.gen::<f32>() < 0.02 * (cave_x.max(0.5).powf(4.0))
                && !ridge_condition
                && cave_depth > 40.0
            {
                let kind = *Lottery::<SpriteKind>::load_expect("common.cave_scatter.dark_ceiling")
                    .read()
                    .choose();
                canvas.map(
                    Vec3::new(wpos2d.x, wpos2d.y, cave_roof_adjusted - 1),
                    |block| block.with_sprite(kind),
                );
            };
        }
    });
}
pub fn apply_caves_supplement<'a>(
    // NOTE: Used only for dynamic elements like chests and entities!
    dynamic_rng: &mut impl Rng,
    wpos2d: Vec2<i32>,
    mut get_column: impl FnMut(Vec2<i32>) -> Option<&'a ColumnSample<'a>>,
    vol: &(impl BaseVol<Vox = Block> + RectSizedVol + ReadVol + WriteVol),
    index: IndexRef,
    supplement: &mut ChunkSupplement,
) {
    for y in 0..vol.size_xy().y as i32 {
        for x in 0..vol.size_xy().x as i32 {
            let offs = Vec2::new(x, y);

            let wpos2d = wpos2d + offs;

            // Sample terrain
            let col_sample = if let Some(col_sample) = get_column(offs) {
                col_sample
            } else {
                continue;
            };
            let surface_z = col_sample.riverless_alt.floor() as i32;

            if let Some((cave_dist, _, cave, _)) = col_sample
                .cave
                .filter(|(dist, _, cave, _)| *dist < cave.width)
            {
                let cave_x = (cave_dist / cave.width).min(1.0);

                // Relative units
                let cave_floor = 0.0 - 0.5 * (1.0 - cave_x.powi(2)).max(0.0).sqrt() * cave.width;

                // Abs units
                let cave_base = (cave.alt + cave_floor) as i32;

                // slightly different from earlier cave depth?
                let cave_depth = (col_sample.alt - cave.alt).max(0.0);

                // Scatter things in caves
                if let Some(z) = (-4..8).map(|z| cave_base + z).find(|z| {
                    (0..2).all(|z_offs| {
                        vol.get(offs.with_z(z + z_offs))
                            .map_or(true, |b| b.is_fluid())
                    })
                }) {
                    if RandomField::new(index.seed).chance(wpos2d.into(), 0.0014)
                        && cave_base < surface_z - 40
                    {
                        let entity = EntityInfo::at(wpos2d.map(|e| e as f32).with_z(z as f32));
                        let entity = {
                            let asset = if z < cave.water_alt {
                                if cave_depth < 190.0 {
                                    match dynamic_rng.gen_range(0..2) {
                                        0 => "common.entity.wild.aggressive.sea_crocodile",
                                        _ => "common.entity.wild.aggressive.hakulaq",
                                    }
                                } else {
                                    match dynamic_rng.gen_range(0..3) {
                                        0 => "common.entity.wild.aggressive.sea_crocodile",
                                        1 => "common.entity.wild.aggressive.hakulaq",
                                        _ => "common.entity.wild.aggressive.akhlut",
                                    }
                                }
                            } else if cave_depth < 70.0 {
                                match dynamic_rng.gen_range(0..4) {
                                    0 => "common.entity.wild.peaceful.truffler",
                                    1 => "common.entity.wild.aggressive.dodarock",
                                    2 => "common.entity.wild.peaceful.holladon",
                                    _ => "common.entity.wild.aggressive.batfox",
                                }
                            } else if cave_depth < 120.0 {
                                match dynamic_rng.gen_range(0..10) {
                                    2 => "common.entity.wild.aggressive.rocksnapper",
                                    5 => "common.entity.wild.aggressive.cave_salamander",
                                    7 => "common.entity.wild.aggressive.cave_spider",
                                    8 => "common.entity.wild.peaceful.crawler_molten",
                                    _ => "common.entity.wild.aggressive.asp",
                                }
                            } else if cave_depth < 190.0 {
                                match dynamic_rng.gen_range(0..5) {
                                    1 => "common.entity.wild.aggressive.rocksnapper",
                                    2 => "common.entity.wild.aggressive.lavadrake",
                                    3 => "common.entity.wild.aggressive.black_widow",
                                    _ => "common.entity.wild.aggressive.basilisk",
                                }
                            } else {
                                match dynamic_rng.gen_range(0..5) {
                                    0 => "common.entity.wild.aggressive.ogre",
                                    1 => "common.entity.wild.aggressive.cyclops",
                                    2 => "common.entity.wild.aggressive.wendigo",
                                    3 => match dynamic_rng.gen_range(0..2) {
                                        0 => "common.entity.wild.aggressive.blue_oni",
                                        _ => "common.entity.wild.aggressive.red_oni",
                                    },
                                    _ => "common.entity.wild.aggressive.cave_troll",
                                }
                            };
                            entity.with_asset_expect(asset, dynamic_rng)
                        };

                        supplement.add_entity(entity);
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
pub fn apply_coral_to(canvas: &mut Canvas) {
    let info = canvas.info();

    if !info.chunk.river.near_water() {
        return; // Don't bother with coral for a chunk nowhere near water
    }

    canvas.foreach_col(|canvas, wpos2d, col| {
        const CORAL_DEPTH: Range<f32> = 14.0..32.0;
        const CORAL_HEIGHT: f32 = 14.0;
        const CORAL_DEPTH_FADEOUT: f32 = 5.0;
        const CORAL_SCALE: f32 = 10.0;

        let water_depth = col.water_level - col.alt;

        if !CORAL_DEPTH.contains(&water_depth) {
            return; // Avoid coral entirely for this column if we're outside coral depths
        }

        for z in col.alt.floor() as i32..(col.alt + CORAL_HEIGHT) as i32 {
            let wpos = Vec3::new(wpos2d.x, wpos2d.y, z);

            let coral_factor = Lerp::lerp(
                1.0,
                0.0,
                // Fade coral out due to incorrect depth
                ((water_depth.clamped(CORAL_DEPTH.start, CORAL_DEPTH.end) - water_depth).abs()
                    / CORAL_DEPTH_FADEOUT)
                    .min(1.0),
            ) * Lerp::lerp(
                1.0,
                0.0,
                // Fade coral out due to incorrect altitude above the seabed
                ((z as f32 - col.alt) / CORAL_HEIGHT).powi(2),
            ) * FastNoise::new(info.index.seed + 7)
                .get(wpos.map(|e| e as f64) / 32.0)
                .sub(0.2)
                .mul(100.0)
                .clamped(0.0, 1.0);

            let nz = Vec3::iota().map(|e: u32| FastNoise::new(info.index.seed + e * 177));

            let wpos_warped = wpos.map(|e| e as f32)
                + nz.map(|nz| {
                    nz.get(wpos.map(|e| e as f64) / CORAL_SCALE as f64) * CORAL_SCALE * 0.3
                });

            // let is_coral = FastNoise2d::new(info.index.seed + 17)
            //     .get(wpos_warped.xy().map(|e| e as f64) / CORAL_SCALE)
            //     .sub(1.0 - coral_factor)
            //     .max(0.0)
            //     .div(coral_factor) > 0.5;

            let is_coral = [
                FastNoise::new(info.index.seed),
                FastNoise::new(info.index.seed + 177),
            ]
            .iter()
            .all(|nz| {
                nz.get(wpos_warped.map(|e| e as f64) / CORAL_SCALE as f64)
                    .abs()
                    < coral_factor * 0.3
            });

            if is_coral {
                canvas.set(wpos, Block::new(BlockKind::Rock, Rgb::new(170, 220, 210)));
            }
        }
    });
}

pub fn apply_caverns_to<R: Rng>(canvas: &mut Canvas, dynamic_rng: &mut R) {
    let info = canvas.info();

    let canvern_nz_at = |wpos2d: Vec2<i32>| {
        // Horizontal average scale of caverns
        let scale = 2048.0;
        // How common should they be? (0.0 - 1.0)
        let common = 0.15;

        let cavern_nz = info
            .index()
            .noise
            .cave_nz
            .get((wpos2d.map(|e| e as f64) / scale).into_array()) as f32;
        ((cavern_nz * 0.5 + 0.5 - (1.0 - common)).max(0.0) / common).powf(common * 2.0)
    };

    // Get cavern attributes at a position
    let cavern_at = |wpos2d| {
        let alt = info.land().get_alt_approx(wpos2d);

        // Range of heights for the caverns
        let height_range = 16.0..250.0;
        // Minimum distance below the surface
        let surface_clearance = 64.0;

        let cavern_avg_height = Lerp::lerp(
            height_range.start,
            height_range.end,
            info.index()
                .noise
                .cave_nz
                .get((wpos2d.map(|e| e as f64) / 300.0).into_array()) as f32
                * 0.5
                + 0.5,
        );

        let cavern_avg_alt =
            CONFIG.sea_level.min(alt * 0.25) - height_range.end - surface_clearance;

        let cavern = canvern_nz_at(wpos2d);
        let cavern_height = cavern * cavern_avg_height;

        // Stalagtites
        let stalactite = info
            .index()
            .noise
            .cave_nz
            .get(wpos2d.map(|e| e as f64 * 0.015).into_array())
            .sub(0.5)
            .max(0.0)
            .mul((cavern_height as f64 - 5.0).mul(0.15).clamped(0.0, 1.0))
            .mul(32.0 + cavern_avg_height as f64);

        let hill = info
            .index()
            .noise
            .cave_nz
            .get((wpos2d.map(|e| e as f64) / 96.0).into_array()) as f32
            * cavern
            * 24.0;
        let rugged = 0.4; // How bumpy should the floor be relative to the ceiling?
        let cavern_bottom = (cavern_avg_alt - cavern_height * rugged + hill) as i32;
        let cavern_avg_bottom =
            (cavern_avg_alt - ((height_range.start + height_range.end) * 0.5) * rugged) as i32;
        let cavern_top = (cavern_avg_alt + cavern_height) as i32;
        let cavern_avg_top = (cavern_avg_alt + cavern_avg_height) as i32;

        // Stalagmites rise up to meet stalactites
        let stalagmite = stalactite;

        let floor = stalagmite as i32;

        (
            cavern_bottom,
            cavern_top,
            cavern_avg_bottom,
            cavern_avg_top,
            floor,
            stalactite,
            cavern_avg_bottom + 16, // Water level
        )
    };

    let mut mushroom_cache = HashMap::new();

    struct Mushroom {
        pos: Vec3<i32>,
        stalk: f32,
        head_color: Rgb<u8>,
    }

    // Get mushroom block, if any, at a position
    let mut get_mushroom = |wpos: Vec3<i32>, dynamic_rng: &mut R| {
        for (wpos2d, seed) in info.chunks().gen_ctx.structure_gen.get(wpos.xy()) {
            let mushroom = if let Some(mushroom) =
                mushroom_cache.entry(wpos2d).or_insert_with(|| {
                    let mut rng = RandomPerm::new(seed);
                    let (cavern_bottom, cavern_top, _, _, floor, _, water_level) =
                        cavern_at(wpos2d);
                    let pos = wpos2d.with_z(cavern_bottom + floor);
                    if rng.gen_bool(0.15)
                        && cavern_top - cavern_bottom > 32
                        && pos.z > water_level - 2
                    {
                        Some(Mushroom {
                            pos,
                            stalk: 12.0 + rng.gen::<f32>().powf(2.0) * 35.0,
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
                    FastNoise::new(seed).get(wposf * warp_freq),
                    FastNoise::new(seed + 1).get(wposf * warp_freq),
                    FastNoise::new(seed + 2).get(wposf * warp_freq),
                ) * warp_amp
                    * (wposf.z as f32 - mushroom.pos.z as f32)
                        .mul(0.1)
                        .clamped(0.0, 1.0);

            let rpos = wposf_warped - mushroom.pos.map(|e| e as f32);

            let stalk_radius = 2.5f32;
            let head_radius = 18.0f32;
            let head_height = 16.0;

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

    canvas.foreach_col(|canvas, wpos2d, _col| {
        if canvern_nz_at(wpos2d) <= 0.0 {
            return;
        }

        let (
            cavern_bottom,
            cavern_top,
            cavern_avg_bottom,
            cavern_avg_top,
            floor,
            stalactite,
            water_level,
        ) = cavern_at(wpos2d);

        let mini_stalactite = info
            .index()
            .noise
            .cave_nz
            .get(wpos2d.map(|e| e as f64 * 0.08).into_array())
            .sub(0.5)
            .max(0.0)
            .mul(
                ((cavern_top - cavern_bottom) as f64 - 5.0)
                    .mul(0.15)
                    .clamped(0.0, 1.0),
            )
            .mul(24.0 + (cavern_avg_top - cavern_avg_bottom) as f64 * 0.2);
        let stalactite_height = (stalactite + mini_stalactite) as i32;

        let moss_common = 1.5;
        let moss = info
            .index()
            .noise
            .cave_nz
            .get(wpos2d.map(|e| e as f64 * 0.035).into_array())
            .sub(1.0 - moss_common)
            .max(0.0)
            .mul(1.0 / moss_common)
            .powf(8.0 * moss_common)
            .mul(
                ((cavern_top - cavern_bottom) as f64)
                    .mul(0.15)
                    .clamped(0.0, 1.0),
            )
            .mul(16.0 + (cavern_avg_top - cavern_avg_bottom) as f64 * 0.35);

        let plant_factor = info
            .index()
            .noise
            .cave_nz
            .get(wpos2d.map(|e| e as f64 * 0.015).into_array())
            .add(1.0)
            .mul(0.5)
            .powf(2.0);

        let is_vine = |wpos: Vec3<f32>, dynamic_rng: &mut R| {
            let wpos = wpos + wpos.xy().yx().with_z(0.0) * 0.2; // A little twist
            let dims = Vec2::new(7.0, 256.0); // Long and thin
            let vine_posf = (wpos + Vec2::new(0.0, (wpos.x / dims.x).floor() * 733.0)) / dims; // ~Random offset
            let vine_pos = vine_posf.map(|e| e.floor() as i32);
            let mut rng = RandomPerm::new(((vine_pos.x << 16) | vine_pos.y) as u32); // Rng for vine attributes
            if rng.gen_bool(0.2) {
                let vine_height = (cavern_avg_top - cavern_avg_bottom).max(64) as f32;
                let vine_base = cavern_avg_bottom as f32 + rng.gen_range(48.0..vine_height);
                let vine_y = (vine_posf.y.fract() - 0.5).abs() * 2.0 * dims.y;
                let vine_reach = (vine_y * 0.05).powf(2.0).min(1024.0);
                let vine_z = vine_base + vine_reach;
                if Vec2::new(vine_posf.x.fract() * 2.0 - 1.0, (wpos.z - vine_z) / 5.0)
                    .magnitude_squared()
                    < 1.0f32
                {
                    let kind = if dynamic_rng.gen_bool(0.025) {
                        BlockKind::GlowingRock
                    } else {
                        BlockKind::Leaves
                    };
                    Some(Block::new(
                        kind,
                        Rgb::new(
                            85,
                            (vine_y + vine_reach).mul(0.05).sin().mul(35.0).add(85.0) as u8,
                            20,
                        ),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        };

        let cavern_top = cavern_top;
        let mut last_kind = BlockKind::Rock;
        for z in cavern_bottom - 1..cavern_top {
            use SpriteKind::*;

            let wpos = wpos2d.with_z(z);
            let wposf = wpos.map(|e| e as f32);

            let block = if z < cavern_bottom {
                if z > water_level + dynamic_rng.gen_range(4..16) {
                    Block::new(BlockKind::Grass, Rgb::new(10, 75, 90))
                } else {
                    Block::new(BlockKind::Rock, Rgb::new(50, 40, 10))
                }
            } else if z < cavern_bottom + floor {
                Block::new(BlockKind::WeakRock, Rgb::new(110, 120, 150))
            } else if z > cavern_top - stalactite_height {
                if dynamic_rng.gen_bool(0.0035) {
                    // Glowing rock in stalactites
                    Block::new(BlockKind::GlowingRock, Rgb::new(30, 150, 120))
                } else {
                    Block::new(BlockKind::WeakRock, Rgb::new(110, 120, 150))
                }
            } else if let Some(mushroom_block) = get_mushroom(wpos, dynamic_rng) {
                mushroom_block
            } else if z > cavern_top - moss as i32 {
                let kind = if dynamic_rng
                    .gen_bool(0.05 / (1.0 + ((cavern_top - z).max(0) as f64).mul(0.1)))
                {
                    BlockKind::GlowingMushroom
                } else {
                    BlockKind::Leaves
                };
                Block::new(kind, Rgb::new(50, 120, 160))
            } else if z < water_level {
                Block::water(Empty).with_sprite(
                    if z == cavern_bottom + floor && dynamic_rng.gen_bool(0.01) {
                        *[Seagrass, SeaGrapes, SeaweedTemperate, StonyCoral]
                            .choose(dynamic_rng)
                            .unwrap()
                    } else {
                        Empty
                    },
                )
            } else if z == water_level
                && dynamic_rng.gen_bool(Lerp::lerp(0.0, 0.05, plant_factor))
                && last_kind == BlockKind::Water
            {
                Block::air(CavernLillypadBlue)
            } else if z == cavern_bottom + floor
                && dynamic_rng.gen_bool(Lerp::lerp(0.0, 0.5, plant_factor))
                && last_kind == BlockKind::Grass
            {
                Block::air(
                    *if dynamic_rng.gen_bool(0.9) {
                        // High density
                        &[
                            CavernGrassBlueShort,
                            CavernGrassBlueMedium,
                            CavernGrassBlueLong,
                        ] as &[_]
                    } else if dynamic_rng.gen_bool(0.5) {
                        // Medium density
                        &[CaveMushroom] as &[_]
                    } else {
                        // Low density
                        &[LeafyPlant, Fern, Pyrebloom, Moonbell, Welwitch, GrassBlue] as &[_]
                    }
                    .choose(dynamic_rng)
                    .unwrap(),
                )
            } else if z == cavern_top - 1 && dynamic_rng.gen_bool(0.001) {
                Block::air(
                    *[CrystalHigh, CeilingMushroom, Orb, CavernMycelBlue]
                        .choose(dynamic_rng)
                        .unwrap(),
                )
            } else if let Some(vine) = is_vine(wposf, dynamic_rng)
                .or_else(|| is_vine(wposf.xy().yx().with_z(wposf.z), dynamic_rng))
            {
                vine
            } else {
                Block::empty()
            };

            last_kind = block.kind();

            let block = if block.is_filled() {
                Block::new(
                    block.kind(),
                    block.get_color().unwrap_or_default().map(|e| {
                        (e as f32 * dynamic_rng.gen_range(0.95..1.05)).clamped(0.0, 255.0) as u8
                    }),
                )
            } else {
                block
            };

            canvas.set(wpos, block);
        }
    });
}
