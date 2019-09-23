mod natural;

use crate::{
    column::{ColumnGen, ColumnSample},
    generator::{Generator, TownGen},
    util::{RandomField, Sampler, SmallCache},
    World, CONFIG,
};
use common::{
    terrain::{structure::StructureBlock, Block, BlockKind, Structure},
    util::saturate_srgb,
    vol::{ReadVol, Vox},
};
use std::ops::{Add, Div, Mul, Neg};
use vek::*;

pub struct BlockGen<'a> {
    world: &'a World,
    column_cache: SmallCache<Option<ColumnSample<'a>>>,
    column_gen: ColumnGen<'a>,
}

impl<'a> BlockGen<'a> {
    pub fn new(world: &'a World, column_gen: ColumnGen<'a>) -> Self {
        Self {
            world,
            column_cache: SmallCache::default(),
            column_gen,
        }
    }

    fn sample_column(
        column_gen: &ColumnGen<'a>,
        cache: &mut SmallCache<Option<ColumnSample<'a>>>,
        wpos: Vec2<i32>,
    ) -> Option<ColumnSample<'a>> {
        cache
            .get(Vec2::from(wpos), |wpos| column_gen.get(wpos))
            .clone()
    }

    fn get_cliff_height(
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

                    let height = RandomField::new(seed + 1).get(cliff_pos3d) % 48;
                    let radius = RandomField::new(seed + 2).get(cliff_pos3d) % 48 + 8;

                    max_height.max(
                        if cliff_pos.map(|e| e as f32).distance_squared(wpos)
                            < (radius as f32 + tolerance).powf(2.0)
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
            world: _,
            column_cache,
            column_gen,
        } = self;

        // Main sample
        let sample = column_gen.get(wpos)?;

        // Tree samples
        let mut structure_samples = [None, None, None, None, None, None, None, None, None];
        for i in 0..structure_samples.len() {
            if let Some(st) = sample.close_structures[i] {
                let st_sample = Self::sample_column(column_gen, column_cache, Vec2::from(st.pos));
                structure_samples[i] = st_sample;
            }
        }

        let mut structures = [None, None, None, None, None, None, None, None, None];
        for i in 0..structures.len() {
            if let (Some(st), Some(st_sample)) =
                (sample.close_structures[i], structure_samples[i].clone())
            {
                let st_info = match st.meta {
                    None => natural::structure_gen(
                        column_gen,
                        column_cache,
                        i,
                        st.pos,
                        st.seed,
                        &structure_samples,
                    ),
                    Some(meta) => Some(StructureInfo {
                        pos: Vec3::from(st.pos) + Vec3::unit_z() * st_sample.alt as i32,
                        seed: st.seed,
                        meta,
                    }),
                };

                if let Some(st_info) = st_info {
                    structures[i] = Some((st_info, st_sample));
                }
            }
        }

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
            world,
            column_cache,
            column_gen,
        } = self;

        let sample = &z_cache?.sample;
        let &ColumnSample {
            alt,
            chaos,
            water_level: _,
            //river,
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

            chunk,
            ..
        } = sample;

        let structures = &z_cache?.structures;

        let wposf = wpos.map(|e| e as f64);

        let (block, height) = if !only_structures {
            let (_definitely_underground, height, water_height) =
                if (wposf.z as f32) < alt - 64.0 * chaos {
                    // Shortcut warping
                    (true, alt, CONFIG.sea_level /*water_level*/)
                } else {
                    // Apply warping
                    let warp = world
                        .sim()
                        .gen_ctx
                        .warp_nz
                        .get(wposf.div(24.0))
                        .mul((chaos - 0.1).max(0.0).powf(2.0))
                        .mul(48.0);

                    let height = if (wposf.z as f32) < alt + warp - 10.0 {
                        // Shortcut cliffs
                        alt + warp
                    } else {
                        let turb = Vec2::new(
                            world.sim().gen_ctx.fast_turb_x_nz.get(wposf.div(25.0)) as f32,
                            world.sim().gen_ctx.fast_turb_y_nz.get(wposf.div(25.0)) as f32,
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

                        (alt + warp).max(cliff_height)
                    };

                    (
                        false,
                        height,
                        /*(water_level + warp).max(*/ CONFIG.sea_level, /*)*/
                    )
                };

            // Sample blocks

            // let stone_col = Rgb::new(240, 230, 220);
            let stone_col = Rgb::new(195, 187, 201);

            // let dirt_col = Rgb::new(79, 67, 60);

            let _air = Block::empty();
            // let stone = Block::new(2, stone_col);
            // let surface_stone = Block::new(1, Rgb::new(200, 220, 255));
            // let dirt = Block::new(1, dirt_col);
            // let sand = Block::new(1, Rgb::new(180, 150, 50));
            // let warm_stone = Block::new(1, Rgb::new(165, 165, 130));

            let water = Block::new(BlockKind::Water, Rgb::new(60, 90, 190));

            let grass_depth = 1.5 + 2.0 * chaos;
            let block = if (wposf.z as f32) < height - grass_depth {
                let col = Lerp::lerp(
                    saturate_srgb(sub_surface_color, 0.45).map(|e| (e * 255.0) as u8),
                    stone_col,
                    (height - grass_depth - wposf.z as f32) * 0.15,
                );

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
                    saturate_srgb(col, 0.45).map(|e| (e * 255.0) as u8),
                ))
            } else if (wposf.z as f32) < height + 0.9
                && temp < CONFIG.desert_temp
                && (wposf.z as f32 > water_height + 3.0)
                && marble > 0.68
                && marble_small > 0.65
                && (marble * 3173.7).fract() < 0.5
            {
                let flowers = [
                    BlockKind::BlueFlower,
                    BlockKind::PinkFlower,
                    BlockKind::PurpleFlower,
                    BlockKind::RedFlower,
                    BlockKind::WhiteFlower,
                    BlockKind::YellowFlower,
                    BlockKind::Sunflower,
                    BlockKind::Mushroom,
                ];

                let grasses = [
                    BlockKind::LongGrass,
                    BlockKind::MediumGrass,
                    BlockKind::ShortGrass,
                ];

                Some(Block::new(
                    if (height * 1271.0).fract() < 0.15 {
                        flowers[(height * 0.2) as usize % flowers.len()]
                    } else {
                        grasses[(height * 0.3) as usize % grasses.len()]
                    },
                    Rgb::broadcast(0),
                ))
            } else if (wposf.z as f32) < height + 0.9
                && temp > CONFIG.desert_temp
                && (marble * 4423.5).fract() < 0.0005
            {
                let large_cacti = [BlockKind::LargeCactus, BlockKind::MedFlatCactus];

                let small_cacti = [
                    BlockKind::BarrelCactus,
                    BlockKind::RoundCactus,
                    BlockKind::ShortCactus,
                    BlockKind::ShortFlatCactus,
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
                    let field0 = RandomField::new(world.sim().seed + 0);
                    let field1 = RandomField::new(world.sim().seed + 1);
                    let field2 = RandomField::new(world.sim().seed + 2);

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

                if cave {
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

        // Structures (like towns)
        let block = chunk
            .structures
            .town
            .as_ref()
            .and_then(|town| TownGen.get((town, wpos, sample, height)))
            .or(block);

        let block = structures
            .iter()
            .find_map(|st| {
                let (st, st_sample) = st.as_ref()?;
                st.get(wpos, st_sample)
            })
            .or(block);

        // Water
        let block = block.or_else(|| {
            if (wposf.z as f32) < water_height {
                // Ocean
                Some(water)
            } else {
                None
            }
        });

        Some(block)
    }
}

pub struct ZCache<'a> {
    wpos: Vec2<i32>,
    sample: ColumnSample<'a>,
    structures: [Option<(StructureInfo, ColumnSample<'a>)>; 9],
}

impl<'a> ZCache<'a> {
    pub fn get_z_limits(&self, block_gen: &mut BlockGen) -> (f32, f32, f32) {
        let cave_depth = if self.sample.cave_xy.abs() > 0.9 {
            (self.sample.alt - self.sample.cave_alt + 8.0).max(0.0)
        } else {
            0.0
        };

        let min = self.sample.alt - (self.sample.chaos * 48.0 + cave_depth) - 4.0;

        let cliff = BlockGen::get_cliff_height(
            &mut block_gen.column_gen,
            &mut block_gen.column_cache,
            self.wpos.map(|e| e as f32),
            &self.sample.close_cliffs,
            self.sample.cliff_hill,
            32.0,
        );

        let rocks = if self.sample.rock > 0.0 { 12.0 } else { 0.0 };

        let warp = self.sample.chaos * 24.0;
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

        let ground_max = (self.sample.alt + 2.0 + warp + rocks).max(cliff);

        let min = min + structure_min;
        let max = (ground_max + structure_max)
            .max(self.sample.water_level)
            .max(CONFIG.sea_level + 2.0);

        // Structures
        let (min, max) = self
            .sample
            .chunk
            .structures
            .town
            .as_ref()
            .map(|town| {
                let (town_min, town_max) = TownGen.get_z_limits(town, self.wpos, &self.sample);
                (town_min.min(min), town_max.max(max))
            })
            .unwrap_or((min, max));

        let structures_only_min_z = ground_max
            .max(self.sample.water_level)
            .max(CONFIG.sea_level + 2.0);

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
            }
            StructureMeta::Volume { units, volume } => {
                let bounds = volume.get_bounds();

                (Aabb {
                    min: Vec3::from(units.0 * bounds.min.x + units.1 * bounds.min.y)
                        + Vec3::unit_z() * bounds.min.z,
                    max: Vec3::from(units.0 * bounds.max.x + units.1 * bounds.max.y)
                        + Vec3::unit_z() * bounds.max.z,
                })
                .made_valid()
            }
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
            }
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
                            volume.default_kind(),
                            block_pos,
                            self.pos.into(),
                            self.seed,
                            sample,
                        )
                    })
            }
        }
    }
}

