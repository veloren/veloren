pub mod cell;
pub mod mat_cell;
pub use mat_cell::Material;

use self::cell::Cell;
use self::mat_cell::MatCell;
use crate::{
    vol::{IntoFullPosIterator, IntoFullVolIterator, ReadVol, SizedVol, Vox, VolSize, WriteVol},
    volumes::{
        dyna::Dyna,
        chunk::Chunk,
        vol_grid_3d::VolGrid3d,
    },
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
    pub fn new() -> Self {
        DynaUnionizer(Vec::new())
    }
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
                    7 => MatCell::Mat(Material::EyeWhite),
                    //1 => MatCell::Mat(Material::HairLight),
                    //1 => MatCell::Mat(Material::HairDark),
                    //6 => MatCell::Mat(Material::Clothing),
                    index => {
                        let color = palette
                            .get(index as usize)
                            .copied()
                            .unwrap_or_else(|| Rgb::broadcast(0));
                        MatCell::Normal(color)
                    }
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

#[derive(Clone, Debug)]
pub struct SscSize;
impl VolSize for SscSize {
    const SIZE: Vec3<u32> = Vec3 { x: 32, y: 32, z: 32 };
}

pub type SparseScene = VolGrid3d<Chunk<Cell, SscSize, ()>>;

impl From<&DotVoxData> for SparseScene {
    fn from(dot_vox_data: &DotVoxData) -> Self {
        let mut sparse_scene = match VolGrid3d::new() {
            Ok(ok) => ok,
            Err(_) => panic!(),
        };

        for (transform, model_id) in &dot_vox_data.scene {
            if let Some(model) = dot_vox_data.models.get(*model_id) {
                let palette = dot_vox_data
                    .palette
                    .iter()
                    .map(|col| Rgba::from(col.to_ne_bytes()).into())
                    .collect::<Vec<_>>();
                
                // Rotation
                let rot = Mat3::from_row_arrays(transform.r).map(|e| e as i32);
                // Get the rotated size of the model
                let size = rot.map(|e| e.abs() as u32) * Vec3::new(model.size.x, model.size.y, model.size.z);
                // Position of min corner
                let pos = Vec3::<i32>::from(transform.t)
                    .map2(size, |m, s| (s, m))
                    .map2(rot * Vec3::<i32>::one(), |(s, m), f| m - (s as i32 + f.min(0) * -1) / 2);
                dbg!(pos);
                // Insert required chunks
                let min_key = sparse_scene.pos_key(pos);
                let max_key = sparse_scene.pos_key(pos+size.map(|e| e as i32 - 1));
                for x in min_key.x..=max_key.x {
                    for y in min_key.y..=max_key.y {
                        for z in min_key.z..=max_key.z {
                            let key = Vec3::new(x, y, z);
                            if sparse_scene.get_key_arc(key).is_none() {
                                sparse_scene.insert(key, std::sync::Arc::new(Chunk::filled(Cell::empty(), ())));
                            }
                        }
                    }
                }

                let offset = (rot * Vec3::new(model.size.x, model.size.y, model.size.z).map(|e| e as i32)).map(|e| if e > 0 { 0 } else { - e - 1});
                for voxel in &model.voxels {
                    if let Some(&color) = palette.get(voxel.i as usize) {
                        sparse_scene.set(
                                (rot * Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| i32::from(e))) + offset + pos,
                                Cell::new(color),
                            )
                            .unwrap();
                    }
                }
            }
        }
        sparse_scene
    }
}