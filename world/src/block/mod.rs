mod natural;

use crate::{
    column::{ColumnGen, ColumnSample, StructureData},
    util::{HashCache, RandomField, Sampler, SamplerMut},
    World, CONFIG,
};
use common::{
    terrain::{structure::StructureBlock, Block, BlockKind, Structure},
    util::saturate_srgb,
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
                Some(cliff_sample) if cliff_sample.is_cliffs && cliff_sample.spawn_rate > 0.5 => {
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
            //close_structures,
            cave_xy,
            cave_alt,
            marble,
            marble_small,
            rock,
            //cliffs,
            cliff_hill,
            close_cliffs,
            //temp,
            ..
        } = &z_cache?.sample;

        let structures = &z_cache?.structures;

        let wposf = wpos.map(|e| e as f64);

        let (definitely_underground, height, water_height) =
            if (wposf.z as f32) < alt - 64.0 * chaos {
                // Shortcut warping
                (true, alt, CONFIG.sea_level /*water_level*/)
            } else {
                // Apply warping
                let warp = (world
                    .sim()
                    .gen_ctx
                    .warp_nz
                    .get((wposf.div(Vec3::new(150.0, 150.0, 150.0))).into_array())
                    as f32)
                    .mul((chaos - 0.1).max(0.0))
                    .mul(96.0);

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

        let air = Block::empty();
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
        });

        // Water
        let block = block.or_else(|| {
            if (wposf.z as f32) < water_height {
                // Ocean
                Some(water)
            } else {
                None
            }
        });

        let block = structures
            .iter()
            .find_map(|st| {
                let (st, st_sample) = st.as_ref()?;
                st.get(wpos, st_sample)
            })
            .or(block)
            .unwrap_or(Block::empty());

        Some(block)
    }
}

pub struct ZCache<'a> {
    wpos: Vec2<i32>,
    sample: ColumnSample<'a>,
    structures: [Option<(StructureInfo, ColumnSample<'a>)>; 9],
}

impl<'a> ZCache<'a> {
    pub fn get_z_limits(&self) -> (f32, f32) {
        let cave_depth = if self.sample.cave_xy.abs() > 0.9 {
            (self.sample.alt - self.sample.cave_alt + 8.0).max(0.0)
        } else {
            0.0
        };

        let min = self.sample.alt - (self.sample.chaos * 48.0 + cave_depth) - 4.0;

        let cliff = if self.sample.near_cliffs { 48.0 } else { 0.0 };
        let warp = self.sample.chaos * 48.0;
        let (structure_min, structure_max) = self
            .structures
            .iter()
            .filter_map(|st| st.as_ref())
            .fold((0.0f32, 0.0f32), |(min, max), (st_info, st_sample)| {
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

        let min = min + structure_min;
        let max = (self.sample.alt + cliff + structure_max + warp + 8.0)
            .max(self.sample.water_level)
            .max(CONFIG.sea_level + 2.0);

        (min, max)
    }
}

impl<'a> SamplerMut for BlockGen<'a> {
    type Index = Vec3<i32>;
    type Sample = Option<Block>;

    fn get(&mut self, wpos: Vec3<i32>) -> Option<Block> {
        let z_cache = self.get_z_cache(wpos.into());
        self.get_with_z_cache(wpos, z_cache.as_ref())
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

fn block_from_structure(
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
        StructureBlock::Hollow => Some(Block::empty()),
        StructureBlock::Normal(color) => {
            Some(Block::new(default_kind, color)).filter(|block| !block.is_empty())
        }
    }
}
