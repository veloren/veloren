pub mod cell;

use self::cell::Cell;
use crate::{
    util::chromify_srgb,
    vol::{ReadVol, SizedVol, Vox, WriteVol},
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
                    // TODO: Maybe don't ignore this error?
                    let _ = segment.set(
                        Vec3::new(voxel.x, voxel.y, voxel.z).map(|e| i32::from(e)),
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

impl Segment {
    /// Create a new `Segment` by combining two existing ones
    pub fn union(&self, other: &Self, other_offset: Vec3<i32>) -> Self {
        let size = self.get_size();
        let other_size = self.get_size();
        let new_size = other_offset
            .map2(other_size, |oo, os| (oo, os))
            .map2(size, |(oo, os), s| {
                (oo + os as i32).max(s as i32) - oo.min(0)
            })
            .map(|e| e as u32);
        let mut combined = Segment::filled(new_size, Cell::empty(), ());
        // Copy self into combined
        let offset = other_offset.map(|e| e.min(0).abs());
        for pos in self.iter_positions() {
            if let Cell::Filled(col) = *self.get(pos).unwrap() {
                combined.set(pos + offset, Cell::Filled(col)).unwrap();
            }
        }
        // Copy other into combined
        let offset = other_offset.map(|e| e.max(0));
        for pos in other.iter_positions() {
            if let Cell::Filled(col) = *other.get(pos).unwrap() {
                combined.set(pos + offset, Cell::Filled(col)).unwrap();
            }
        }

        combined
    }
    /// Replaces one cell with another
    pub fn replace(mut self, old: Cell, new: Cell) -> Self {
        for pos in self.iter_positions() {
            if old == *self.get(pos).unwrap() {
                self.set(pos, new);
            }
        }

        self
    }
    /// Preserve the luminance of all the colors but set the chomaticity to match the provided color
    pub fn chromify(mut self, chroma: Rgb<u8>) -> Self {
        let chroma = chroma.map(|e| e as f32 * 255.0);
        for pos in self.iter_positions() {
            match self.get(pos).unwrap() {
                Cell::Filled(rgb) => self
                    .set(
                        pos,
                        Cell::Filled(
                            chromify_srgb(Rgb::from_slice(rgb).map(|e| e as f32 / 255.0), chroma)
                                .map(|e| (e * 255.0) as u8)
                                .into_array(),
                        ),
                    )
                    .unwrap(),
                Cell::Empty => (),
            }
        }

        self
    }
}
