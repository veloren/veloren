use crate::{
    column::{ColumnGen, ColumnSample},
    util::{FastNoise, RandomField, RandomPerm, Sampler, SmallCache},
    IndexRef, CONFIG,
};
use common::{
    calendar::{Calendar, CalendarEvent},
    comp::item::ItemDefinitionIdOwned,
    terrain::{
        structure::{self, StructureBlock},
        Block, BlockKind, SpriteCfg, SpriteKind, UnlockKind,
    },
};
use core::ops::{Div, Mul, Range};
use serde::Deserialize;
use vek::*;

type Gradients = Vec<Range<(u8, u8, u8)>>;

#[derive(Deserialize)]
pub struct Colors {
    // TODO(@Sharp): After the merge, construct enough infrastructure to make it convenient to
    // define mapping functions over the input; i.e. we should be able to interpret some fields as
    // defining App<Abs<Fun, Type>, Arg>, where Fun : (Context, Arg) → (S, Type).
    pub structure_blocks: structure::structure_block::PureCases<Option<Gradients>>,
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
        calendar: Option<&'a Calendar>,
    ) -> Option<&'b ColumnSample<'a>> {
        cache
            .get(wpos, |wpos| column_gen.get((wpos, index, calendar)))
            .as_ref()
    }

    pub fn get_z_cache(
        &mut self,
        wpos: Vec2<i32>,
        index: IndexRef<'a>,
        calendar: Option<&'a Calendar>,
    ) -> Option<ZCache<'a>> {
        let BlockGen { column_gen } = self;

        // Main sample
        let sample = column_gen.get((wpos, index, calendar))?;

        Some(ZCache { sample, calendar })
    }

    pub fn get_with_z_cache(&mut self, wpos: Vec3<i32>, z_cache: Option<&ZCache>) -> Option<Block> {
        let z_cache = z_cache?;
        let sample = &z_cache.sample;
        let &ColumnSample {
            alt,
            basement,
            chaos,
            water_level,
            surface_color,
            sub_surface_color,
            stone_col,
            snow_cover,
            cliff_offset,
            cliff_height,
            ice_depth,
            ..
        } = sample;

        let wposf = wpos.map(|e| e as f64);

        // Sample blocks
        let water = Block::new(BlockKind::Water, Rgb::zero());
        let grass_depth = (1.5 + 2.0 * chaos).min(alt - basement);
        if (wposf.z as f32) < alt - grass_depth {
            let stone_factor = (alt - grass_depth - wposf.z as f32) * 0.15;
            let col = Lerp::lerp(
                sub_surface_color,
                stone_col.map(|e| e as f32 / 255.0),
                stone_factor,
            )
            .map(|e| (e * 255.0) as u8);

            if stone_factor >= 0.5 {
                if wposf.z as f32 > alt - cliff_offset.max(0.0) {
                    if cliff_offset.max(0.0)
                        > cliff_height
                            - (FastNoise::new(37).get(wposf / Vec3::new(6.0, 6.0, 10.0)) * 0.5
                                + 0.5)
                                * (alt - grass_depth - wposf.z as f32)
                                    .mul(0.25)
                                    .clamped(0.0, 8.0)
                    {
                        Some(Block::empty())
                    } else {
                        let col = Lerp::lerp(
                            col.map(|e| e as f32),
                            col.map(|e| e as f32) * 0.7,
                            (wposf.z as f32 - basement * 0.3).div(2.0).sin() * 0.5 + 0.5,
                        )
                        .map(|e| e as u8);
                        Some(Block::new(BlockKind::Rock, col))
                    }
                } else {
                    Some(Block::new(BlockKind::Rock, col))
                }
            } else {
                Some(Block::new(BlockKind::Earth, col))
            }
        } else if wposf.z as i32 <= alt as i32 {
            let grass_factor = (wposf.z as f32 - (alt - grass_depth))
                .div(grass_depth)
                .sqrt();
            // Surface
            Some(if water_level > alt.ceil() {
                Block::new(
                    BlockKind::Sand,
                    sub_surface_color.map(|e| (e * 255.0) as u8),
                )
            } else {
                let col = Lerp::lerp(sub_surface_color, surface_color, grass_factor);
                if grass_factor < 0.7 {
                    Block::new(BlockKind::Earth, col.map(|e| (e * 255.0) as u8))
                } else if snow_cover {
                    //if temp < CONFIG.snow_temp + 0.031 {
                    Block::new(BlockKind::Snow, col.map(|e| (e * 255.0) as u8))
                } else {
                    Block::new(BlockKind::Grass, col.map(|e| (e * 255.0) as u8))
                }
            })
        } else {
            None
        }
        .or_else(|| {
            let over_water = alt < water_level;
            // Water
            if over_water && (wposf.z as f32 - water_level).abs() < ice_depth {
                Some(Block::new(BlockKind::Ice, CONFIG.ice_color))
            } else if (wposf.z as f32) < water_level {
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
    pub calendar: Option<&'a Calendar>,
}

impl<'a> ZCache<'a> {
    pub fn get_z_limits(&self) -> (f32, f32) {
        let min = self.sample.alt
            - (self.sample.chaos.min(1.0) * 16.0)
            - self.sample.cliff_offset.max(0.0);
        let min = min - 4.0;

        let warp = self.sample.chaos * 32.0;

        let ground_max = self.sample.alt + warp + 2.0;

        let max = ground_max.max(self.sample.water_level + 2.0 + self.sample.ice_depth);

        (min, max)
    }
}

fn ori_of_unit(unit: &Vec2<i32>) -> u8 {
    if unit.y != 0 {
        if unit.y < 0 { 6 } else { 2 }
    } else if unit.x < 0 {
        4
    } else {
        0
    }
}

fn rotate_for_units(ori: u8, units: &Vec2<Vec2<i32>>) -> u8 {
    let ox = ori_of_unit(&units.x);
    let oy = ori_of_unit(&units.y);
    let positive = ((ox + 8 - oy) & 4) != 0;
    if positive {
        // I have no idea why ox&2 is special here, but it is
        (ox + ori + if (ox & 2) != 0 { 4 } else { 0 }) % 8
    } else {
        (ox + 8 - ori) % 8
    }
}

pub fn block_from_structure(
    index: IndexRef,
    sblock: &StructureBlock,
    pos: Vec3<i32>,
    structure_pos: Vec2<i32>,
    structure_seed: u32,
    sample: &ColumnSample,
    mut with_sprite: impl FnMut(SpriteKind) -> Block,
    calendar: Option<&Calendar>,
    units: &Vec2<Vec2<i32>>,
) -> Option<(Block, Option<SpriteCfg>)> {
    let field = RandomField::new(structure_seed);

    let lerp = ((field.get(Vec3::from(structure_pos)).rem_euclid(256)) as f32 / 255.0) * 0.8
        + ((field.get(pos + i32::MAX / 2).rem_euclid(256)) as f32 / 255.0) * 0.2;

    let block = match sblock {
        StructureBlock::None => None,
        StructureBlock::Hollow => Some(Block::air(SpriteKind::Empty)),
        StructureBlock::Grass => Some(Block::new(
            BlockKind::Grass,
            sample.surface_color.map(|e| (e * 255.0) as u8),
        )),
        StructureBlock::Normal(color) => Some(Block::new(BlockKind::Misc, *color)),
        StructureBlock::Filled(kind, color) => Some(Block::new(*kind, *color)),
        StructureBlock::Sprite(kind) => Some(with_sprite(*kind).into_vacant().with_sprite(*kind)),
        StructureBlock::RotatedSprite(kind, ori) => Some(
            with_sprite(*kind)
                .into_vacant()
                .with_sprite(*kind)
                .with_ori(rotate_for_units(*ori, units))
                .unwrap_or(Block::air(*kind)),
        ),
        StructureBlock::EntitySpawner(_entitykind, _spawn_chance) => {
            Some(Block::new(BlockKind::Air, Rgb::new(255, 255, 255)))
        },
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
            let old_block = with_sprite(SpriteKind::Empty);
            let block = if old_block.is_fluid() {
                old_block
            } else {
                Block::air(SpriteKind::Empty)
            };
            if field.chance(pos + structure_pos, 0.5) {
                Some(block)
            } else {
                Some(block.with_sprite(SpriteKind::Chest))
            }
        },
        StructureBlock::Log => Some(Block::new(BlockKind::Wood, Rgb::new(60, 30, 0))),
        // We interpolate all these BlockKinds as needed.
        StructureBlock::TemperateLeaves
        | StructureBlock::PineLeaves
        | StructureBlock::FrostpineLeaves
        | StructureBlock::PalmLeavesInner
        | StructureBlock::PalmLeavesOuter
        | StructureBlock::Acacia
        | StructureBlock::Mangrove
        | StructureBlock::Chestnut
        | StructureBlock::Baobab
        | StructureBlock::MapleLeaves
        | StructureBlock::CherryLeaves 
        | StructureBlock::AutumnLeaves => {
            let ranges = sblock
                .elim_case_pure(&index.colors.block.structure_blocks)
                .as_ref()
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let range = if ranges.is_empty() {
                None
            } else {
                ranges.get(
                    RandomPerm::new(structure_seed).get(structure_seed) as usize % ranges.len(),
                )
            };

            range.map(|range| {
                if calendar.map_or(false, |c| c.is_event(CalendarEvent::Christmas))
                    && field.chance(pos + structure_pos, 0.025)
                {
                    Block::new(BlockKind::GlowingWeakRock, Rgb::new(255, 0, 0))
                } else if calendar.map_or(false, |c| c.is_event(CalendarEvent::Halloween))
                    && *sblock != StructureBlock::PineLeaves
                {
                    let (c0, c1) = match structure_seed % 6 {
                        0 => (Rgb::new(165.0, 150.0, 11.0), Rgb::new(170.0, 165.0, 16.0)),
                        1 | 2 => (Rgb::new(218.0, 53.0, 3.0), Rgb::new(226.0, 62.0, 5.0)),
                        _ => (Rgb::new(230.0, 120.0, 20.0), Rgb::new(242.0, 130.0, 25.0)),
                    };

                    Block::new(
                        BlockKind::Leaves,
                        Rgb::<f32>::lerp(c0, c1, lerp).map(|e| e as u8),
                    )
                } else {
                    Block::new(
                        BlockKind::Leaves,
                        Rgb::<f32>::lerp(
                            Rgb::<u8>::from(range.start).map(f32::from),
                            Rgb::<u8>::from(range.end).map(f32::from),
                            lerp,
                        )
                        .map(|e| e as u8),
                    )
                }
            })
        },
        StructureBlock::BirchWood => {
            let wpos = pos + structure_pos;
            if field.chance(
                (wpos + Vec3::new(wpos.z, wpos.z, 0) / 2)
                    / Vec3::new(1 + wpos.z % 2, 1 + (wpos.z + 1) % 2, 1),
                0.25,
            ) && wpos.z % 2 == 0
            {
                Some(Block::new(BlockKind::Wood, Rgb::new(70, 35, 25)))
            } else {
                Some(Block::new(BlockKind::Wood, Rgb::new(220, 170, 160)))
            }
        },
        StructureBlock::Keyhole(consumes) => {
            return Some((
                Block::air(SpriteKind::Keyhole),
                Some(SpriteCfg {
                    unlock: Some(UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                        consumes.clone(),
                    ))),
                    ..SpriteCfg::default()
                }),
            ));
        },
        StructureBlock::Sign(content, ori) => {
            return Some((
                Block::air(SpriteKind::Sign)
                    .with_ori(rotate_for_units(*ori, units))
                    .expect("signs can always be rotated"),
                Some(SpriteCfg {
                    content: Some(content.clone()),
                    ..SpriteCfg::default()
                }),
            ));
        },
        StructureBlock::BoneKeyhole(consumes) => {
            return Some((
                Block::air(SpriteKind::BoneKeyhole),
                Some(SpriteCfg {
                    unlock: Some(UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                        consumes.clone(),
                    ))),
                    ..SpriteCfg::default()
                }),
            ));
        },
        StructureBlock::HaniwaKeyhole(consumes) => {
            return Some((
                Block::air(SpriteKind::HaniwaKeyhole),
                Some(SpriteCfg {
                    unlock: Some(UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                        consumes.clone(),
                    ))),
                    ..SpriteCfg::default()
                }),
            ));
        },
        StructureBlock::KeyholeBars(consumes) => {
            return Some((
                Block::air(SpriteKind::KeyholeBars),
                Some(SpriteCfg {
                    unlock: Some(UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                        consumes.clone(),
                    ))),
                    ..SpriteCfg::default()
                }),
            ));
        },
        StructureBlock::GlassKeyhole(consumes) => {
            return Some((
                Block::air(SpriteKind::GlassKeyhole),
                Some(SpriteCfg {
                    unlock: Some(UnlockKind::Consumes(ItemDefinitionIdOwned::Simple(
                        consumes.clone(),
                    ))),
                    ..SpriteCfg::default()
                }),
            ));
        },
    };

    Some((block?, None))
}
