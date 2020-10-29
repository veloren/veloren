use common::{
    span,
    terrain::{BlockKind, SpriteKind, TerrainChunk},
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
    // Note: these are only needed for chunks within the iteraction range so this is a potential
    // area for optimization
    pub interactables: Vec<Vec3<i32>>,
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
        let mut interactables = Vec::new();

        chunk
            .vol_iter(
                Vec3::new(0, 0, chunk.get_min_z()),
                Vec3::new(
                    TerrainChunk::RECT_SIZE.x as i32,
                    TerrainChunk::RECT_SIZE.y as i32,
                    chunk.get_max_z(),
                ),
            )
            .for_each(|(pos, block)| {
                match block.kind() {
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
                    _ => match block.get_sprite() {
                        Some(SpriteKind::Ember) => embers.push(pos),
                        Some(SpriteKind::Beehive) => beehives.push(pos),
                        Some(SpriteKind::Reed) => reeds.push(pos),
                        Some(SpriteKind::PinkFlower) => flowers.push(pos),
                        Some(SpriteKind::PurpleFlower) => flowers.push(pos),
                        Some(SpriteKind::RedFlower) => flowers.push(pos),
                        Some(SpriteKind::WhiteFlower) => flowers.push(pos),
                        Some(SpriteKind::YellowFlower) => flowers.push(pos),
                        Some(SpriteKind::Sunflower) => flowers.push(pos),
                        _ => {},
                    },
                }
                if block.is_collectible() {
                    interactables.push(pos);
                }
            });

        Self {
            leaves,
            grass,
            embers,
            beehives,
            reeds,
            flowers,
            interactables,
        }
    }
}
