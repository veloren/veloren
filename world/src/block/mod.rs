mod natural;

use crate::{
    column::{ColumnGen, ColumnSample},
    util::{RandomField, Sampler, SmallCache},
    CONFIG,
};
use common::{
    terrain::{structure::StructureBlock, Block, BlockKind, Structure},
    vol::{ReadVol, Vox},
};
use std::ops::{Add, Div, Mul, Neg};
use vek::*;

pub struct BlockGen<'a> {
    pub column_cache: SmallCache<Option<ColumnSample<'a>>>,
    pub column_gen: ColumnGen<'a>,
}

impl<'a> BlockGen<'a> {
    pub fn new(column_gen: ColumnGen<'a>) -> Self {
        Self {
            column_cache: SmallCache::default(),
            column_gen,
        }
    }

    pub fn sample_column<'b>(
        column_gen: &ColumnGen<'a>,
        cache: &'b mut SmallCache<Option<ColumnSample<'a>>>,
        wpos: Vec2<i32>,
    ) -> Option<&'b ColumnSample<'a>> {
        cache
            .get(Vec2::from(wpos), |wpos| column_gen.get(wpos))
            .as_ref()
    }

    pub fn get_cliff_height(
        column_gen: &ColumnGen<'a>,
        cache: &mut SmallCache<Option<ColumnSample<'a>>>,
        wpos: Vec2<f32>,
        close_cliffs: &[(Vec2<i32>, u32); 9],
        cliff_hill: f32,
        tolerance: f32,
    ) -> f32 {
        close_cliffs.iter().fold(
            0.0f32,
            |max_height, (cliff_pos, seed)| match Self::sample_column(
                column_gen,
                cache,
                Vec2::from(*cliff_pos),
            ) {
                Some(cliff_sample) if cliff_sample.is_cliffs && cliff_sample.spawn_rate > 0.5 => {
                    let cliff_pos3d = Vec3::from(*cliff_pos);

                    // Conservative range of height: [15.70, 49.33]
                    let height = (RandomField::new(seed + 1).get(cliff_pos3d) % 64) as f32
                        // [0, 63] / (1 + 3 * [0.12, 1.32]) + 3 =
                        // [0, 63] / (1 + [0.36, 3.96]) + 3 =
                        // [0, 63] / [1.36, 4.96] + 3 =
                        // [0, 63] / [1.36, 4.96] + 3 =
                        // (height min) [0, 0] + 3 = [3, 3]
                        // (height max) [12.70, 46.33] + 3 = [15.70, 49.33]
                        / (1.0 + 3.0 * cliff_sample.chaos)
                        + 3.0;
                    // Conservative range of radius: [8, 47]
                    let radius = RandomField::new(seed + 2).get(cliff_pos3d) % 48 + 8;

                    if cliff_sample
                        .water_dist
                        .map(|d| d > radius as f32)
                        .unwrap_or(true)
                    {
                        max_height.max(
                            if cliff_pos.map(|e| e as f32).distance_squared(wpos)
                                < (radius as f32 + tolerance).powf(2.0)
                            {
                                cliff_sample.alt + height * (1.0 - cliff_sample.chaos) + cliff_hill
                            } else {
                                0.0
                            },
                        )
                    } else {
                        max_height
                    }
                },
                _ => max_height,
            },
        )
    }

    pub fn get_z_cache(&mut self, wpos: Vec2<i32>) -> Option<ZCache<'a>> {
        let BlockGen {
            column_cache,
            column_gen,
        } = self;

        // Main sample
        let sample = column_gen.get(wpos)?;

        // Tree samples
        let mut structures = [None, None, None, None, None, None, None, None, None];
        sample
            .close_structures
            .iter()
            .zip(structures.iter_mut())
            .for_each(|(close_structure, structure)| {
                if let Some(st) = *close_structure {
                    let st_sample =
                        Self::sample_column(column_gen, column_cache, Vec2::from(st.pos));
                    if let Some(st_sample) = st_sample {
                        let st_sample = st_sample.clone();
                        let st_info = match st.meta {
                            None => natural::structure_gen(
                                column_gen,
                                column_cache,
                                st.pos,
                                st.seed,
                                &st_sample,
                            ),
                            Some(meta) => Some(StructureInfo {
                                pos: Vec3::from(st.pos) + Vec3::unit_z() * st_sample.alt as i32,
                                seed: st.seed,
                                meta,
                            }),
                        };
                        if let Some(st_info) = st_info {
                            *structure = Some((st_info, st_sample));
                        }
                    }
                }
            });

        Some(ZCache {
            wpos,
            sample,
            structures,
        })
    }

    pub fn get_with_z_cache(
        &mut self,
        wpos: Vec3<i32>,
        z_cache: Option<&ZCache>,
        only_structures: bool,
    ) -> Option<Block> {
        let BlockGen {
            column_cache,
            column_gen,
        } = self;
        let world = column_gen.sim;

        let sample = &z_cache?.sample;
        let &ColumnSample {
            alt,
            basement,
            chaos,
            water_level,
            warp_factor,
            surface_color,
            sub_surface_color,
            //tree_density,
            //forest_kind,
            //close_structures,
            cave_xy,
            cave_alt,
            marble,
            marble_small,
            rock,
            //cliffs,
            cliff_hill,
            close_cliffs,
            temp,
            humidity,
            stone_col,
            ..
        } = sample;

        let structures = &z_cache?.structures;

        let wposf = wpos.map(|e| e as f64);

        let (block, _height) = if !only_structures {
            let (_definitely_underground, height, on_cliff, basement_height, water_height) =
                if (wposf.z as f32) < alt - 64.0 * chaos {
                    // Shortcut warping
                    (true, alt, false, basement, water_level)
                } else {
                    // Apply warping
                    let warp = world
                        .gen_ctx
                        .warp_nz
                        .get(wposf.div(24.0))
                        .mul((chaos - 0.1).max(0.0).min(1.0).powf(2.0))
                        .mul(16.0);
                    let warp = Lerp::lerp(0.0, warp, warp_factor);

                    let surface_height = alt + warp;

                    let (height, on_cliff) = if (wposf.z as f32) < alt + warp - 10.0 {
                        // Shortcut cliffs
                        (surface_height, false)
                    } else {
                        let turb = Vec2::new(
                            world.gen_ctx.fast_turb_x_nz.get(wposf.div(25.0)) as f32,
                            world.gen_ctx.fast_turb_y_nz.get(wposf.div(25.0)) as f32,
                        ) * 8.0;

                        let wpos_turb = Vec2::from(wpos).map(|e: i32| e as f32) + turb;
                        let cliff_height = Self::get_cliff_height(
                            column_gen,
                            column_cache,
                            wpos_turb,
                            &close_cliffs,
                            cliff_hill,
                            0.0,
                        );

                        (
                            surface_height.max(cliff_height),
                            cliff_height > surface_height + 16.0,
                        )
                    };

                    (
                        false,
                        height,
                        on_cliff,
                        basement + height - alt,
                        (if water_level <= alt {
                            water_level + warp
                        } else {
                            water_level
                        }),
                    )
                };

            // Sample blocks

            // let stone_col = Rgb::new(195, 187, 201);

            // let dirt_col = Rgb::new(79, 67, 60);

            let _air = Block::empty();
            // let stone = Block::new(2, stone_col);
            // let surface_stone = Block::new(1, Rgb::new(200, 220, 255));
            // let dirt = Block::new(1, dirt_col);
            // let sand = Block::new(1, Rgb::new(180, 150, 50));
            // let warm_stone = Block::new(1, Rgb::new(165, 165, 130));

            let water = Block::new(BlockKind::Water, Rgb::new(60, 90, 190));

            let grass_depth = (1.5 + 2.0 * chaos).min(height - basement_height);
            let block = if (wposf.z as f32) < height - grass_depth {
                let col = Lerp::lerp(
                    sub_surface_color,
                    stone_col.map(|e| e as f32 / 255.0),
                    (height - grass_depth - wposf.z as f32) * 0.15,
                )
                .map(|e| (e * 255.0) as u8);

                // Underground
                if (wposf.z as f32) > alt - 32.0 * chaos {
                    Some(Block::new(BlockKind::Normal, col))
                } else {
                    Some(Block::new(BlockKind::Dense, col))
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
                Some(Block::new(
                    BlockKind::Normal,
                    col.map(|e| (e * 255.0) as u8),
                ))
            } else if (wposf.z as f32) < height + 0.9
                && temp < CONFIG.desert_temp
                && (wposf.z as f32 > water_height + 3.0)
                && marble > 0.6
                && marble_small > 0.55
                && (marble * 3173.7).fract() < 0.6
                && humidity > CONFIG.desert_hum
            {
                let treasures = [BlockKind::Chest, BlockKind::Velorite];

                let flowers = [
                    BlockKind::BlueFlower,
                    BlockKind::PinkFlower,
                    BlockKind::PurpleFlower,
                    BlockKind::RedFlower,
                    BlockKind::WhiteFlower,
                    BlockKind::YellowFlower,
                    BlockKind::Sunflower,
                    BlockKind::Mushroom,
                    BlockKind::LeafyPlant,
                    BlockKind::Blueberry,
                    BlockKind::LingonBerry,
                    BlockKind::Fern,
                ];
                let grasses = [
                    BlockKind::LongGrass,
                    BlockKind::MediumGrass,
                    BlockKind::ShortGrass,
                ];

                Some(Block::new(
                    if on_cliff && (height * 1271.0).fract() < 0.015 {
                        treasures[(height * 731.3) as usize % treasures.len()]
                    } else if (height * 1271.0).fract() < 0.1 {
                        flowers[(height * 0.2) as usize % flowers.len()]
                    } else {
                        grasses[(height * 103.3) as usize % grasses.len()]
                    },
                    Rgb::broadcast(0),
                ))
            } else if (wposf.z as f32) < height + 0.9
                && temp > CONFIG.desert_temp
                && (marble * 4423.5).fract() < 0.0005
            {
                let large_cacti = [
                    BlockKind::LargeCactus,
                    BlockKind::MedFlatCactus,
                    BlockKind::Welwitch,
                ];

                let small_cacti = [
                    BlockKind::BarrelCactus,
                    BlockKind::RoundCactus,
                    BlockKind::ShortCactus,
                    BlockKind::ShortFlatCactus,
                    BlockKind::DeadBush,
                ];

                Some(Block::new(
                    if (height * 1271.0).fract() < 0.5 {
                        large_cacti[(height * 0.2) as usize % large_cacti.len()]
                    } else {
                        small_cacti[(height * 0.3) as usize % small_cacti.len()]
                    },
                    Rgb::broadcast(0),
                ))
            } else {
                None
            }
            .or_else(|| {
                // Rocks
                if (height + 2.5 - wposf.z as f32).div(7.5).abs().powf(2.0) < rock {
                    let field0 = RandomField::new(world.seed + 0);
                    let field1 = RandomField::new(world.seed + 1);
                    let field2 = RandomField::new(world.seed + 2);

                    Some(Block::new(
                        BlockKind::Normal,
                        stone_col
                            - Rgb::new(
                                field0.get(wpos) as u8 % 16,
                                field1.get(wpos) as u8 % 16,
                                field2.get(wpos) as u8 % 16,
                            ),
                    ))
                } else {
                    None
                }
            })
            .and_then(|block| {
                // Caves
                // Underground
                let cave = cave_xy.powf(2.0)
                    * (wposf.z as f32 - cave_alt)
                        .div(40.0)
                        .powf(4.0)
                        .neg()
                        .add(1.0)
                    > 0.9993;

                if cave && wposf.z as f32 > water_height + 3.0 {
                    None
                } else {
                    Some(block)
                }
            })
            .or_else(|| {
                // Water
                if (wposf.z as f32) < water_height {
                    // Ocean
                    Some(water)
                } else {
                    None
                }
            });

            (block, height)
        } else {
            (None, sample.alt)
        };

        let block = structures
            .iter()
            .find_map(|st| {
                let (st, st_sample) = st.as_ref()?;
                st.get(wpos, st_sample)
            })
            .or(block);

        Some(block.unwrap_or(Block::empty()))
    }
}

