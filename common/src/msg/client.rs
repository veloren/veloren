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
    Attack,
    Roll,
    Cidle,
    Respawn,
    RequestState(ClientState),
    SetViewDistance(u32),
    Ping,
    Pong,
    Chat(String),
    PlayerAnimation(comp::AnimationInfo),
    PlayerPhysics {
        pos: comp::phys::Pos,
        vel: comp::phys::Vel,
        ori: comp::phys::Ori,
    },
    TerrainChunkRequest {
        key: Vec2<i32>,
    },
    Disconnect,
}
