use super::BlockKind;
use crate::{
    assets::{self, AssetExt, AssetHandle, DotVoxAsset, Error},
    make_case_elim,
    vol::{BaseVol, ReadVol, SizedVol, WriteVol},
    volumes::dyna::{Dyna, DynaError},
};
use serde::Deserialize;
use std::sync::Arc;
use vek::*;

make_case_elim!(
    structure_block,
    #[derive(Copy, Clone, PartialEq)]
    #[repr(u32)]
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
        Block(kind: BlockKind, color: Rgb<u8>) = 17,
    }
);

#[derive(Debug)]
pub enum StructureError {
    OutOfBounds,
}

#[derive(Clone)]
pub struct Structure {
    center: Vec3<i32>,
    base: Arc<BaseStructure>,
}

struct BaseStructure {
    vol: Dyna<StructureBlock, ()>,
    default_kind: BlockKind,
}

pub struct StructuresGroup(Vec<Structure>);

impl std::ops::Deref for StructuresGroup {
    type Target = [Structure];

    fn deref(&self) -> &[Structure] { &self.0 }
}

impl assets::Compound for StructuresGroup {
    fn load<S: assets_manager::source::Source>(
        cache: &assets_manager::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, Error> {
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
                    })
                })
                .collect::<Result<_, Error>>()?,
        ))
    }
}

impl Structure {
    pub fn load_group(specifier: &str) -> AssetHandle<StructuresGroup> {
        StructuresGroup::load_expect(&["world.manifests.", specifier].concat())
    }

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

    pub fn default_kind(&self) -> BlockKind { self.base.default_kind }
}

impl BaseVol for Structure {
    type Error = StructureError;
    type Vox = StructureBlock;
}

impl ReadVol for Structure {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, StructureError> {
        match self.base.vol.get(pos + self.center) {
            Ok(block) => Ok(block),
            Err(DynaError::OutOfBounds) => Err(StructureError::OutOfBounds),
        }
    }
}

impl assets::Compound for BaseStructure {
    fn load<S: assets_manager::source::Source>(
        cache: &assets_manager::AssetCache<S>,
        specifier: &str,
    ) -> Result<Self, Error> {
        let dot_vox_data = cache.load::<DotVoxAsset>(specifier)?.read();
        let dot_vox_data = &dot_vox_data.0;

        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut vol = Dyna::filled(
                Vec3::new(model.size.x, model.size.y, model.size.z),
                StructureBlock::None,
                (),
            );

            for voxel in &model.voxels {
                let block = match voxel.i {
                    0 => StructureBlock::TemperateLeaves,
                    1 => StructureBlock::PineLeaves,
                    3 => StructureBlock::Water,
                    4 => StructureBlock::Acacia,
                    5 => StructureBlock::Mangrove,
                    6 => StructureBlock::GreenSludge,
                    7 => StructureBlock::Fruit,
                    8 => StructureBlock::Grass,
                    9 => StructureBlock::Liana,
                    10 => StructureBlock::Chest,
                    11 => StructureBlock::Coconut,
                    13 => StructureBlock::PalmLeavesOuter,
                    14 => StructureBlock::PalmLeavesInner,
                    15 => StructureBlock::Hollow,
                    index => {
                        let color = palette
                            .get(index as usize)
                            .copied()
                            .unwrap_or_else(|| Rgb::broadcast(0));
                        StructureBlock::Normal(color)
                    },
                };

                let _ = vol.set(Vec3::new(voxel.x, voxel.y, voxel.z).map(i32::from), block);
            }

            Ok(BaseStructure {
                vol,
                default_kind: BlockKind::Misc,
            })
        } else {
            Ok(BaseStructure {
                vol: Dyna::filled(Vec3::zero(), StructureBlock::None, ()),
                default_kind: BlockKind::Misc,
            })
        }
    }
}

#[derive(Deserialize)]
struct StructureSpec {
    specifier: String,
    center: [i32; 3],
}

#[derive(Deserialize)]
struct StructuresGroupSpec(Vec<StructureSpec>);

impl assets::Asset for StructuresGroupSpec {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}
