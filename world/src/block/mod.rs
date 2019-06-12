mod tree;

use std::ops::{Add, Div, Mul, Neg, Sub};
use noise::NoiseFn;
use vek::*;
use common::{
    terrain::{Block, structure::StructureBlock},
    vol::{Vox, ReadVol},
};
use crate::{
    util::{Sampler, HashCache},
    column::{ColumnGen, ColumnSample},
    CONFIG,
    World,
};

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
            forest_kind,
            close_trees,
            cave_xy,
            cave_alt,
            rock,
            cliff,
            temp,
        } = self.sample_column(Vec2::from(wpos))?;

        let wposf = wpos.map(|e| e as f64);

        // Apply warping

        let warp = (self.world.sim()
            .gen_ctx
            .warp_nz
            .get((wposf.div(Vec3::new(150.0, 150.0, 150.0))).into_array())
            as f32)
            .mul((chaos - 0.1).max(0.0))
            .mul(115.0);

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

        let stone_col = Rgb::new(200, 220, 255);
        let dirt_col = Rgb::new(79, 67, 60);

        let air = Block::empty();
        let stone = Block::new(2, stone_col);
        let surface_stone = Block::new(1, Rgb::new(200, 220, 255));
        let dirt = Block::new(1, dirt_col);
        let sand = Block::new(1, Rgb::new(180, 150, 50));
        let water = Block::new(1, Rgb::new(100, 150, 255));
        let warm_stone = Block::new(1, Rgb::new(165, 165, 130));

        let block = if (wposf.z as f32) < height - 3.0 {
            let col = Lerp::lerp(dirt_col, stone_col, (height - 4.0 - wposf.z as f32) * 0.15);

            // Underground
            if (wposf.z as f32) > alt - 32.0 * chaos {
                Some(Block::new(1, col))
            } else {
                Some(Block::new(2, col))
            }
        } else if (wposf.z as f32) < height {
            let col = Lerp::lerp(
                dirt_col.map(|e| e as f32 / 255.0),
                surface_color,
                (wposf.z as f32 - (height - 4.0)) * 0.25,
            );
            // Surface
            Some(Block::new(1, col.map(|e| (e * 255.0) as u8)))
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

        fn block_from_structure(sblock: StructureBlock, structure_pos: Vec2<i32>, sample: &ColumnSample) -> Block {
            let temp_lerp = sample.temp * 4.0;
            match sblock {
                StructureBlock::TemperateLeaves => Block::new(1, Lerp::lerp(
                        Rgb::new(0.0, 150.0, 50.0),
                        Rgb::new(200.0, 255.0, 0.0),
                        temp_lerp,
                    ).map(|e| e as u8)),
                StructureBlock::PineLeaves => Block::new(1, Lerp::lerp(
                        Rgb::new(0.0, 100.0, 90.0),
                        Rgb::new(50.0, 150.0, 50.0),
                        temp_lerp,
                    ).map(|e| e as u8)),
                StructureBlock::PalmLeaves => Block::new(1, Lerp::lerp(
                        Rgb::new(80.0, 150.0, 0.0),
                        Rgb::new(180.0, 255.0, 0.0),
                        temp_lerp,
                    ).map(|e| e as u8)),
                StructureBlock::Block(block) => block,
            }
        }

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

                            let trees = tree::kinds(tree_sample.forest_kind); // Choose tree kind

                            block.or(trees[*tree_seed as usize % trees.len()]
                                .get((rpos * 128) / 128) // Scaling
                                .map(|b| block_from_structure(*b, *tree_pos, &tree_sample))
                                .unwrap_or(Block::empty()))
                        }
                        _ => block,
                    }
                }),
        };

        Some(block)
    }
}
