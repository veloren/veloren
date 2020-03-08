use crate::{comp, terrain::block::Block};
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Register {
        player: comp::Player,
        password: String,
    },
    Character {
        name: String,
        body: comp::Body,
        main: Option<String>, // Specifier for the weapon
    },
    /// Request `ClientState::Registered` from an ingame state
    ExitIngame,
    /// Request `ClientState::Spectator` from a registered or ingame state
    Spectate,
    ControllerInputs(comp::ControllerInputs),
    ControlEvent(comp::ControlEvent),
    SetViewDistance(u32),
    BreakBlock(Vec3<i32>),
    PlaceBlock(Vec3<i32>, Block),
    Ping,
    Pong,
    ChatMsg {
        message: String,
    },
    PlayerPhysics {
        pos: comp::Pos,
        vel: comp::Vel,
        ori: comp::Ori,
    },
    TerrainChunkRequest {
        key: Vec2<i32>,
    },
    Disconnect,
    Terminate,
}
