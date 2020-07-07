use crate::{
    comp,
    comp::{Skill, SkillGroupType},
    terrain::block::Block,
};
use serde::{Deserialize, Serialize};
use vek::*;

/// Messages sent from the client to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Register {
        view_distance: Option<u32>,
        token_or_username: String,
    },
    RequestCharacterList,
    CreateCharacter {
        alias: String,
        tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(i32),
    Character(i32),
    /// Request `ClientState::Registered` from an ingame state
    ExitIngame,
    /// Request `ClientState::Spectator` from a registered or ingame state
    Spectate,
    ControllerInputs(comp::ControllerInputs),
    ControlEvent(comp::ControlEvent),
    ControlAction(comp::ControlAction),
    SetViewDistance(u32),
    BreakBlock(Vec3<i32>),
    PlaceBlock(Vec3<i32>, Block),
    Ping,
    Pong,
    /// Send the chat message or command to be processed by the server
    ChatMsg(String),
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
    UnlockSkill(Skill),
    RefundSkill(Skill),
    UnlockSkillGroup(SkillGroupType),
}
