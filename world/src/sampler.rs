use std::{
    ops::{Add, Div, Mul, Neg, Sub},
    sync::Arc,
};
use vek::*;
use lazy_static::lazy_static;
use noise::NoiseFn;
use common::{
    assets,
    terrain::{Block, Structure},
    vol::{ReadVol, VolSize, Vox},
};
use crate::{
    structure::StructureGen2d,
    sim::{
        GenCtx,
        WorldSim,
        SEA_LEVEL,
        MOUNTAIN_HEIGHT,
    },
    Cache,
};

pub struct Sampler<'a> {
    sim: &'a WorldSim,
    sample2d_cache: Cache<Vec2<i32>, Option<Sample2d>>,
}

impl<'a> Sampler<'a> {
    pub(crate) fn new(sim: &'a WorldSim) -> Self {
        Self {
            sim,
            sample2d_cache: Cache::with_capacity(1024),
        }
    }

    fn sample_2d_impl(sim: &WorldSim, wpos: Vec2<i32>) -> Option<Sample2d> {
        let wposf = wpos.map(|e| e as f64);

        let alt_base = sim.get_interpolated(wpos, |chunk| chunk.alt_base)?;
        let chaos = sim.get_interpolated(wpos, |chunk| chunk.chaos)?;
        let temp = sim.get_interpolated(wpos, |chunk| chunk.temp)?;
        let rockiness = sim.get_interpolated(wpos, |chunk| chunk.rockiness)?;
        let tree_density = sim.get_interpolated(wpos, |chunk| chunk.tree_density)?;

        let rock = (sim.gen_ctx.small_nz.get((wposf.div(100.0)).into_array()) as f32)
            .mul(rockiness)
            .sub(0.35)
            .max(0.0)
            .mul(6.0);

        let alt = sim.get_interpolated(wpos, |chunk| chunk.alt)?
            + sim.gen_ctx.small_nz.get((wposf.div(256.0)).into_array()) as f32
                * chaos.max(0.2)
                * 64.0;

        let wposf3d = Vec3::new(wposf.x, wposf.y, alt as f64);

        let marble = (sim.gen_ctx.hill_nz.get((wposf3d.div(48.0)).into_array()) as f32)
            .add(1.0)
            .mul(0.5);

        // Colours
        let cold_grass = Rgb::new(0.0, 0.55, 0.15);
        let warm_grass = Rgb::new(0.25, 0.8, 0.05);
        let cold_stone = Rgb::new(0.55, 0.7, 0.75);
        let warm_stone = Rgb::new(0.65, 0.65, 0.35);
        let beach_sand = Rgb::new(0.93, 0.84, 0.33);
        let desert_sand = Rgb::new(0.97, 0.84, 0.23);
        let snow = Rgb::broadcast(1.0);

        let grass = Rgb::lerp(cold_grass, warm_grass, marble);
        let grassland = grass; //Rgb::lerp(grass, warm_stone, rock.mul(5.0).min(0.8));
        let cliff = Rgb::lerp(cold_stone, warm_stone, marble);

        let ground = Rgb::lerp(
            Rgb::lerp(snow, grassland, temp.add(0.4).mul(32.0).sub(0.4)),
            desert_sand,
            temp.sub(0.4).mul(32.0).add(0.4),
        );

        // Caves
        let cave_at = |wposf: Vec2<f64>| {
            (sim.gen_ctx.cave_0_nz.get(
                Vec3::new(wposf.x, wposf.y, alt as f64 * 8.0)
                    .div(800.0)
                    .into_array(),
            ) as f32)
                .powf(2.0)
                .neg()
                .add(1.0)
                .mul((1.15 - chaos).min(1.0))
        };
        let cave_xy = cave_at(wposf);
        let cave_alt = alt - 32.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(48.0).into_array()) as f32)
                * 8.0
            + (sim
                .gen_ctx
                .cave_1_nz
                .get(Vec2::new(wposf.x, wposf.y).div(300.0).into_array()) as f32)
                .add(1.0)
                .mul(0.5)
                .powf(8.0)
                .mul(256.0);

        Some(Sample2d {
            alt,
            chaos,
            surface_color: Rgb::lerp(
                beach_sand,
                // Land
                Rgb::lerp(
                    ground,
                    // Mountain
                    Rgb::lerp(
                        cliff,
                        snow,
                        (alt - SEA_LEVEL
                            - 0.3 * MOUNTAIN_HEIGHT
                            - alt_base
                            - temp * 96.0
                            - marble * 24.0)
                            / 12.0,
                    ),
                    (alt - SEA_LEVEL - 0.15 * MOUNTAIN_HEIGHT) / 180.0,
                ),
                // Beach
                (alt - SEA_LEVEL - 2.0) / 5.0,
            ),
            tree_density,
            close_trees: sim.tree_gen.sample(wpos),
            cave_xy,
            cave_alt,
            rock,
        })
    }

    pub fn sample_2d(&mut self, wpos2d: Vec2<i32>) -> Option<&Sample2d> {
        let sim = &self.sim;
        self.sample2d_cache
            .get(wpos2d, |wpos2d| Self::sample_2d_impl(sim, wpos2d))
            .as_ref()
    }

    pub fn sample_3d(&mut self, wpos: Vec3<i32>) -> Option<Sample3d> {
        let wpos2d = Vec2::from(wpos);
        let wposf = wpos.map(|e| e as f64);

        // Sample 2D terrain attributes

        let Sample2d {
            alt,
            chaos,
            surface_color,
            tree_density,
            close_trees,
            cave_xy,
            cave_alt,
            rock,
        } = *self.sample_2d(wpos2d)?;

        // Apply warping

        let warp = (self
            .sim
            .gen_ctx
            .warp_nz
            .get((wposf.div(Vec3::new(120.0, 120.0, 150.0))).into_array())
            as f32)
            .mul((chaos - 0.1).max(0.0))
            .mul(110.0);

        let height = alt + warp;

        // Sample blocks

        let air = Block::empty();
        let stone = Block::new(2, Rgb::new(200, 220, 255));
        let dirt = Block::new(1, Rgb::new(128, 90, 0));
        let sand = Block::new(1, Rgb::new(180, 150, 50));
        let water = Block::new(1, Rgb::new(100, 150, 255));
        let warm_stone = Block::new(1, Rgb::new(165, 165, 90));

        let block = if (wposf.z as f32) < height - 4.0 {
            // Underground
            Some(stone)
        } else if (wposf.z as f32) < height {
            // Surface
            Some(Block::new(1, surface_color.map(|e| (e * 255.0) as u8)))
        } else if (wposf.z as f32) < SEA_LEVEL {
            // Ocean
            Some(water)
        } else {
            None
        };

        // Caves
        let block = block.and_then(|block| {
            // Underground
            let cave = cave_xy.powf(2.0)
                * (wposf.z as f32 - cave_alt)
                    .div(40.0)
                    .powf(4.0)
                    .neg()
                    .add(1.0)
                > 0.9993;

            if cave {
                None
            } else {
                Some(block)
            }
        });

        // Rocks
        let block = block.or_else(|| {
            if (height + 2.5 - wposf.z as f32).div(7.5).abs().powf(2.0) < rock {
                Some(warm_stone)
            } else {
                None
            }
        });

        let block = match block {
            Some(block) => block,
            None => (&close_trees)
                .iter()
                .fold(air, |block, (tree_pos, tree_seed)| {
                    match self.sample_2d(*tree_pos) {
                        Some(tree_sample)
                            if tree_sample.tree_density
                                > 0.5 + (*tree_seed as f32 / 1000.0).fract() * 0.2 =>
                        {
                            let tree_pos3d =
                                Vec3::new(tree_pos.x, tree_pos.y, tree_sample.alt as i32);
                            let rpos = wpos - tree_pos3d;
                            block.or(TREES[*tree_seed as usize % TREES.len()]
                                .get((rpos * 160) / 128) // Scaling
                                .map(|b| b.clone())
                                .unwrap_or(Block::empty()))
                        }
                        _ => block,
                    }
                }),
        };

        Some(Sample3d { block })
    }
}