pub struct ZCache<'a> {
    wpos: Vec2<i32>,
    pub sample: ColumnSample<'a>,
    structures: [Option<(StructureInfo, ColumnSample<'a>)>; 9],
}

impl<'a> ZCache<'a> {
    pub fn get_z_limits(&self, block_gen: &mut BlockGen) -> (f32, f32, f32) {
        let cave_depth =
            if self.sample.cave_xy.abs() > 0.9 && self.sample.water_level <= self.sample.alt {
                (self.sample.alt - self.sample.cave_alt + 8.0).max(0.0)
            } else {
                0.0
            };

        let min = self.sample.alt - (self.sample.chaos.min(1.0) * 16.0 + cave_depth);
        let min = min - 4.0;

        let cliff = BlockGen::get_cliff_height(
            &mut block_gen.column_gen,
            &mut block_gen.column_cache,
            self.wpos.map(|e| e as f32),
            &self.sample.close_cliffs,
            self.sample.cliff_hill,
            32.0,
        );

        let rocks = if self.sample.rock > 0.0 { 12.0 } else { 0.0 };

        let warp = self.sample.chaos * 32.0;

        let (structure_min, structure_max) = self
            .structures
            .iter()
            .filter_map(|st| st.as_ref())
            .fold((0.0f32, 0.0f32), |(min, max), (st_info, _st_sample)| {
                let bounds = st_info.get_bounds();
                let st_area = Aabr {
                    min: Vec2::from(bounds.min),
                    max: Vec2::from(bounds.max),
                };

                if st_area.contains_point(self.wpos - st_info.pos) {
                    (min.min(bounds.min.z as f32), max.max(bounds.max.z as f32))
                } else {
                    (min, max)
                }
            });

        let ground_max = (self.sample.alt + warp + rocks).max(cliff) + 2.0;

        let min = min + structure_min;
        let max = (ground_max + structure_max).max(self.sample.water_level + 2.0);

        let structures_only_min_z = ground_max.max(self.sample.water_level + 2.0);

        (min, structures_only_min_z, max)
    }
}

#[derive(Copy, Clone)]
pub enum StructureMeta {
    Pyramid {
        height: i32,
    },
    Volume {
        units: (Vec2<i32>, Vec2<i32>),
        volume: &'static Structure,
    },
}

pub struct StructureInfo {
    pos: Vec3<i32>,
    seed: u32,
    meta: StructureMeta,
}

impl StructureInfo {
    fn get_bounds(&self) -> Aabb<i32> {
        match self.meta {
            StructureMeta::Pyramid { height } => {
                let base = 40;
                Aabb {
                    min: Vec3::new(-base - height, -base - height, -base),
                    max: Vec3::new(base + height, base + height, height),
                }
            },
            StructureMeta::Volume { units, volume } => {
                let bounds = volume.get_bounds();

                (Aabb {
                    min: Vec3::from(units.0 * bounds.min.x + units.1 * bounds.min.y)
                        + Vec3::unit_z() * bounds.min.z,
                    max: Vec3::from(units.0 * bounds.max.x + units.1 * bounds.max.y)
                        + Vec3::unit_z() * bounds.max.z,
                })
                .made_valid()
            },
        }
    }

    fn get(&self, wpos: Vec3<i32>, sample: &ColumnSample) -> Option<Block> {
        match self.meta {
            StructureMeta::Pyramid { height } => {
                if wpos.z - self.pos.z
                    < height
                        - Vec2::from(wpos - self.pos)
                            .map(|e: i32| (e.abs() / 2) * 2)
                            .reduce_max()
                {
                    Some(Block::new(BlockKind::Dense, Rgb::new(203, 170, 146)))
                } else {
                    None
                }
            },
            StructureMeta::Volume { units, volume } => {
                let rpos = wpos - self.pos;
                let block_pos = Vec3::unit_z() * rpos.z
                    + Vec3::from(units.0) * rpos.x
                    + Vec3::from(units.1) * rpos.y;

                volume
                    .get((block_pos * 128) / 128) // Scaling
                    .ok()
                    .and_then(|b| {
                        block_from_structure(
                            *b,
                            block_pos,
                            self.pos.into(),
                            self.seed,
                            sample,
                        )
                    })
            },
        }
    }
}

pub fn block_from_structure(
    sblock: StructureBlock,
    pos: Vec3<i32>,
    structure_pos: Vec2<i32>,
    structure_seed: u32,
    sample: &ColumnSample,
) -> Option<Block> {
    let field = RandomField::new(structure_seed + 0);

    let lerp = ((field.get(Vec3::from(structure_pos)).rem_euclid(256)) as f32 / 255.0) * 0.85
        + ((field.get(pos + std::i32::MAX / 2).rem_euclid(256)) as f32 / 255.0) * 0.15;

    let saturate_leaves = |col: Rgb<f32>| {
        // /*saturate_srgb(col / 255.0, 0.65)*/
        /* let rgb = srgb_to_linear(col / 255.0);
        /* let mut xyy = rgb_to_xyy(rgb);
        xyy.x *= xyy.x;
        xyy.y *= xyy.y;
        linear_to_srgb(xyy_to_rgb(xyy).map(|e| e.min(1.0).max(0.0))).map(|e| e * 255.0) */
        /* let xyz = rgb_to_xyz(rgb);
        let col_adjusted = if xyz.y == 0.0 {
            Rgb::zero()
        } else {
            rgb / xyz.y
        };
        let col = col_adjusted * col_adjusted * xyz.y;
        linear_to_srgb(col).map(|e| e * 255.0) */
        /* let mut hsv = rgb_to_hsv(rgb);
        hsv.y *= hsv.y;
        linear_to_srgb(hsv_to_rgb(hsv).map(|e| e.min(1.0).max(0.0))).map(|e| e * 255.0) */
        linear_to_srgb(rgb * rgb).map(|e| e * 255.0) */
        col
    };

    match sblock {
        StructureBlock::None => None,
        StructureBlock::Grass => Some(Block::new(
            BlockKind::Normal,
            sample.surface_color.map(|e| (e * 255.0) as u8),
        )),
        StructureBlock::TemperateLeaves => Some(Block::new(
            BlockKind::Leaves,
            Lerp::lerp(
                saturate_leaves(Rgb::new(0.0, 132.0, 94.0)),
                saturate_leaves(Rgb::new(142.0, 181.0, 0.0)),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::PineLeaves => Some(Block::new(
            BlockKind::Leaves,
            Lerp::lerp(Rgb::new(0.0, 60.0, 50.0), Rgb::new(30.0, 100.0, 10.0), lerp)
                .map(|e| e as u8),
        )),
        StructureBlock::PalmLeavesInner => Some(Block::new(
            BlockKind::Leaves,
            Lerp::lerp(
                Rgb::new(61.0, 166.0, 43.0),
                Rgb::new(29.0, 130.0, 32.0),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::PalmLeavesOuter => Some(Block::new(
            BlockKind::Leaves,
            Lerp::lerp(
                Rgb::new(62.0, 171.0, 38.0),
                Rgb::new(45.0, 171.0, 65.0),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::Water => Some(Block::new(
            BlockKind::Water,
            saturate_leaves(Rgb::new(100.0, 150.0, 255.0)).map(|e| e as u8),
        )),
        StructureBlock::GreenSludge => Some(Block::new(
            BlockKind::Water,
            saturate_leaves(Rgb::new(30.0, 126.0, 23.0)).map(|e| e as u8),
        )),
        StructureBlock::Acacia => Some(Block::new(
            BlockKind::Normal,
            Lerp::lerp(
                Rgb::new(15.0, 126.0, 50.0),
                Rgb::new(30.0, 180.0, 10.0),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::Fruit => Some(if field.get(pos + structure_pos) % 3 > 0 {
            Block::empty()
        } else {
            Block::new(BlockKind::Apple, Rgb::new(1, 1, 1))
        }),
        StructureBlock::Coconut => Some(if field.get(pos + structure_pos) % 3 > 0 {
            Block::empty()
        } else {
            Block::new(BlockKind::Coconut, Rgb::new(1, 1, 1))
        }),
        StructureBlock::Chest => Some(if structure_seed % 10 < 7 {
            Block::empty()
        } else {
            Block::new(BlockKind::Chest, Rgb::new(1, 1, 1))
        }),
        StructureBlock::Liana => Some(Block::new(
            BlockKind::Liana,
            Lerp::lerp(
                saturate_leaves(Rgb::new(0.0, 125.0, 107.0)),
                saturate_leaves(Rgb::new(0.0, 155.0, 129.0)),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::Mangrove => Some(Block::new(
            BlockKind::Normal,
            Lerp::lerp(
                saturate_leaves(Rgb::new(32.0, 56.0, 22.0)),
                saturate_leaves(Rgb::new(57.0, 69.0, 27.0)),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::Hollow => Some(Block::empty()),
        StructureBlock::Normal(color) => {
            Some(Block::new(BlockKind::Normal, color)).filter(|block| !block.is_empty())
        },
    }
}
