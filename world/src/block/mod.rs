use crate::{
    column::{ColumnGen, ColumnSample},
    util::{RandomField, Sampler, SmallCache, FastNoise},
    IndexRef,
};
use common::terrain::{
    structure::{self, StructureBlock},
    Block, BlockKind, SpriteKind,
};
use core::ops::{Div, Mul, Range};
use serde::Deserialize;
use vek::*;

#[derive(Deserialize)]
pub struct Colors {
    // TODO(@Sharp): After the merge, construct enough infrastructure to make it convenient to
    // define mapping functions over the input; i.e. we should be able to interpret some fields as
    // defining App<Abs<Fun, Type>, Arg>, where Fun : (Context, Arg) → (S, Type).
    pub structure_blocks: structure::structure_block::PureCases<Option<Range<(u8, u8, u8)>>>,
}

pub struct BlockGen<'a> {
    pub column_gen: ColumnGen<'a>,
}

impl<'a> BlockGen<'a> {
    pub fn new(column_gen: ColumnGen<'a>) -> Self { Self { column_gen } }

    pub fn sample_column<'b>(
        column_gen: &ColumnGen<'a>,
        cache: &'b mut SmallCache<Option<ColumnSample<'a>>>,
        wpos: Vec2<i32>,
        index: IndexRef<'a>,
    ) -> Option<&'b ColumnSample<'a>> {
        cache
            .get(wpos, |wpos| column_gen.get((wpos, index)))
            .as_ref()
    }

    pub fn get_z_cache(&mut self, wpos: Vec2<i32>, index: IndexRef<'a>) -> Option<ZCache<'a>> {
        let BlockGen { column_gen } = self;

        // Main sample
        let sample = column_gen.get((wpos, index))?;

        Some(ZCache { sample })
    }

    pub fn get_with_z_cache(&mut self, wpos: Vec3<i32>, z_cache: Option<&ZCache>) -> Option<Block> {
        let BlockGen { column_gen } = self;
        let world = column_gen.sim;

        let z_cache = z_cache?;
        let sample = &z_cache.sample;
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
            // marble,
            // marble_small,
            rock,
            // temp,
            // humidity,
            stone_col,
            snow_cover,
            cliff_offset,
            cliff_height,
            ..
        } = sample;

        let wposf = wpos.map(|e| e as f64);

        let (_definitely_underground, height, basement_height, water_height) =
            if (wposf.z as f32) < alt - 64.0 * chaos {
                // Shortcut warping
                (true, alt, basement, water_level)
            } else {
                // Apply warping
                let warp = world
                    .gen_ctx
                    .warp_nz
                    .get(wposf.div(24.0))
                    .mul((chaos - 0.1).max(0.0).min(1.0).powi(2))
                    .mul(16.0);
                let warp = Lerp::lerp(0.0, warp, warp_factor);

                let height = alt + warp;

                (
                    false,
                    height,
                    basement + height - alt,
                    (if water_level <= alt {
                        water_level + warp
                    } else {
                        water_level
                    }),
                )
            };

        // Sample blocks

        let water = Block::new(BlockKind::Water, Rgb::zero());

        let grass_depth = (1.5 + 2.0 * chaos).min(height - basement_height);
        if (wposf.z as f32) < height - grass_depth {
            let stone_factor = (height - grass_depth - wposf.z as f32) * 0.15;
            let col = Lerp::lerp(
                sub_surface_color,
                stone_col.map(|e| e as f32 / 255.0),
                stone_factor,
            )
            .map(|e| (e * 255.0) as u8);

            if stone_factor >= 0.5 {
                if wposf.z as f32 > height - cliff_offset.max(0.0) {
                    if cliff_offset.max(0.0) > cliff_height - (FastNoise::new(37).get(wposf / Vec3::new(6.0, 6.0, 10.0)) * 0.5 + 0.5) * (height - grass_depth - wposf.z as f32).mul(0.25).clamped(0.0, 8.0) {
                        Some(Block::empty())
                    } else {
                        let col = Lerp::lerp(
                            col.map(|e| e as f32),
                            col.map(|e| e as f32) * 0.7,
                            (wposf.z as f32 - basement * 0.3).div(2.0).sin() * 0.5 + 0.5,
                        ).map(|e| e as u8);
                        Some(Block::new(BlockKind::Rock, col))
                    }
                } else {
                    Some(Block::new(BlockKind::Rock, col))
                }
            } else {
                Some(Block::new(BlockKind::Earth, col))
            }
        } else if (wposf.z as f32) < height {
            let grass_factor = (wposf.z as f32 - (height - grass_depth))
                .div(grass_depth)
                .sqrt();
            let col = Lerp::lerp(sub_surface_color, surface_color, grass_factor);
            // Surface
            Some(Block::new(
                if snow_cover {
                    //if temp < CONFIG.snow_temp + 0.031 {
                    BlockKind::Snow
                } else if grass_factor > 0.7 {
                    BlockKind::Grass
                } else {
                    BlockKind::Earth
                },
                col.map(|e| (e * 255.0) as u8),
            ))
        } else {
            None
        }
        .or_else(|| {
            // Rocks
            if (height + 2.5 - wposf.z as f32).div(7.5).abs().powi(2) < rock {
                #[allow(clippy::identity_op)]
                let field0 = RandomField::new(world.seed + 0);
                let field1 = RandomField::new(world.seed + 1);
                let field2 = RandomField::new(world.seed + 2);

                Some(Block::new(
                    BlockKind::WeakRock,
                    stone_col.map2(
                        Rgb::new(
                            field0.get(wpos) as u8 % 16,
                            field1.get(wpos) as u8 % 16,
                            field2.get(wpos) as u8 % 16,
                        ),
                        |stone, x| stone.saturating_sub(x),
                    ),
                ))
            } else {
                None
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
        })
    }
}

pub struct ZCache<'a> {
    pub sample: ColumnSample<'a>,
}

