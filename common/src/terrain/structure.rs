use super::{Block, BlockKind};
use crate::{
    assets::{self, Asset},
    vol::{BaseVol, ReadVol, SizedVol, Vox, WriteVol},
    volumes::dyna::{Dyna, DynaErr},
};
use dot_vox::DotVoxData;
use std::fs::File;
use std::io::{BufReader, Read};
use vek::*;

#[derive(Copy, Clone)]
pub enum StructureBlock {
    None,
    TemperateLeaves,
    PineLeaves,
    Acacia,
    PalmLeaves,
    Water,
    GreenSludge,
    Fruit,
    Hollow,
    Normal(Rgb<u8>),
}

impl Vox for StructureBlock {
    fn empty() -> Self {
        StructureBlock::None
    }

    fn is_empty(&self) -> bool {
        match self {
            StructureBlock::None => true,
            _ => false,
        }
    }
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
            max: self.vol.get_size().map(|e| e as i32) - self.center,
        }
    }

    pub fn default_kind(&self) -> BlockKind {
        self.default_kind
    }
}

impl BaseVol for Structure {
    type Vox = StructureBlock;
    type Err = StructureError;
}

impl ReadVol for Structure {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Self::Vox, StructureError> {
        match self.vol.get(pos + self.center) {
            Ok(block) => Ok(block),
            Err(DynaErr::OutOfBounds) => Ok(&self.empty),
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
                    2 => StructureBlock::PalmLeaves,
                    3 => StructureBlock::Water,
                    4 => StructureBlock::Acacia,
                    6 => StructureBlock::GreenSludge,
                    7 => StructureBlock::Fruit,
                    15 => StructureBlock::Hollow,
                    index => {
                        let color = palette
                            .get(index as usize)
                            .copied()
                            .unwrap_or_else(|| Rgb::broadcast(0));
                        StructureBlock::Normal(color)
                    }
                };

                let _ = vol.set(
                    Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| i32::from(e)),
                    block,
                );
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
