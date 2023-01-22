pub mod cell;
pub mod mat_cell;
pub use mat_cell::Material;

// Reexport
pub use self::{
    cell::{Cell, CellData},
    mat_cell::MatCell,
};

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
    fn from(dot_vox_data: &DotVoxData) -> Self { Segment::from_vox(dot_vox_data, false) }
}

impl Segment {
    /// Take a list of voxel data, offsets, and x-mirror flags, and assembled
    /// them into a combined segment
    pub fn from_voxes(data: &[(&DotVoxData, Vec3<i32>, bool)]) -> (Self, Vec3<i32>) {
        let mut union = DynaUnionizer::new();
        for (datum, offset, xmirror) in data.iter() {
            union = union.add(Segment::from_vox(datum, *xmirror), *offset);
        }
        union.unify()
    }

    pub fn from_vox(dot_vox_data: &DotVoxData, flipped: bool) -> Self {
        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgb::new(col.r, col.g, col.b))
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
                            Vec3::new(
                                if flipped {
                                    model.size.x as u8 - 1 - voxel.x
                                } else {
                                    voxel.x
                                },
                                voxel.y,
                                voxel.z,
                            )
                            .map(i32::from),
                            Cell::new(
                                color,
                                (13..16).contains(&voxel.i), // Glowy
                                (8..13).contains(&voxel.i),  // Shiny
                                voxel.i == 16,               //Hollow
                            ),
                        )
                        .unwrap();
                };
            }

            segment
        } else {
            Segment::filled(Vec3::zero(), Cell::empty(), ())
        }
    }

    /// Transform cells
    #[must_use]
    pub fn map(mut self, transform: impl Fn(Cell) -> Option<Cell>) -> Self {
        for pos in self.full_pos_iter() {
            if let Some(new) = transform(*self.get(pos).unwrap()) {
                self.set(pos, new).unwrap();
            }
        }

        self
    }

    /// Transform cell colors
    #[must_use]
    pub fn map_rgb(self, transform: impl Fn(Rgb<u8>) -> Rgb<u8>) -> Self {
        self.map(|cell| {
            cell.get_color().map(|rgb| {
                Cell::new(
                    transform(rgb),
                    cell.is_glowy(),
                    cell.is_shiny(),
                    cell.is_hollow(),
                )
            })
        })
    }
}

// TODO: move
/// A `Dyna` builder that combines Dynas
pub struct DynaUnionizer<V: Vox>(Vec<(Dyna<V, ()>, Vec3<i32>)>);

impl<V: Vox + Copy> DynaUnionizer<V> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self { DynaUnionizer(Vec::new()) }

    #[must_use]
    pub fn add(mut self, dyna: Dyna<V, ()>, offset: Vec3<i32>) -> Self {
        self.0.push((dyna, offset));
        self
    }

    #[must_use]
    pub fn maybe_add(self, maybe: Option<(Dyna<V, ()>, Vec3<i32>)>) -> Self {
        match maybe {
            Some((dyna, offset)) => self.add(dyna, offset),
            None => self,
        }
    }

    pub fn unify(self) -> (Dyna<V, ()>, Vec3<i32>) { self.unify_with(|v| v) }

    pub fn unify_with(self, mut f: impl FnMut(V) -> V) -> (Dyna<V, ()>, Vec3<i32>) {
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
        let origin = min_point.map(|e| -e);
        for (dyna, offset) in self.0 {
            for (pos, vox) in dyna.full_vol_iter() {
                if !vox.is_empty() {
                    combined.set(origin + offset + pos, f(*vox)).unwrap();
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
            let data = match vox {
                MatCell::None => continue,
                MatCell::Mat(mat) => CellData::new(map(*mat), false, false, false),
                MatCell::Normal(data) => *data,
            };
            vol.set(pos, Cell::Filled(data)).unwrap();
        }
        vol
    }

    /// Transform cells
    #[must_use]
    pub fn map(mut self, transform: impl Fn(MatCell) -> Option<MatCell>) -> Self {
        for pos in self.full_pos_iter() {
            if let Some(new) = transform(*self.get(pos).unwrap()) {
                self.set(pos, new).unwrap();
            }
        }

        self
    }

    /// Transform cell colors
    #[must_use]
    pub fn map_rgb(self, transform: impl Fn(Rgb<u8>) -> Rgb<u8>) -> Self {
        self.map(|cell| match cell {
            MatCell::Normal(data) => Some(MatCell::Normal(CellData {
                col: transform(data.col),
                ..data
            })),
            _ => None,
        })
    }

    pub fn from_vox(dot_vox_data: &DotVoxData, flipped: bool) -> Self {
        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgb::new(col.r, col.g, col.b))
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
                        MatCell::Normal(CellData::new(
                            color,
                            (13..16).contains(&index),
                            (8..13).contains(&index),
                            index == 16, // Hollow
                        ))
                    },
                };

                vol.set(
                    Vec3::new(
                        if flipped {
                            model.size.x as u8 - 1 - voxel.x
                        } else {
                            voxel.x
                        },
                        voxel.y,
                        voxel.z,
                    )
                    .map(i32::from),
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

impl From<&DotVoxData> for MatSegment {
    fn from(dot_vox_data: &DotVoxData) -> Self { Self::from_vox(dot_vox_data, false) }
}
