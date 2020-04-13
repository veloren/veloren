use vek::*;
use rand::prelude::*;
use common::{
    terrain::{Block, BlockKind},
    vol::Vox,
};
use crate::util::{RandomField, Sampler};
use super::{
    Archetype,
    BlockMask,
    super::skeleton::*,
};

pub struct House {
    roof_color: Rgb<u8>,
    noise: RandomField,
    chimney: Option<i32>,
    roof_ribbing: bool,
}

enum RoofStyle {
    Hip,
    Gable,
    Rounded,
}

enum StoreyFill {
    None,
    Upper,
    All,
}

impl StoreyFill {
    fn has_lower(&self) -> bool { !if let StoreyFill::None = self { true } else { false } }
    fn has_upper(&self) -> bool { if let StoreyFill::None = self { false } else { true } }
}

pub struct Attr {
    central_supports: bool,
    storey_fill: StoreyFill,
    roof_style: RoofStyle,
    mansard: i32,
}

impl Attr {
    fn generate<R: Rng>(rng: &mut R) -> Self {
        Self {
            central_supports: rng.gen(),
            storey_fill: match rng.gen_range(0, 2) {
                //0 => StoreyFill::None,
                0 => StoreyFill::Upper,
                _ => StoreyFill::All,
            },
            roof_style: match rng.gen_range(0, 3) {
                0 => RoofStyle::Hip,
                1 => RoofStyle::Gable,
                _ => RoofStyle::Rounded,
            },
            mansard: rng.gen_range(-8, 6).max(0),
        }
    }
}

impl Archetype for House {
    type Attr = Attr;

    fn generate<R: Rng>(rng: &mut R) -> (Self, Skeleton<Self::Attr>) {
        let len = rng.gen_range(-8, 24).clamped(0, 20);
        let locus = 6 + rng.gen_range(0, 5);
        let branches_per_side = 1 + len as usize / 20;
        let skel = Skeleton {
            offset: -rng.gen_range(0, len + 7).clamped(0, len),
            ori: if rng.gen() { Ori::East } else { Ori::North },
            root: Branch {
                len,
                attr: Attr {
                    storey_fill: StoreyFill::All,
                    mansard: 0,
                    ..Attr::generate(rng)
                },
                locus,
                border: 4,
                children: [1, -1]
                    .iter()
                    .map(|flip| (0..branches_per_side).map(move |i| (i, *flip)))
                    .flatten()
                    .filter_map(|(i, flip)| if rng.gen() {
                        Some((i as i32 * len / (branches_per_side - 1).max(1) as i32, Branch {
                            len: rng.gen_range(5, 16) * flip,
                            attr: Attr::generate(rng),
                            locus: (6 + rng.gen_range(0, 3)).min(locus),
                            border: 4,
                            children: Vec::new(),
                        }))
                    } else {
                        None
                    })
                    .collect(),
            },
        };

        let this = Self {
            roof_color: Rgb::new(
                rng.gen_range(50, 200),
                rng.gen_range(50, 200),
                rng.gen_range(50, 200),
            ),
            noise: RandomField::new(rng.gen()),
            chimney: if rng.gen() { Some(8 + skel.root.locus + rng.gen_range(1, 5)) } else { None },
            roof_ribbing: rng.gen(),
        };

        (this, skel)
    }

