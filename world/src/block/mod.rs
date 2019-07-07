mod tree;

use crate::{
    column::{ColumnGen, ColumnSample},
    util::{HashCache, RandomField, Sampler, SamplerMut},
    World,
};
use common::{
    terrain::{structure::StructureBlock, Block},
    vol::{ReadVol, Vox},
};
use noise::NoiseFn;
use std::ops::{Add, Div, Mul, Neg};
use vek::*;

pub struct BlockGen<'a> {
    world: &'a World,
    column_cache: HashCache<Vec2<i32>, Option<ColumnSample<'a>>>,
    column_gen: ColumnGen<'a>,
}

impl<'a> BlockGen<'a> {
    pub fn new(world: &'a World, column_gen: ColumnGen<'a>) -> Self {
        Self {
            world,
            column_cache: HashCache::with_capacity(64),
            column_gen,
        }
    }

    fn sample_column(
        column_gen: &ColumnGen<'a>,
        cache: &mut HashCache<Vec2<i32>, Option<ColumnSample<'a>>>,
        wpos: Vec2<i32>,
    ) -> Option<ColumnSample<'a>> {
        cache
            .get(Vec2::from(wpos), |wpos| column_gen.get(wpos))
            .clone()
    }

    fn get_cliff_height(
        column_gen: &ColumnGen<'a>,
        cache: &mut HashCache<Vec2<i32>, Option<ColumnSample<'a>>>,
        wpos: Vec2<f32>,
        close_cliffs: &[(Vec2<i32>, u32); 9],
        cliff_hill: f32,
    ) -> f32 {
        close_cliffs.iter().fold(
            0.0f32,
            |max_height, (cliff_pos, seed)| match Self::sample_column(
                column_gen,
                cache,
                Vec2::from(*cliff_pos),
            ) {
                Some(cliff_sample) if cliff_sample.cliffs && cliff_sample.spawn_rate > 0.5 => {
                    let cliff_pos3d = Vec3::from(*cliff_pos);

                    let height = RandomField::new(seed + 1).get(cliff_pos3d) % 48;
                    let radius = RandomField::new(seed + 2).get(cliff_pos3d) % 48 + 8;

                    max_height.max(
                        if cliff_pos.map(|e| e as f32).distance_squared(wpos)
                            < (radius * radius) as f32
                        {
                            cliff_sample.alt
                                + height as f32 * (1.0 - cliff_sample.chaos)
                                + cliff_hill
                        } else {
                            0.0
                        },
                    )
                }
                _ => max_height,
            },
        )
    }

    pub fn get_z_cache(&mut self, wpos: Vec2<i32>) -> Option<ZCache<'a>> {
        let BlockGen {
            world,
            column_cache,
            column_gen,
        } = self;

        // Main sample
        let sample = Self::sample_column(column_gen, column_cache, wpos)?;

        // Tree samples
        let mut tree_samples = [None, None, None, None, None, None, None, None, None];
        for i in 0..tree_samples.len() {
            tree_samples[i] = Self::sample_column(
                column_gen,
                column_cache,
                Vec2::from(sample.close_trees[i].0),
            );
        }

        Some(ZCache {
            sample,
            tree_samples,
        })
    }

    pub fn get_with_z_cache(&mut self, wpos: Vec3<i32>, z_cache: Option<&ZCache>) -> Option<Block> {
        let BlockGen {
            world,
            column_cache,
            column_gen,
        } = self;

        let &ColumnSample {
            alt,
            chaos,
            water_level,
            //river,
            surface_color,
            sub_surface_color,
            //tree_density,
            //forest_kind,
            close_trees,
            cave_xy,
            cave_alt,
            rock,
            //cliffs,
            cliff_hill,
            close_cliffs,
            //temp,
            ..
        } = &z_cache?.sample;

        let tree_samples = &z_cache?.tree_samples;

        let wpos2d = Vec2::from(wpos);
        let wposf = wpos.map(|e| e as f64);

        let (definitely_underground, height, water_height) =
            if (wposf.z as f32) < alt - 64.0 * chaos {
                // Shortcut warping
                (true, alt, water_level)
            } else {
                // Apply warping
                let warp = (world
                    .sim()
                    .gen_ctx
                    .warp_nz
                    .get((wposf.div(Vec3::new(150.0, 150.0, 150.0))).into_array())
                    as f32)
                    .mul((chaos - 0.1).max(0.0))
                    .mul(115.0);

                let height = if (wposf.z as f32) < alt + warp - 10.0 {
                    // Shortcut cliffs
                    alt + warp
                } else {
                    let turb = Vec2::new(
                        world
                            .sim()
                            .gen_ctx
                            .turb_x_nz
                            .get((wposf.div(48.0)).into_array()) as f32,
                        world
                            .sim()
                            .gen_ctx
                            .turb_y_nz
                            .get((wposf.div(48.0)).into_array()) as f32,
                    ) * 12.0;

                    let wpos_turb = Vec2::from(wpos).map(|e: i32| e as f32) + turb;
                    let cliff_height = Self::get_cliff_height(
                        column_gen,
                        column_cache,
                        wpos_turb,
                        &close_cliffs,
                        cliff_hill,
                    );

                    (alt + warp).max(cliff_height)
                };

                (false, height, water_level + warp)
            };

        // Sample blocks

        let stone_col = Rgb::new(200, 220, 255);
        // let dirt_col = Rgb::new(79, 67, 60);

        let air = Block::empty();
        // let stone = Block::new(2, stone_col);
        // let surface_stone = Block::new(1, Rgb::new(200, 220, 255));
        // let dirt = Block::new(1, dirt_col);
        // let sand = Block::new(1, Rgb::new(180, 150, 50));
        // let warm_stone = Block::new(1, Rgb::new(165, 165, 130));
        let water = Block::new(1, Rgb::new(100, 150, 255));

        let grass_depth = 2.0;
        let block = if (wposf.z as f32) < height - grass_depth {
            let col = Lerp::lerp(
                sub_surface_color.map(|e| (e * 255.0) as u8),
                stone_col,
                (height - grass_depth - wposf.z as f32) * 0.15,
            );

            // Underground
            if (wposf.z as f32) > alt - 32.0 * chaos {
                Some(Block::new(1, col))
            } else {
                Some(Block::new(2, col))
            }
        } else if (wposf.z as f32) < height {
            let col = Lerp::lerp(
                sub_surface_color,
                surface_color,
                (wposf.z as f32 - (height - grass_depth))
                    .div(grass_depth)
                    .powf(0.5),
            );
            // Surface
            Some(Block::new(1, col.map(|e| (e * 255.0) as u8)))
        } else if (wposf.z as f32) < water_height {
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
                let field0 = RandomField::new(world.sim().seed + 0);
                let field1 = RandomField::new(world.sim().seed + 1);
                let field2 = RandomField::new(world.sim().seed + 2);

                Some(Block::new(
                    1,
                    stone_col
                        - Rgb::new(
                            field0.get(wpos) as u8 % 32,
                            field1.get(wpos) as u8 % 32,
                            field2.get(wpos) as u8 % 32,
                        ),
                ))
            } else {
                None
            }
        });

        fn block_from_structure(
            sblock: StructureBlock,
            pos: Vec3<i32>,
            structure_pos: Vec2<i32>,
            structure_seed: u32,
            _sample: &ColumnSample,
        ) -> Block {
            let field = RandomField::new(structure_seed + 0);

            let lerp = 0.5
                + ((field.get(Vec3::from(structure_pos)) % 256) as f32 / 256.0 - 0.5) * 0.75
                + ((field.get(Vec3::from(pos)) % 256) as f32 / 256.0 - 0.5) * 0.2;

            match sblock {
                StructureBlock::TemperateLeaves => Block::new(
                    1,
                    Lerp::lerp(
                        Rgb::new(0.0, 80.0, 40.0),
                        Rgb::new(120.0, 255.0, 10.0),
                        lerp,
                    )
                    .map(|e| e as u8),
                ),
                StructureBlock::PineLeaves => Block::new(
                    1,
                    Lerp::lerp(Rgb::new(0.0, 60.0, 50.0), Rgb::new(30.0, 100.0, 10.0), lerp)
                        .map(|e| e as u8),
                ),
                StructureBlock::PalmLeaves => Block::new(
                    1,
                    Lerp::lerp(
                        Rgb::new(25.0, 100.0, 30.0),
                        Rgb::new(75.0, 255.0, 0.0),
                        lerp,
                    )
                    .map(|e| e as u8),
                ),
                StructureBlock::Block(block) => block,
            }
        }

        let block = if definitely_underground {
            block.unwrap_or(Block::empty())
        } else {
            match block {
                Some(block) => block,
                None => (&close_trees).iter().enumerate().fold(
                    air,
                    |block, (tree_idx, (tree_pos, tree_seed))| {
                        if !block.is_empty() {
                            block
                        } else {
                            match &tree_samples[tree_idx] {
                                Some(tree_sample)
                                    if wpos2d.distance_squared(*tree_pos) < 28 * 28
                                        && tree_sample.tree_density
                                            > 0.5 + (*tree_seed as f32 / 1000.0).fract() * 0.2
                                        && tree_sample.alt > tree_sample.water_level
                                        && tree_sample.spawn_rate > 0.5 =>
                                {
                                    let cliff_height = Self::get_cliff_height(
                                        column_gen,
                                        column_cache,
                                        tree_pos.map(|e| e as f32),
                                        &tree_sample.close_cliffs,
                                        cliff_hill,
                                    );
                                    let height = tree_sample.alt.max(cliff_height);
                                    let tree_pos3d =
                                        Vec3::new(tree_pos.x, tree_pos.y, height as i32);
                                    let rpos = wpos - tree_pos3d;

                                    let trees = tree::kinds(tree_sample.forest_kind); // Choose tree kind

                                    block.or(trees[*tree_seed as usize % trees.len()]
                                        .get((rpos * 128) / 128) // Scaling
                                        .map(|b| {
                                            block_from_structure(
                                                *b,
                                                rpos,
                                                *tree_pos,
                                                *tree_seed,
                                                &tree_sample,
                                            )
                                        })
                                        .unwrap_or(Block::empty()))
                                }
                                _ => block,
                            }
                        }
                    },
                ),
            }
        };

        Some(block)
    }
}

pub struct ZCache<'a> {
    sample: ColumnSample<'a>,
    tree_samples: [Option<ColumnSample<'a>>; 9],
}

impl<'a> SamplerMut for BlockGen<'a> {
    type Index = Vec3<i32>;
    type Sample = Option<Block>;

    fn get(&mut self, wpos: Vec3<i32>) -> Option<Block> {
        let z_cache = self.get_z_cache(wpos.into());
        self.get_with_z_cache(wpos, z_cache.as_ref())
    }
}
