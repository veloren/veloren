use vek::*;
use crate::comp;

#[derive(Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Connect {
        player: comp::Player,
        
    },
    Character {
        character: comp::Character,
    },
    Ping,
    Pong,
    Chat(String),
    PlayerAnimation(comp::character::AnimationHistory),
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
