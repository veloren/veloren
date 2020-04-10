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
        bound_offset: Vec2<i32>,
        center_offset: Vec2<i32>,
        z: i32,
        branch: &Branch<Self::Attr>,
    ) -> Option<Option<Block>> {
        let profile = Vec2::new(bound_offset.x, z);

        let make_block = |r, g, b| {
            Some(Some(Block::new(BlockKind::Normal, Rgb::new(r, g, b))))
        };

        let foundation = make_block(100, 100, 100);
        let log = make_block(60, 45, 30);
        let wall = make_block(75, 100, 125);
        let roof = make_block(150, 120, 50);
        let empty = Some(Some(Block::empty()));

        let width = 3 + branch.locus;
        let rampart_width = 5 + branch.locus;
        let roof_height = 12 + width;
        let ceil_height = 16;

        if profile.y <= 1 - (dist - width - 1).max(0) && dist < width + 3 { // Foundations
            foundation
        } else if profile.y == ceil_height && dist < rampart_width {
            roof
        } else if dist == rampart_width && profile.y >= ceil_height && profile.y < ceil_height + 4 {
            wall
        } else if dist == width && profile.y <= ceil_height {
            wall
        } else {
            empty
        }
    }
}
