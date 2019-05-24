use dot_vox::DotVoxData;
use vek::*;
use crate::{
    assets::{self, Asset, load_from_path},
    volumes::dyna::{Dyna, DynaErr},
    vol::{Vox, BaseVol, ReadVol, WriteVol},
};
use super::Block;

#[derive(Debug)]
pub enum StructureError {}

#[derive(Clone)]
pub struct Structure {
    center: Vec3<i32>,
    vol: Dyna<Block, ()>,
    empty: Block,
}

impl Structure {
    pub fn with_center(mut self, center: Vec3<i32>) -> Self {
        self.center = center;
        self
    }
}

impl BaseVol for Structure {
    type Vox = Block;
    type Err = StructureError;
}

impl ReadVol for Structure {
    #[inline(always)]
    fn get(&self, pos: Vec3<i32>) -> Result<&Block, StructureError> {
        match self.vol.get(pos + self.center) {
            Ok(block) => Ok(block),
            Err(DynaErr::OutOfBounds) => Ok(&self.empty),
        }
    }
}

impl Asset for Structure {
    fn load(specifier: &str) -> Result<Self, assets::Error> {
        let dot_vox_data = DotVoxData::load(specifier)?;

        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut vol = Dyna::filled(Vec3::new(
                model.size.x,
                model.size.y,
                model.size.z,
            ), Block::empty(), ());

            for voxel in &model.voxels {
                if let Some(&color) = palette.get(voxel.i as usize) {
                    let _ = vol.set(
                        Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| e as i32),
                        Block::new(1, color),
                    );
                }
            }

            Ok(Structure {
                center: Vec3::zero(),
                vol,
                empty: Block::empty(),
            })
        } else {
            Ok(Self {
                center: Vec3::zero(),
                vol: Dyna::filled(Vec3::zero(), Block::empty(), ()),
                empty: Block::empty(),
            })
        }
    }
}
