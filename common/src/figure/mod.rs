pub mod cell;
pub mod mat_cell;
pub use mat_cell::Material;

use self::{cell::Cell, mat_cell::MatCell};
use crate::{
    vol::{IntoFullPosIterator, IntoFullVolIterator, ReadVol, SizedVol, Vox, WriteVol},
    volumes::dyna::Dyna,
};
use dot_vox::DotVoxData;
use vek::*;

/// A type representing a volume that may be part of an animated figure.
///
/// Figures are used to represent things like characters, NPCs, mobs, etc.
pub type Segment = Dyna<Cell, ()>;

impl From<&DotVoxData> for Segment {
    fn from(dot_vox_data: &DotVoxData) -> Self {
        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut segment = Segment::filled(
                Vec3::new(model.size.x, model.size.y, model.size.z),
                Cell::empty(),
                (),
            );

            for voxel in &model.voxels {
                if let Some(&color) = palette.get(voxel.i as usize) {
                    segment
                        .set(
                            Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| i32::from(e)),
                            Cell::new(color),
                        )
                        .unwrap();
                }
            }

            segment
        } else {
            Segment::filled(Vec3::zero(), Cell::empty(), ())
        }
    }
}

impl Segment {
    /// Transform cells
    pub fn map(mut self, transform: impl Fn(Cell) -> Option<Cell>) -> Self {
        for pos in self.full_pos_iter() {
            if let Some(new) = transform(*self.get(pos).unwrap()) {
                self.set(pos, new).unwrap();
            }
        }

        self
    }

    /// Transform cell colors
    pub fn map_rgb(self, transform: impl Fn(Rgb<u8>) -> Rgb<u8>) -> Self {
        self.map(|cell| cell.get_color().map(|rgb| Cell::new(transform(rgb))))
    }
}

// TODO: move
/// A `Dyna` builder that combines Dynas
pub struct DynaUnionizer<V: Vox>(Vec<(Dyna<V, ()>, Vec3<i32>)>);

impl<V: Vox + Copy> DynaUnionizer<V> {
    pub fn new() -> Self { DynaUnionizer(Vec::new()) }

    pub fn add(mut self, dyna: Dyna<V, ()>, offset: Vec3<i32>) -> Self {
        self.0.push((dyna, offset));
        self
    }

    pub fn maybe_add(self, maybe: Option<(Dyna<V, ()>, Vec3<i32>)>) -> Self {
        match maybe {
            Some((dyna, offset)) => self.add(dyna, offset),
            None => self,
        }
    }

    pub fn unify(self) -> (Dyna<V, ()>, Vec3<i32>) {
        if self.0.is_empty() {
            return (Dyna::filled(Vec3::zero(), V::empty(), ()), Vec3::zero());
        }

        // Determine size of the new Dyna
        let mut min_point = self.0[0].1;
        let mut max_point = self.0[0].1 + self.0[0].0.size().map(|e| e as i32);
        for (dyna, offset) in self.0.iter().skip(1) {
            let size = dyna.size().map(|e| e as i32);
            min_point = min_point.map2(*offset, std::cmp::min);
            max_point = max_point.map2(offset + size, std::cmp::max);
        }
        let new_size = (max_point - min_point).map(|e| e as u32);
        // Allocate new segment
        let mut combined = Dyna::filled(new_size, V::empty(), ());
        // Copy segments into combined
        let origin = min_point.map(|e| e * -1);
        for (dyna, offset) in self.0 {
            for (pos, vox) in dyna.full_vol_iter() {
                if !vox.is_empty() {
                    combined.set(origin + offset + pos, *vox).unwrap();
                }
            }
        }

        (combined, origin)
    }
}

pub type MatSegment = Dyna<MatCell, ()>;

impl MatSegment {
    pub fn to_segment(&self, map: impl Fn(Material) -> Rgb<u8>) -> Segment {
        let mut vol = Dyna::filled(self.size(), Cell::empty(), ());
        for (pos, vox) in self.full_vol_iter() {
            let rgb = match vox {
                MatCell::None => continue,
                MatCell::Mat(mat) => map(*mat),
                MatCell::Normal(rgb) => *rgb,
            };
            vol.set(pos, Cell::new(rgb)).unwrap();
        }
        vol
    }

    /// Transform cells
    pub fn map(mut self, transform: impl Fn(MatCell) -> Option<MatCell>) -> Self {
        for pos in self.full_pos_iter() {
            if let Some(new) = transform(*self.get(pos).unwrap()) {
                self.set(pos, new).unwrap();
            }
        }

        self
    }

    /// Transform cell colors
    pub fn map_rgb(self, transform: impl Fn(Rgb<u8>) -> Rgb<u8>) -> Self {
        self.map(|cell| match cell {
            MatCell::Normal(rgb) => Some(MatCell::Normal(transform(rgb))),
            _ => None,
        })
    }
}

impl From<&DotVoxData> for MatSegment {
    fn from(dot_vox_data: &DotVoxData) -> Self {
        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut vol = Dyna::filled(
                Vec3::new(model.size.x, model.size.y, model.size.z),
                MatCell::empty(),
                (),
            );

            for voxel in &model.voxels {
                let block = match voxel.i {
                    0 => MatCell::Mat(Material::Skin),
                    1 => MatCell::Mat(Material::Hair),
                    2 => MatCell::Mat(Material::EyeDark),
                    3 => MatCell::Mat(Material::EyeLight),
                    4 => MatCell::Mat(Material::SkinDark),
                    5 => MatCell::Mat(Material::SkinLight),
                    7 => MatCell::Mat(Material::EyeWhite),
                    //6 => MatCell::Mat(Material::Clothing),
                    index => {
                        let color = palette
                            .get(index as usize)
                            .copied()
                            .unwrap_or_else(|| Rgb::broadcast(0));
                        MatCell::Normal(color)
                    },
                };

                vol.set(
                    Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| i32::from(e)),
                    block,
                )
                .unwrap();
            }

            vol
        } else {
            Dyna::filled(Vec3::zero(), MatCell::empty(), ())
        }
    }
}
