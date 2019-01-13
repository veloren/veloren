pub mod cell;

// Library
use vek::*;
use dot_vox::DotVoxData;

// Crate
use crate::{
    vol::WriteVol,
    volumes::dyna::Dyna,
};

// Local
use self::cell::Cell;

/// A type representing a single figure bone (e.g: the limb of a character).
#[derive(Copy, Clone)]
pub struct Bone {
    origin: Vec3<f32>,
    offset: Vec3<f32>,
    ori: Vec3<f32>,
}

/// A type representing a volume that may be part of an animated figure.
///
/// Figures are used to represent things like characters, NPCs, mobs, etc.
pub type Segment = Dyna<Cell, ()>;

impl From<DotVoxData> for Segment {
    fn from(dot_vox_data: DotVoxData) -> Self {
        if let Some(model) = dot_vox_data.models.get(0) {
            let palette = dot_vox_data
                .palette
                .iter()
                .map(|col| Rgba::from(col.to_ne_bytes()).into())
                .collect::<Vec<_>>();

            let mut segment = Segment::filled(
                Vec3::new(
                    model.size.x,
                    model.size.y,
                    model.size.z,
                ),
                Cell::empty(),
                (),
            );

            for voxel in &model.voxels {
                if let Some(&color) = palette.get(voxel.i as usize) {
                    // TODO: Maybe don't ignore this error?
                    let _ = segment.set(
                        Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| e as i32),
                        Cell::new(color),
                    );
                }
            }

            segment
        } else {
            Segment::filled(Vec3::zero(), Cell::empty(), ())
        }
    }
}