pub fn block_from_structure(
    sblock: StructureBlock,
    default_kind: BlockKind,
    pos: Vec3<i32>,
    structure_pos: Vec2<i32>,
    structure_seed: u32,
    _sample: &ColumnSample,
) -> Option<Block> {
    let field = RandomField::new(structure_seed + 0);

    let lerp = 0.5
        + ((field.get(Vec3::from(structure_pos)) % 256) as f32 / 256.0 - 0.5) * 0.65
        + ((field.get(Vec3::from(pos)) % 256) as f32 / 256.0 - 0.5) * 0.15;

    match sblock {
        StructureBlock::None => None,
        StructureBlock::TemperateLeaves => Some(Block::new(
            BlockKind::Normal,
            Lerp::lerp(
                Rgb::new(0.0, 132.0, 94.0),
                Rgb::new(142.0, 181.0, 0.0),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::PineLeaves => Some(Block::new(
            BlockKind::Normal,
            Lerp::lerp(Rgb::new(0.0, 60.0, 50.0), Rgb::new(30.0, 100.0, 10.0), lerp)
                .map(|e| e as u8),
        )),
        StructureBlock::PalmLeaves => Some(Block::new(
            BlockKind::Normal,
            Lerp::lerp(
                Rgb::new(0.0, 108.0, 113.0),
                Rgb::new(30.0, 156.0, 10.0),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::Water => Some(Block::new(BlockKind::Water, Rgb::new(100, 150, 255))),
        StructureBlock::GreenSludge => Some(Block::new(BlockKind::Water, Rgb::new(30, 126, 23))),
        StructureBlock::Acacia => Some(Block::new(
            BlockKind::Normal,
            Lerp::lerp(
                Rgb::new(15.0, 126.0, 50.0),
                Rgb::new(30.0, 180.0, 10.0),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::Fruit => Some(Block::new(BlockKind::Apple, Rgb::new(194, 30, 37))),
        StructureBlock::Liana => Some(Block::new(
            BlockKind::Liana,
            Lerp::lerp(
                Rgb::new(0.0, 125.0, 107.0),
                Rgb::new(0.0, 155.0, 129.0),
                lerp,
            )
            .map(|e| e as u8),
        )),
        StructureBlock::Mangrove => Some(Block::new(
            BlockKind::Normal,
            Lerp::lerp(Rgb::new(32.0, 56.0, 22.0), Rgb::new(57.0, 69.0, 27.0), lerp)
                .map(|e| e as u8),
        )),
        StructureBlock::Hollow => Some(Block::empty()),
        StructureBlock::Normal(color) => {
            Some(Block::new(default_kind, color)).filter(|block| !block.is_empty())
        }
    }
}
