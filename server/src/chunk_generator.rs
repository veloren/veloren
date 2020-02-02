#[cfg(not(feature = "worldgen"))]
use crate::test_world::World;
use common::{generation::ChunkSupplement, terrain::TerrainChunk};
use crossbeam::channel;
use hashbrown::{hash_map::Entry, HashMap};
use specs::Entity as EcsEntity;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use vek::*;
#[cfg(feature = "worldgen")] use world::World;

type ChunkGenResult = (
    Vec2<i32>,
    Result<(TerrainChunk, ChunkSupplement), EcsEntity>,
);

pub struct ChunkGenerator {
    chunk_tx: channel::Sender<ChunkGenResult>,
    chunk_rx: channel::Receiver<ChunkGenResult>,
    pending_chunks: HashMap<Vec2<i32>, Arc<AtomicBool>>,
}
impl ChunkGenerator {
    pub fn new() -> Self {
        let (chunk_tx, chunk_rx) = channel::unbounded();
        Self {
            chunk_tx,
            chunk_rx,
            pending_chunks: HashMap::new(),
        }
    }

    pub fn generate_chunk(
        &mut self,
        entity: EcsEntity,
        key: Vec2<i32>,
        thread_pool: &mut uvth::ThreadPool,
        world: Arc<World>,
    ) {
        let v = if let Entry::Vacant(v) = self.pending_chunks.entry(key) {
            v
        } else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        v.insert(Arc::clone(&cancel));
        let chunk_tx = self.chunk_tx.clone();
        thread_pool.execute(move || {
            let payload = world
                .generate_chunk(key, || cancel.load(Ordering::Relaxed))
                .map_err(|_| entity);
            let _ = chunk_tx.send((key, payload));
        });
    }

    pub fn recv_new_chunk(&mut self) -> Option<ChunkGenResult> {
        if let Ok((key, res)) = self.chunk_rx.try_recv() {
            self.pending_chunks.remove(&key);
            // TODO: do anything else if res is an Err?
            Some((key, res))
        } else {
            None
        }
    }

    pub fn pending_chunks<'a>(&'a self) -> impl Iterator<Item = Vec2<i32>> + 'a {
        self.pending_chunks.keys().copied()
    }

    pub fn cancel_if_pending(&mut self, key: Vec2<i32>) {
        if let Some(cancel) = self.pending_chunks.remove(&key) {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}
