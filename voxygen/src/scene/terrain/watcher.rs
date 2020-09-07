use common::{
    span,
    terrain::{BlockKind, TerrainChunk},
    vol::{IntoVolIterator, RectRasterableVol},
};
use rand::prelude::*;
use vek::*;

pub struct BlocksOfInterest {
    pub leaves: Vec<Vec3<i32>>,
    pub grass: Vec<Vec3<i32>>,
    pub embers: Vec<Vec3<i32>>,
    pub beehives: Vec<Vec3<i32>>,
    pub reeds: Vec<Vec3<i32>>,
    pub flowers: Vec<Vec3<i32>>,
}

impl BlocksOfInterest {
    pub fn from_chunk(chunk: &TerrainChunk) -> Self {
        span!(_guard, "from_chunk", "BlocksOfInterest::from_chunk");
        let mut leaves = Vec::new();
        let mut grass = Vec::new();
        let mut embers = Vec::new();
        let mut beehives = Vec::new();
        let mut reeds = Vec::new();
        let mut flowers = Vec::new();

        chunk
            .vol_iter(
                Vec3::new(0, 0, chunk.get_min_z()),
                Vec3::new(
                    TerrainChunk::RECT_SIZE.x as i32,
                    TerrainChunk::RECT_SIZE.y as i32,
                    chunk.get_max_z(),
                ),
            )
            .for_each(|(pos, block)| match block.kind() {
                BlockKind::Leaves => {
                    if thread_rng().gen_range(0, 16) == 0 {
                        leaves.push(pos)
                    }
                },
                BlockKind::Grass => {
                    if thread_rng().gen_range(0, 16) == 0 {
                        grass.push(pos)
                    }
                },
                BlockKind::Ember => embers.push(pos),
                BlockKind::Beehive => beehives.push(pos),
                BlockKind::Reed => reeds.push(pos),
                BlockKind::PinkFlower => flowers.push(pos),
                BlockKind::PurpleFlower => flowers.push(pos),
                BlockKind::RedFlower => flowers.push(pos),
                BlockKind::WhiteFlower => flowers.push(pos),
                BlockKind::YellowFlower => flowers.push(pos),
                BlockKind::Sunflower => flowers.push(pos),
                _ => {},
            });

        Self {
            leaves,
            grass,
            embers,
            beehives,
            reeds,
            flowers,
        }
    }
}
