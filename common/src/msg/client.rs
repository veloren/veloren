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
    RequestState(ClientState),
    SetViewDistance(u32),
    Ping,
    Pong,
    Chat(String),
    PlayerActionState(comp::ActionState),
    PlayerPhysics {
        pos: comp::phys::Pos,
        vel: comp::phys::Vel,
        dir: comp::phys::Dir,
    },
    TerrainChunkRequest {
        key: Vec2<i32>,
    },
    Disconnect,
}