    fn draw(
        &self,
        dist: i32,
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        branch: &Branch<Self::Attr>,
    ) -> BlockMask {
        let profile = Vec2::new(bound_offset.x, z);

        let make_block = |r, g, b| {
            let nz = self.noise.get(Vec3::new(center_offset.x, center_offset.y, z * 8));
            BlockMask::new(Block::new(BlockKind::Normal, Rgb::new(r, g, b) + (nz & 0x0F) as u8 - 8), 2)
        };

        let foundation = make_block(100, 100, 100);
        let log = make_block(60, 45, 30);
        let floor = make_block(100, 75, 50).with_priority(3);
        let wall = make_block(200, 180, 150);
        let roof = make_block(self.roof_color.r, self.roof_color.g, self.roof_color.b);
        let empty = BlockMask::nothing();
        let internal = BlockMask::new(Block::empty(), 4);
        let fire = BlockMask::new(Block::new(BlockKind::Ember, Rgb::white()), 2);

        let ceil_height = 6;
        let lower_width = branch.locus - 1;
        let upper_width = branch.locus;
        let width = if profile.y >= ceil_height { upper_width } else { lower_width };
        let foundation_height = 0 - (dist - width - 1).max(0);
        let roof_top = 8 + width;

        if let Some(chimney_top) = self.chimney {
            // Chimney shaft
            if center_offset.map(|e| e.abs()).reduce_max() == 0 && profile.y >= foundation_height + 1 {
                return if profile.y == foundation_height + 1 {
                    fire
                } else {
                    internal
                };
            }

            // Chimney
            if center_offset.map(|e| e.abs()).reduce_max() <= 1 && profile.y < chimney_top {
                // Fireplace
                if center_offset.product() == 0 && profile.y > foundation_height + 1 && profile.y <= foundation_height + 3 {
                    return internal;
                } else {
                    return foundation;
                }
            }
        }

        if profile.y <= foundation_height && dist < width + 3 { // Foundations
            if branch.attr.storey_fill.has_lower() {
                if dist == width - 1 { // Floor lining
                    return log;
                } else if dist < width - 1 && profile.y == foundation_height { // Floor
                    return floor;
                }
            }

            if dist < width && profile.y < foundation_height && profile.y >= foundation_height - 3 { // Basement
                return internal;
            } else {
                return foundation.with_priority(1);
            }
        }

        let do_roof = |profile: Vec2<i32>, dist, roof_top, roof_width, mansard| {
            if profile.y > roof_top - profile.x.max(mansard) && profile.y >= roof_top - roof_width { // Air above roof
                return Some(empty);
            }

            // Roof
            if profile.y == roof_top - profile.x.max(mansard)
                && dist <= roof_width
            {
                let is_ribbing = (profile.y - ceil_height) % 3 == 0 && self.roof_ribbing;
                if (profile.x == 0 && mansard == 0) || dist == roof_width|| is_ribbing { // Eaves
                    return Some(log.with_priority(1));
                } else {
                    return Some(roof.with_priority(1));
                }
            }

            None
        };

        if let Some(block) = match &branch.attr.roof_style {
            RoofStyle::Hip => do_roof(Vec2::new(dist, profile.y), dist, roof_top, width + 2, branch.attr.mansard),
            RoofStyle::Gable => do_roof(profile, dist, roof_top, width + 2, branch.attr.mansard),
            RoofStyle::Rounded => {
                let circular_dist = (bound_offset.map(|e| e.pow(4) as f32).sum().powf(0.25) + 0.5).ceil() as i32;
                do_roof(Vec2::new(circular_dist, profile.y), circular_dist, roof_top, width + 2, branch.attr.mansard)
            },
        } {
            return block;
        }

        // Walls
        if dist == width {
            if bound_offset.x == bound_offset.y || profile.y == ceil_height { // Support beams
                return log;
            } else if !branch.attr.storey_fill.has_lower() && profile.y < ceil_height {
                return empty;
            } else if !branch.attr.storey_fill.has_upper() {
                return empty;
            } else {
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
                        return internal;
                    } else if frame_bounds.contains_point(surface_pos) {
                        return log.with_priority(3);
                    };
                }

                // Wall
                return if branch.attr.central_supports && profile.x == 0 { // Support beams
                    log.with_priority(4)
                } else {
                    wall
                };
            }
        }

        if dist < width { // Internals
            if profile.y == ceil_height {
                if profile.x == 0 {// Rafters
                    return log;
                } else if branch.attr.storey_fill.has_upper() { // Ceiling
                    return floor;
                }
            } else if (!branch.attr.storey_fill.has_lower() && profile.y < ceil_height)
                || (!branch.attr.storey_fill.has_upper() && profile.y >= ceil_height)
            {
                return empty;
            } else {
                return internal;
            }
        }

        empty
    }
}
