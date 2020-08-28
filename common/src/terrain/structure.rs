use super::BlockKind;
use crate::{
    assets::{self, Asset, Ron},
    make_case_elim,
    vol::{BaseVol, ReadVol, SizedVol, Vox, WriteVol},
    volumes::dyna::{Dyna, DynaError},
};
use dot_vox::DotVoxData;
use serde::Deserialize;
use std::{fs::File, io::BufReader, sync::Arc};
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
    }
);

impl Vox for StructureBlock {
    fn empty() -> Self { StructureBlock::None }

    fn is_empty(&self) -> bool { matches!(self, StructureBlock::None) }
}

#[derive(Debug)]
pub enum StructureError {}

#[derive(Clone)]
pub struct Structure {
    center: Vec3<i32>,
    vol: Dyna<StructureBlock, ()>,
    empty: StructureBlock,
    default_kind: BlockKind,
}

impl Structure {
    pub fn load_group(specifier: &str) -> Vec<Arc<Structure>> {
        let spec = StructuresSpec::load_expect(&["world.manifests.", specifier].concat());
        spec.iter()
            .map(|sp| {
                Structure::load_map(&sp.specifier[..], |s| s.with_center(Vec3::from(sp.center)))
                    .unwrap()
            })
            .collect()
    }

    pub fn with_center(mut self, center: Vec3<i32>) -> Self {
        self.center = center;
        self
    }

    pub fn with_default_kind(mut self, kind: BlockKind) -> Self {
        self.default_kind = kind;
        self
    }

    pub fn get_bounds(&self) -> Aabb<i32> {
        Aabb {
            min: -self.center,
            max: self.vol.size().map(|e| e as i32) - self.center,
        }
    }

    pub fn default_kind(&self) -> BlockKind { self.default_kind }
}

impl BaseVol for Structure {
    type Error = StructureError;
    type Vox = StructureBlock;
}

impl ReadVol for Structure {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, StructureError> {
        match self.vol.get(pos + self.center) {
            Ok(block) => Ok(block),
            Err(DynaError::OutOfBounds) => Ok(&self.empty),
        }
    }
}

impl Asset for Structure {
    const ENDINGS: &'static [&'static str] = &["vox"];

    fn parse(buf_reader: BufReader<File>) -> Result<Self, assets::Error> {
        let dot_vox_data = DotVoxData::parse(buf_reader)?;

        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut vol = Dyna::filled(
                Vec3::new(model.size.x, model.size.y, model.size.z),
                StructureBlock::empty(),
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

            Ok(Structure {
                center: Vec3::zero(),
                vol,
                empty: StructureBlock::empty(),
                default_kind: BlockKind::Normal,
            })
        } else {
            Ok(Self {
                center: Vec3::zero(),
                vol: Dyna::filled(Vec3::zero(), StructureBlock::empty(), ()),
                empty: StructureBlock::empty(),
                default_kind: BlockKind::Normal,
            })
        }
    }
}

#[derive(Deserialize)]
struct StructureSpec {
    specifier: String,
    center: [i32; 3],
}

type StructuresSpec = Ron<Vec<StructureSpec>>;