impl<'a> ZCache<'a> {
    pub fn get_z_limits(&self) -> (f32, f32) {
        let min = self.sample.alt - (self.sample.chaos.min(1.0) * 16.0) - self.sample.cliff_offset.max(0.0);
        let min = min - 4.0;

        let rocks = if self.sample.rock > 0.0 { 12.0 } else { 0.0 };

        let warp = self.sample.chaos * 32.0;

        let ground_max = self.sample.alt + warp + rocks + 2.0;

        let max = ground_max.max(self.sample.water_level + 2.0);

        (min, max)
    }
}

pub fn block_from_structure(
    index: IndexRef,
    sblock: StructureBlock,
    pos: Vec3<i32>,
    structure_pos: Vec2<i32>,
    structure_seed: u32,
    sample: &ColumnSample,
    mut with_sprite: impl FnMut(SpriteKind) -> Block,
) -> Option<Block> {
    let field = RandomField::new(structure_seed);

    let lerp = ((field.get(Vec3::from(structure_pos)).rem_euclid(256)) as f32 / 255.0) * 0.8
        + ((field.get(pos + std::i32::MAX / 2).rem_euclid(256)) as f32 / 255.0) * 0.2;

    match sblock {
        StructureBlock::None => None,
        StructureBlock::Hollow => Some(with_sprite(SpriteKind::Empty)),
        StructureBlock::Grass => Some(Block::new(
            BlockKind::Grass,
            sample.surface_color.map(|e| (e * 255.0) as u8),
        )),
        StructureBlock::Normal(color) => Some(Block::new(BlockKind::Misc, color)),
        StructureBlock::Block(kind, color) => Some(Block::new(kind, color)),
        StructureBlock::Water => Some(Block::water(SpriteKind::Empty)),
        // TODO: If/when liquid supports other colors again, revisit this.
        StructureBlock::GreenSludge => Some(Block::water(SpriteKind::Empty)),
        // None of these BlockKinds has an orientation, so we just use zero for the other color
        // bits.
        StructureBlock::Liana => Some(with_sprite(SpriteKind::Liana)),
        StructureBlock::Fruit => {
            if field.get(pos + structure_pos) % 24 == 0 {
                Some(with_sprite(SpriteKind::Beehive))
            } else if field.get(pos + structure_pos + 1) % 3 == 0 {
                Some(with_sprite(SpriteKind::Apple))
            } else {
                None
            }
        },
        StructureBlock::Coconut => {
            if field.get(pos + structure_pos) % 3 > 0 {
                None
            } else {
                Some(with_sprite(SpriteKind::Coconut))
            }
        },
        StructureBlock::Chest => {
            if structure_seed % 10 < 7 {
                Some(Block::empty())
            } else {
                Some(with_sprite(SpriteKind::Chest))
            }
        },
        StructureBlock::Log => Some(Block::new(BlockKind::Wood, Rgb::new(60, 30, 0))),
        // We interpolate all these BlockKinds as needed.
        StructureBlock::TemperateLeaves
        | StructureBlock::PineLeaves
        | StructureBlock::PalmLeavesInner
        | StructureBlock::PalmLeavesOuter
        | StructureBlock::Acacia
        | StructureBlock::Mangrove => sblock
            .elim_case_pure(&index.colors.block.structure_blocks)
            .as_ref()
            .map(|range| {
                Block::new(
                    BlockKind::Leaves,
                    Rgb::<f32>::lerp(
                        Rgb::<u8>::from(range.start).map(f32::from),
                        Rgb::<u8>::from(range.end).map(f32::from),
                        lerp,
                    )
                    .map(|e| e as u8),
                )
            }),
    }
}
