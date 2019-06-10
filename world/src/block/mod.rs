mod tree;

use std::ops::{Add, Div, Mul, Neg, Sub};
use noise::NoiseFn;
use vek::*;
use common::{
    terrain::Block,
    vol::{Vox, ReadVol},
};
use crate::{
    util::{Sampler, HashCache},
    column::{ColumnGen, ColumnSample},
    CONFIG,
    World,
};
use self::tree::TREES;

pub struct BlockGen<'a> {
    world: &'a World,
    column_cache: HashCache<Vec2<i32>, Option<ColumnSample>>,
    column_gen: ColumnGen<'a>,
}

impl<'a> BlockGen<'a> {
    pub fn new(world: &'a World, column_gen: ColumnGen<'a>) -> Self {
        Self {
            world,
            column_cache: HashCache::with_capacity(1024),
            column_gen,
        }
    }

    fn sample_column(&mut self, wpos: Vec2<i32>) -> Option<ColumnSample> {
        let column_gen = &mut self.column_gen;
        self.column_cache
            .get(Vec2::from(wpos), |wpos| column_gen.get(wpos))
            .clone()
    }
}

impl<'a> Sampler for BlockGen<'a> {
    type Index = Vec3<i32>;
    type Sample = Option<Block>;

    fn get(&mut self, wpos: Vec3<i32>) -> Option<Block> {
        let ColumnSample {
            alt,
            chaos,
            surface_color,
            tree_density,
            close_trees,
            cave_xy,
            cave_alt,
            rock,
            cliff,
        } = self.sample_column(Vec2::from(wpos))?;

        let wposf = wpos.map(|e| e as f64);

        // Apply warping

        let warp = (self.world.sim()
            .gen_ctx
            .warp_nz
            .get((wposf.div(Vec3::new(150.0, 150.0, 150.0))).into_array())
            as f32)
            .mul((chaos - 0.1).max(0.0))
            .mul(130.0);

        let is_cliff = if cliff > 0.0 {
            (self.world.sim()
            .gen_ctx
            .warp_nz
            .get((wposf.div(Vec3::new(300.0, 300.0, 1500.0))).into_array())
            as f32) * cliff > 0.3
        } else {
            false
        };

        let cliff = if is_cliff {
            (0.0
            + (self.world.sim()
                .gen_ctx
                .warp_nz
                .get((wposf.div(Vec3::new(350.0, 350.0, 800.0))).into_array())
                as f32) * 0.8
            + (self.world.sim()
                .gen_ctx
                .warp_nz
                .get((wposf.div(Vec3::new(100.0, 100.0, 70.0))).into_array())
                as f32) * 0.3)
            .add(0.4)
            .mul(64.0)
        } else {
            0.0
        };

        let height = alt + warp + cliff;

        // Sample blocks

        let air = Block::empty();
        let stone = Block::new(2, Rgb::new(200, 220, 255));
        let surface_stone = Block::new(1, Rgb::new(200, 220, 255));
        let dirt = Block::new(1, Rgb::new(128, 90, 0));
        let sand = Block::new(1, Rgb::new(180, 150, 50));
        let water = Block::new(1, Rgb::new(100, 150, 255));
        let warm_stone = Block::new(1, Rgb::new(165, 165, 130));

        let block = if (wposf.z as f32) < height - 2.0 {
            // Underground
            if (wposf.z as f32) > alt {
                Some(surface_stone)
            } else {
                Some(stone)
            }
        } else if (wposf.z as f32) < height {
            // Surface
            Some(Block::new(1, surface_color.map(|e| (e * 255.0) as u8)))
        } else if (wposf.z as f32) < CONFIG.sea_level {
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
                    match self.sample_column(Vec2::from(*tree_pos)) {
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

        Some(block)
    }
}
