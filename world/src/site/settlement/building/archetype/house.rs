use vek::*;
use rand::prelude::*;
use common::{
    terrain::{Block, BlockKind},
    vol::Vox,
};
use crate::util::{RandomField, Sampler};
use super::{
    Archetype,
    super::skeleton::*,
};

pub struct House {
    roof_color: Rgb<u8>,
    noise: RandomField,
    roof_ribbing: bool,
    central_supports: bool,
    chimney: Option<i32>,
}

impl Archetype for House {
    type Attr = ();

    fn generate<R: Rng>(rng: &mut R) -> Self {
        Self {
            roof_color: Rgb::new(
                rng.gen_range(50, 200),
                rng.gen_range(50, 200),
                rng.gen_range(50, 200),
            ),
            noise: RandomField::new(rng.gen()),
            roof_ribbing: rng.gen(),
            central_supports: rng.gen(),
            chimney: if rng.gen() { Some(rng.gen_range(1, 6)) } else { None },
        }
    }

    fn draw(
        &self,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        branch: &Branch<Self::Attr>,
    ) -> Option<Option<Block>> {
        let profile = Vec2::new(bound_offset.x, z);

        let make_block = |r, g, b| {
            let nz = self.noise.get(Vec3::new(center_offset.x, center_offset.y, z * 8));
            Some(Some(Block::new(BlockKind::Normal, Rgb::new(r, g, b) + (nz & 0x0F) as u8 - 8)))
        };

        let foundation = make_block(100, 100, 100);
        let log = make_block(60, 45, 30);
        let floor = make_block(100, 75, 50);
        let wall = make_block(200, 180, 150);
        let roof = make_block(self.roof_color.r, self.roof_color.g, self.roof_color.b);
        let empty = Some(Some(Block::empty()));

        let ceil_height = 6;
        let width = -3 + branch.locus + if profile.y >= ceil_height { 1 } else { 0 };
        let foundation_height = 0 - (dist - width - 1).max(0);
        let roof_height = 8 + width;

        if let Some(chimney_height) = self.chimney {
            // Chimney shaft
            if center_offset.map(|e| e.abs()).reduce_max() == 0 && profile.y > foundation_height + 1 {
                return empty;
            }

            // Chimney
            if center_offset.map(|e| e.abs()).reduce_max() <= 1 && profile.y < roof_height + chimney_height {
                // Fireplace
                if center_offset.product() == 0 && profile.y > foundation_height + 1 && profile.y <= foundation_height + 3 {
                    return empty;
                } else {
                    return foundation;
                }
            }
        }

        if profile.y <= foundation_height && dist < width + 3 { // Foundations
            if dist == width - 1 { // Floor lining
                return log;
            } else if dist < width - 1 && profile.y == foundation_height { // Floor
                return floor;
            } else if dist < width && profile.y >= foundation_height - 3 { // Basement
                return empty;
            } else {
                return foundation;
            }
        }

        if profile.y > roof_height - profile.x { // Air above roof
            return Some(None);
        }

        // Roof
        if profile.y == roof_height - profile.x
            && profile.y >= ceil_height
            && dist <= width + 2
        {
            let is_ribbing = (roof_height - profile.y) % 3 == 0 && self.roof_ribbing;
            if profile.x == 0 || dist == width + 2 || is_ribbing { // Eaves
                return log;
            } else {
                return roof;
            }
        }

        // Walls
        if dist == width {
            let frame_bounds = if profile.y >= ceil_height {
                Aabr {
                    min: Vec2::new(-1, ceil_height + 2),
                    max: Vec2::new(1, ceil_height + 5),
                }
            } else {
                Aabr {
                    min: Vec2::new(2, foundation_height + 2),
                    max: Vec2::new(width - 2, ceil_height - 2),
                }
            };
            let window_bounds = Aabr {
                min: (frame_bounds.min + 1).map2(frame_bounds.center(), |a, b| a.min(b)),
                max: (frame_bounds.max - 1).map2(frame_bounds.center(), |a, b| a.max(b)),
            };

            // Window
            if (frame_bounds.size() + 1).reduce_min() > 2 { // Window frame is large enough for a window
                let surface_pos = Vec2::new(bound_offset.x, profile.y);
                if window_bounds.contains_point(surface_pos) {
                    return empty;
                } else if frame_bounds.contains_point(surface_pos) {
                    return log;
                };
            }

            // Wall
            return if
                bound_offset.x == bound_offset.y ||
                (profile.x == 0 && self.central_supports) ||
                profile.y == ceil_height
            { // Support beams
                log
            } else {
                wall
            };
        }

        if dist < width { // Internals
            return if profile.y == ceil_height {
                if profile.x == 0 {// Rafters
                    log
                } else { // Ceiling
                    floor
                }
            } else {
                empty
            };
        }

        None
    }
}
