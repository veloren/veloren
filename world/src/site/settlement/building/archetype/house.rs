use vek::*;
use rand::prelude::*;
use common::{
    terrain::{Block, BlockKind},
    vol::Vox,
};
use super::{
    Archetype,
    super::skeleton::*,
};

pub struct House {
    roof_color: Rgb<u8>,
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
        }
    }

    fn draw(
        &self,
        dist: i32,
        offset: Vec2<i32>,
        z: i32,
        branch: &Branch<Self::Attr>,
    ) -> Option<Block> {
        let profile = Vec2::new(offset.x, z);

        let foundation = Block::new(BlockKind::Normal, Rgb::new(100, 100, 100));
        let log = Block::new(BlockKind::Normal, Rgb::new(60, 45, 30));
        let floor = Block::new(BlockKind::Normal, Rgb::new(100, 75, 50));
        let wall = Block::new(BlockKind::Normal, Rgb::new(200, 180, 150));
        let roof = Block::new(BlockKind::Normal, self.roof_color);
        let empty = Block::empty();

        let width = 3 + branch.locus;
        let roof_height = 8 + width;
        let ceil_height = 6;

        if profile.y <= 1 - (dist - width - 1).max(0) && dist < width + 3 { // Foundations
            if dist < width { // Floor
                Some(floor)
            } else {
                Some(foundation)
            }
        } else if profile.y > roof_height - profile.x { // Air above roof
            None
        } else if profile.y == roof_height - profile.x
            && profile.y >= ceil_height
            && dist <= width + 2
        { // Roof
            if profile.x == 0 || dist == width + 2 { // Eaves
                Some(log)
            } else {
                Some(roof)
            }
        } else if dist == width { // Wall
            if offset.x == offset.y || profile.y == ceil_height || offset.x == 0 {
                Some(log)
            } else {
                Some(wall)
            }
        } else if dist < width { // Internals
            if profile.y == ceil_height {
                if profile.x == 0 {// Rafters
                    Some(log)
                } else { // Ceiling
                    Some(floor)
                }
            } else {
                Some(empty)
            }
        } else {
            None
        }
    }
}
