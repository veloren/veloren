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

pub struct Keep;

impl Archetype for Keep {
    type Attr = ();

    fn generate<R: Rng>(rng: &mut R) -> Self {
        Self
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
        let wall = Block::new(BlockKind::Normal, Rgb::new(75, 100, 125));
        let roof = Block::new(BlockKind::Normal, Rgb::new(150, 120, 50));
        let empty = Block::empty();

        let width = 3 + branch.locus;
        let rampart_width = 5 + branch.locus;
        let roof_height = 12 + width;
        let ceil_height = 8;

        if profile.y <= 1 - (dist - width - 1).max(0) && dist < width + 3 { // Foundations
            Some(foundation)
        } else if profile.y == ceil_height && dist < rampart_width {
            Some(roof)
        } else if dist == rampart_width && profile.y >= ceil_height && profile.y < ceil_height + 4 {
            Some(wall)
        } else if dist == width && profile.y <= ceil_height {
            Some(wall)
        } else {
            None
        }
    }
}
