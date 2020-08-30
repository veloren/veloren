use common::{
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
}

impl BlocksOfInterest {
    pub fn from_chunk(chunk: &TerrainChunk) -> Self {
        let mut leaves = Vec::new();
        let mut grass = Vec::new();
        let mut embers = Vec::new();
        let mut beehives = Vec::new();

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
                if block.kind() == BlockKind::Leaves && thread_rng().gen_range(0, 16) == 0 {
                    leaves.push(pos);
                } else if block.kind() == BlockKind::Grass && thread_rng().gen_range(0, 16) == 0 {
                    grass.push(pos);
                } else if block.kind() == BlockKind::Ember {
                    embers.push(pos);
                } else if block.kind() == BlockKind::Beehive {
                    beehives.push(pos);
                }
            });

        Self {
            leaves,
            grass,
            embers,
            beehives,
        }
    }
}
