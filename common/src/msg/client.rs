use super::ClientState;
use crate::comp;
use vek::*;

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
