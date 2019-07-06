use super::ClientState;
use crate::comp;
use crate::terrain::block::Block;
use vek::*;
use fxhash::FxHashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Register {
        player: comp::Player,
    },
    Character {
        name: String,
        body: comp::Body,
    },
    Controller(comp::Controller),
    RequestState(ClientState),
    SetViewDistance(u32),
    BlockChanges(FxHashMap<Vec3<i32>, Block>),
    Ping,
    Pong,
    Chat(String),
    PlayerPhysics {
        pos: comp::Pos,
        vel: comp::Vel,
        ori: comp::Ori,
    },
    TerrainChunkRequest {
        key: Vec2<i32>,
    },
    Disconnect,
}
