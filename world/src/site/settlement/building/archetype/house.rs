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
        let width = 3 + branch.locus + if profile.y >= ceil_height { 1 } else { 0 };
        let foundation_height = 1 - (dist - width - 1).max(0);
        let roof_height = 8 + width;

        if center_offset.map(|e| e.abs()).reduce_max() == 0 && profile.y > foundation_height + 1 { // Chimney shaft
            empty
        } else if center_offset.map(|e| e.abs()).reduce_max() <= 1 && profile.y < roof_height + 2 { // Chimney
            if center_offset.product() == 0 && profile.y > foundation_height + 1 && profile.y <= foundation_height + 3 { // Fireplace
                empty
            } else {
                foundation
            }
        } else if profile.y <= foundation_height && dist < width + 3 { // Foundations
            if dist == width - 1 { // Floor lining
                log
            } else if dist < width - 1 && profile.y == foundation_height { // Floor
                floor
            } else if dist < width && profile.y >= foundation_height - 3 { // Basement
                empty
            } else {
                foundation
            }
        } else if profile.y > roof_height - profile.x { // Air above roof
            Some(None)
        } else if profile.y == roof_height - profile.x
            && profile.y >= ceil_height
            && dist <= width + 2
        { // Roof
            if profile.x == 0 || dist == width + 2 || profile.x.abs() % 3 == 0 { // Eaves
                log
            } else {
                roof
            }
        } else if dist == width { // Wall
            if bound_offset.x == bound_offset.y || profile.y == ceil_height || bound_offset.x == 0 {
                log
            } else if profile.x >= 2 && profile.x <= width - 2 && profile.y >= foundation_height + 2 && profile.y <= foundation_height + 3 { // Windows
                empty
            } else {
                wall
            }
        } else if dist < width { // Internals
            if profile.y == ceil_height {
                if profile.x == 0 {// Rafters
                    log
                } else { // Ceiling
                    floor
                }
            } else {
                empty
            }
        } else {
            None
        }
    }
}
