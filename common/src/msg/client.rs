use vek::*;
use crate::comp;

#[derive(Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Connect {
        player: comp::Player,
        character: Option<comp::Character>,
    },
    Ping,
    Pong,
    Chat(String),
    PlayerPhysics {
        pos: comp::phys::Pos,
        vel: comp::phys::Vel,
        dir: comp::phys::Dir,
    },
    TerrainChunkRequest {
        key: Vec3<i32>,
    },
    Disconnect,
}