#[derive(Copy, Clone)]
pub struct Sample2d {
    pub alt: f32,
    pub chaos: f32,
    pub surface_color: Rgb<f32>,
    pub tree_density: f32,
    pub close_trees: [(Vec2<i32>, u32); 9],
    pub cave_xy: f32,
    pub cave_alt: f32,
    pub rock: f32,
}

#[derive(Copy, Clone)]
pub struct Sample3d {
    pub block: Block,
}

lazy_static! {
    static ref TREES: [Arc<Structure>; 61] = [
        // green oaks
        assets::load_map("world/tree/oak_green/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/3.vox", |s: Structure| s
            .with_center(Vec3::new(16, 20, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/4.vox", |s: Structure| s
            .with_center(Vec3::new(18, 21, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/5.vox", |s: Structure| s
            .with_center(Vec3::new(18, 18, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/6.vox", |s: Structure| s
            .with_center(Vec3::new(16, 21, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/7.vox", |s: Structure| s
            .with_center(Vec3::new(20, 19, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/8.vox", |s: Structure| s
            .with_center(Vec3::new(22, 20, 14)))
        .unwrap(),
        assets::load_map("world/tree/oak_green/9.vox", |s: Structure| s
            .with_center(Vec3::new(26, 26, 14)))
        .unwrap(),
        // green pines
        assets::load_map("world/tree/pine_green/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // green pines 2
         assets::load_map("world/tree/pine_green_2/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_green_2/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // blue pines
        assets::load_map("world/tree/pine_blue/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/pine_blue/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // temperate small
        assets::load_map("world/tree/temperate_small/1.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/2.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/3.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/4.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/5.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        assets::load_map("world/tree/temperate_small/6.vox", |s: Structure| s
            .with_center(Vec3::new(4, 4, 7)))
        .unwrap(),
        // birch
        assets::load_map("world/tree/birch/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/3.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/4.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/5.vox", |s: Structure| s
            .with_center(Vec3::new(9, 11, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/6.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/8.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/9.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/11.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 5)))
        .unwrap(),
        assets::load_map("world/tree/birch/12.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 5)))
        .unwrap(),
        // poplar
        assets::load_map("world/tree/poplar/1.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/2.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/3.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/4.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/5.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/6.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/7.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/8.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/9.vox", |s: Structure| s
            .with_center(Vec3::new(6, 6, 10)))
        .unwrap(),
        assets::load_map("world/tree/poplar/10.vox", |s: Structure| s
            .with_center(Vec3::new(7, 7, 10)))
        .unwrap(),
        // palm trees
        /*assets::load_map("world/tree/desert_palm/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/2.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/3.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/5.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/6.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/8.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/9.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        assets::load_map("world/tree/desert_palm/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 10)))
        .unwrap(),
        // snow pines
        assets::load_map("world/tree/snow_pine/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/2.vox", |s: Structure| s
            .with_center(Vec3::new(15, 15, 14)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/3.vox", |s: Structure| s
            .with_center(Vec3::new(17, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/4.vox", |s: Structure| s
            .with_center(Vec3::new(10, 8, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/5.vox", |s: Structure| s
            .with_center(Vec3::new(12, 12, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/6.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/7.vox", |s: Structure| s
            .with_center(Vec3::new(16, 15, 12)))
        .unwrap(),
        assets::load_map("world/tree/snow_pine/8.vox", |s: Structure| s
            .with_center(Vec3::new(12, 10, 12)))
        .unwrap(),
        // snow birches -> need roots!
        assets::load_map("world/tree/snow_birch/1.vox", |s: Structure| s
            .with_center(Vec3::new(12, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/3.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/4.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/5.vox", |s: Structure| s
            .with_center(Vec3::new(9, 11, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/6.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/7.vox", |s: Structure| s
            .with_center(Vec3::new(10, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/8.vox", |s: Structure| s
            .with_center(Vec3::new(9, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/9.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/10.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/11.vox", |s: Structure| s
            .with_center(Vec3::new(9, 10, 4)))
        .unwrap(),
        assets::load_map("world/tree/snow_birch/12.vox", |s: Structure| s
            .with_center(Vec3::new(10, 9, 4)))
        .unwrap(),
        // willows
        assets::load_map("world/tree/willow/1.vox", |s: Structure| s
            .with_center(Vec3::new(15, 14, 1)))
        .unwrap(),
        assets::load_map("world/tree/willow/2.vox", |s: Structure| s
            .with_center(Vec3::new(11, 12, 1)))
        .unwrap(),
        */

    ];
}
