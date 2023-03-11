use super::{BlockKind, SpriteKind};
use crate::{
    assets::{self, AssetExt, AssetHandle, BoxedError, DotVoxAsset},
    make_case_elim,
    vol::{BaseVol, ReadVol, SizedVol, WriteVol},
    volumes::dyna::{Dyna, DynaError},
};
use hashbrown::HashMap;
use serde::Deserialize;
use std::{num::NonZeroU8, sync::Arc};
use vek::*;

make_case_elim!(
    structure_block,
    #[derive(Clone, PartialEq, Debug, Deserialize)]
    #[repr(u8)]
    pub enum StructureBlock {
        None = 0,
        Grass = 1,
        TemperateLeaves = 2,
        PineLeaves = 3,
        Acacia = 4,
        Mangrove = 5,
        PalmLeavesInner = 6,
        PalmLeavesOuter = 7,
        Water = 8,
        GreenSludge = 9,
        Fruit = 10,
        Coconut = 11,
        Chest = 12,
        Hollow = 13,
        Liana = 14,
        Normal(color: Rgb<u8>) = 15,
        Log = 16,
        Filled(kind: BlockKind, color: Rgb<u8>) = 17,
        Sprite(kind: SpriteKind) = 18,
        Chestnut = 19,
        Baobab = 20,
        BirchWood = 21,
        FrostpineLeaves = 22,
        RotatedSprite(kind: SpriteKind, ori: u8) = 23,
        EntitySpawner(entitykind: String, spawn_chance: f32) = 24,
        Keyhole(consumes: String) = 25,
    }
);

#[derive(Debug)]
pub enum StructureError {
    OutOfBounds,
}

#[derive(Clone, Debug)]
pub struct Structure {
    center: Vec3<i32>,
    base: Arc<BaseStructure>,
    custom_indices: [Option<StructureBlock>; 256],
}

#[derive(Debug)]
struct BaseStructure {
    vol: Dyna<Option<NonZeroU8>, ()>,
    palette: [StructureBlock; 256],
}

pub struct StructuresGroup(Vec<Structure>);

impl std::ops::Deref for StructuresGroup {
    type Target = [Structure];

    fn deref(&self) -> &[Structure] { &self.0 }
}

impl assets::Compound for StructuresGroup {
    fn load(cache: assets::AnyCache, specifier: &assets::SharedString) -> Result<Self, BoxedError> {
        let specs = cache.load::<StructuresGroupSpec>(specifier)?.read();

        Ok(StructuresGroup(
            specs
                .0
                .iter()
                .map(|sp| {
                    let base = cache.load::<Arc<BaseStructure>>(&sp.specifier)?.cloned();
                    Ok(Structure {
                        center: Vec3::from(sp.center),
                        base,
                        custom_indices: {
                            let mut indices = std::array::from_fn(|_| None);
                            for (&idx, custom) in default_custom_indices()
                                .iter()
                                .chain(sp.custom_indices.iter())
                            {
                                indices[idx as usize] = Some(custom.clone());
                            }
                            indices
                        },
                    })
                })
                .collect::<Result<_, BoxedError>>()?,
        ))
    }
}

impl Structure {
    pub fn load_group(specifier: &str) -> AssetHandle<StructuresGroup> {
        StructuresGroup::load_expect(&["world.manifests.", specifier].concat())
    }

    #[must_use]
    pub fn with_center(mut self, center: Vec3<i32>) -> Self {
        self.center = center;
        self
    }

    pub fn get_bounds(&self) -> Aabb<i32> {
        Aabb {
            min: -self.center,
            max: self.base.vol.size().map(|e| e as i32) - self.center,
        }
    }
}

impl BaseVol for Structure {
    type Error = StructureError;
    type Vox = StructureBlock;
}

impl ReadVol for Structure {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, StructureError> {
        match self.base.vol.get(pos + self.center) {
            Ok(None) => Ok(&StructureBlock::None),
            Ok(Some(index)) => match &self.custom_indices[index.get() as usize] {
                Some(sb) => Ok(sb),
                None => Ok(&self.base.palette[index.get() as usize]),
            },
            Err(DynaError::OutOfBounds) => Err(StructureError::OutOfBounds),
        }
    }
}

impl assets::Compound for BaseStructure {
    fn load(cache: assets::AnyCache, specifier: &assets::SharedString) -> Result<Self, BoxedError> {
        let dot_vox_data = cache.load::<DotVoxAsset>(specifier)?.read();
        let dot_vox_data = &dot_vox_data.0;

        if let Some(model) = dot_vox_data.models.get(0) {
            let mut palette = std::array::from_fn(|_| StructureBlock::None);

            for (i, col) in dot_vox_data
                .palette
                .iter()
                .map(|col| Rgb::new(col.r, col.g, col.b))
                .enumerate()
            {
                palette[(i + 1).min(255)] = StructureBlock::Filled(BlockKind::Misc, col);
            }

            let mut vol = Dyna::filled(
                Vec3::new(model.size.x, model.size.y, model.size.z),
                None,
                (),
            );

            for voxel in &model.voxels {
                let _ = vol.set(
                    Vec3::new(voxel.x, voxel.y, voxel.z).map(i32::from),
                    Some(NonZeroU8::new(voxel.i + 1).unwrap()),
                );
            }

            Ok(BaseStructure { vol, palette })
        } else {
            Ok(BaseStructure {
                vol: Dyna::filled(Vec3::zero(), None, ()),
                palette: std::array::from_fn(|_| StructureBlock::None),
            })
        }
    }
}

#[derive(Deserialize)]
struct StructureSpec {
    specifier: String,
    center: [i32; 3],
    #[serde(default)]
    custom_indices: HashMap<u8, StructureBlock>,
}

fn default_custom_indices() -> HashMap<u8, StructureBlock> {
    let blocks: [_; 16] = [
        /* 1 */ Some(StructureBlock::TemperateLeaves),
        /* 2 */ Some(StructureBlock::PineLeaves),
        /* 3 */ None,
        /* 4 */ Some(StructureBlock::Water),
        /* 5 */ Some(StructureBlock::Acacia),
        /* 6 */ Some(StructureBlock::Mangrove),
        /* 7 */ Some(StructureBlock::GreenSludge),
        /* 8 */ Some(StructureBlock::Fruit),
        /* 9 */ Some(StructureBlock::Grass),
        /* 10 */ Some(StructureBlock::Liana),
        /* 11 */ Some(StructureBlock::Chest),
        /* 12 */ Some(StructureBlock::Coconut),
        /* 13 */ None,
        /* 14 */ Some(StructureBlock::PalmLeavesOuter),
        /* 15 */ Some(StructureBlock::PalmLeavesInner),
        /* 16 */ Some(StructureBlock::Hollow),
    ];

    blocks
        .iter()
        .enumerate()
        .filter_map(|(i, sb)| sb.as_ref().map(|sb| (i as u8 + 1, sb.clone())))
        .collect()
}

#[derive(Deserialize)]
struct StructuresGroupSpec(Vec<StructureSpec>);

impl assets::Asset for StructuresGroupSpec {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
