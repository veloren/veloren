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
    // TODO add more advanced recoloring and/or indexed based coloring
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

/// A `Segment` builder that combines segments
pub struct SegmentUnionizer(Vec<(Segment, Vec3<i32>)>);

impl SegmentUnionizer {
    pub fn new() -> Self {
        SegmentUnionizer(Vec::new())
    }
    pub fn add(mut self, segment: Segment, offset: Vec3<i32>) -> Self {
        self.0.push((segment, offset));
        self
    }
    pub fn maybe_add(self, maybe: Option<(Segment, Vec3<i32>)>) -> Self {
        match maybe {
            Some((segment, offset)) => self.add(segment, offset),
            None => self,
        }
    }
    pub fn unify(self) -> (Segment, Vec3<i32>) {
        if self.0.is_empty() {
            return (
                Segment::filled(Vec3::new(0, 0, 0), Cell::empty(), ()),
                Vec3::new(0, 0, 0),
            );
        }

        // Determine size of the new segment
        let mut min_point = self.0[0].1;
        let mut max_point = self.0[0].1 + self.0[0].0.get_size().map(|e| e as i32);
        for (segment, offset) in self.0.iter().skip(1) {
            let size = segment.get_size().map(|e| e as i32);
            min_point = min_point.map2(*offset, std::cmp::min);
            max_point = max_point.map2(offset + size, std::cmp::max);
        }
        let new_size = (max_point - min_point).map(|e| e as u32);
        // Allocate new segment
        let mut combined = Segment::filled(new_size, Cell::empty(), ());
        // Copy segments into combined
        let origin = min_point.map(|e| e * -1);
        for (segment, offset) in self.0 {
            for pos in segment.iter_positions() {
                if let Cell::Filled(col) = *segment.get(pos).unwrap() {
                    combined
                        .set(origin + offset + pos, Cell::Filled(col))
                        .unwrap();
                }
            }
        }

        (combined, origin)
    }
}
