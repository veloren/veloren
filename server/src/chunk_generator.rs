use crate::metrics::ChunkGenMetrics;
#[cfg(not(feature = "worldgen"))]
use crate::test_world::{IndexOwned, World};
use common::{generation::ChunkSupplement, terrain::TerrainChunk};
use hashbrown::{hash_map::Entry, HashMap};
use specs::Entity as EcsEntity;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::runtime::Runtime;
use vek::*;
#[cfg(feature = "worldgen")]
use world::{IndexOwned, World};

type ChunkGenResult = (
    Vec2<i32>,
    Result<(TerrainChunk, ChunkSupplement), Option<EcsEntity>>,
);

pub struct ChunkGenerator {
    chunk_tx: crossbeam_channel::Sender<ChunkGenResult>,
    chunk_rx: crossbeam_channel::Receiver<ChunkGenResult>,
    pending_chunks: HashMap<Vec2<i32>, Arc<AtomicBool>>,
    metrics: Arc<ChunkGenMetrics>,
}
impl ChunkGenerator {
    #[allow(clippy::new_without_default)] // TODO: Pending review in #587
    pub fn new(metrics: ChunkGenMetrics) -> Self {
        let (chunk_tx, chunk_rx) = crossbeam_channel::unbounded();
        Self {
            chunk_tx,
            chunk_rx,
            pending_chunks: HashMap::new(),
            metrics: Arc::new(metrics),
        }
    }

    pub fn generate_chunk(
        &mut self,
        entity: Option<EcsEntity>,
        key: Vec2<i32>,
        runtime: &mut Arc<Runtime>,
        world: Arc<World>,
        index: IndexOwned,
    ) {
        let v = if let Entry::Vacant(v) = self.pending_chunks.entry(key) {
            v
        } else {
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        v.insert(Arc::clone(&cancel));
        let chunk_tx = self.chunk_tx.clone();
        self.metrics.chunks_requested.inc();
        runtime.spawn_blocking(move || {
            let index = index.as_index_ref();
            let payload = world
                .generate_chunk(index, key, || cancel.load(Ordering::Relaxed))
                .map_err(|_| entity);
            let _ = chunk_tx.send((key, payload));
        });
    }

    pub fn recv_new_chunk(&mut self) -> Option<ChunkGenResult> {
        if let Ok((key, res)) = self.chunk_rx.try_recv() {
            self.pending_chunks.remove(&key);
            self.metrics.chunks_served.inc();
            // TODO: do anything else if res is an Err?
            Some((key, res))
        } else {
            None
        }
    }

    pub fn pending_chunks(&self) -> impl Iterator<Item = Vec2<i32>> + '_ {
        self.pending_chunks.keys().copied()
    }

    pub fn cancel_if_pending(&mut self, key: Vec2<i32>) {
        if let Some(cancel) = self.pending_chunks.remove(&key) {
            cancel.store(true, Ordering::Relaxed);
            self.metrics.chunks_canceled.inc();
        }
    }

    pub fn cancel_all(&mut self) {
        let metrics = Arc::clone(&self.metrics);
        self.pending_chunks.drain().for_each(|(_, cancel)| {
            cancel.store(true, Ordering::Relaxed);
            metrics.chunks_canceled.inc();
        });
    }
}
