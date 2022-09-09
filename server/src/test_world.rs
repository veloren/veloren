use common::{
    calendar::Calendar,
    generation::{ChunkSupplement, EntityInfo},
    resources::TimeOfDay,
    terrain::{
        Block, BlockKind, MapSizeLg, SpriteKind, TerrainChunk, TerrainChunkMeta, TerrainChunkSize,
    },
    vol::{ReadVol, RectVolSize, WriteVol},
};
use rand::{prelude::*, rngs::SmallRng};
use std::time::Duration;
use vek::*;

const DEFAULT_WORLD_CHUNKS_LG: MapSizeLg =
    if let Ok(map_size_lg) = MapSizeLg::new(Vec2 { x: 1, y: 1 }) {
        map_size_lg
    } else {
        panic!("Default world chunk size does not satisfy required invariants.");
    };

pub struct World;

#[derive(Clone)]
pub struct IndexOwned;

#[derive(Clone, Copy)]
pub struct IndexRef<'a>(&'a IndexOwned);

impl IndexOwned {
    pub fn reload_if_changed<R>(&mut self, _reload: impl FnOnce(&mut Self) -> R) -> Option<R> {
        None
    }

    pub fn as_index_ref(&self) -> IndexRef { IndexRef(self) }
}

impl World {
    pub fn generate(_seed: u32) -> (Self, IndexOwned) { (Self, IndexOwned) }

    pub fn tick(&self, dt: Duration) {}

    #[inline(always)]
    pub const fn map_size_lg(&self) -> MapSizeLg { DEFAULT_WORLD_CHUNKS_LG }

    pub fn generate_oob_chunk(&self) -> TerrainChunk { TerrainChunk::water(0) }

    pub fn generate_chunk(
        &self,
        _index: IndexRef,
        chunk_pos: Vec2<i32>,
        _should_continue: impl FnMut() -> bool,
        _time: Option<(TimeOfDay, Calendar)>,
    ) -> Result<(TerrainChunk, ChunkSupplement), ()> {
        let (x, y) = chunk_pos.map(|e| e.to_le_bytes()).into_tuple();
        let mut rng = SmallRng::from_seed([
            x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], x[0], x[1], x[2], x[3], y[0], y[1],
            y[2], y[3], x[0], x[1], x[2], x[3], y[0], y[1], y[2], y[3], x[0], x[1], x[2], x[3],
            y[0], y[1], y[2], y[3],
        ]);
        let height = rng.gen::<i32>() % 8;

        let mut supplement = ChunkSupplement::default();

        Ok((
            TerrainChunk::new(
                256 + if rng.gen::<u8>() < 64 { height } else { 0 },
                Block::new(BlockKind::Grass, Rgb::new(11, 102, 35)),
                Block::air(SpriteKind::Empty),
                TerrainChunkMeta::void(),
            ),
            supplement,
        ))
    }
}
