use crate::client::PreparedMsg;
use specs::{Component, Entity};
use specs_idvs::IdvStorage;
use vek::Vec2;

/// Curing the runtime of a tick, multiple systems can request a chunk to be
/// synced to a client E.g. msg::terrain will do so, when a client requested a
/// chunk that already exist terrain will do so when a chunk came back from
/// ChunkGeneration. All those sends are deferred by this queue.
/// Deferring allows us to remove code duplication and maybe serialize ONCE,
/// send to MULTIPLE clients TODO: store a urgent flag and seperate even more, 5
/// ticks vs 5 seconds
#[derive(Default, Clone, Debug, PartialEq)]
pub struct ChunkSendQueue {
    pub chunks: Vec<Vec2<i32>>,
}

impl Component for ChunkSendQueue {
    type Storage = IdvStorage<Self>;
}

pub struct SerializedChunk {
    pub(crate) lossy_compression: bool,
    pub(crate) msg: PreparedMsg,
    pub(crate) recipients: Vec<Entity>,
}
