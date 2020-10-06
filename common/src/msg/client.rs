use crate::{
    character::CharacterId,
    comp,
    comp::{Skill, SkillGroupType},
    terrain::block::Block,
};
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClientType {
    // Regular Client like Voxygen who plays the game
    Game,
    // A Chatonly client, which doesn't want to connect via its character
    ChatOnly,
    // A unprivileged bot, e.g. to request world information
    // Or a privileged bot, e.g. to run admin commands used by server-cli
    Bot { privileged: bool },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientRegisterMsg {
    pub token_or_username: String,
}

//messages send by clients only valid when in character screen
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientCharacterScreenMsg {
    RequestCharacterList,
    CreateCharacter {
        alias: String,
        tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(CharacterId),
    Character(CharacterId),
    Spectate,
}

//messages send by clients only valid when in game (with a character)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientInGameMsg {
    ControllerInputs(comp::ControllerInputs),
    ControlEvent(comp::ControlEvent),
    ControlAction(comp::ControlAction),
    SetViewDistance(u32),
    BreakBlock(Vec3<i32>),
    PlaceBlock(Vec3<i32>, Block),
    ExitInGame,
    PlayerPhysics {
        pos: comp::Pos,
        vel: comp::Vel,
        ori: comp::Ori,
    },
    TerrainChunkRequest {
        key: Vec2<i32>,
    },
    UnlockSkill(Skill),
    RefundSkill(Skill),
    UnlockSkillGroup(SkillGroupType),
}

/// Messages sent from the client to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientGeneralMsg {
    ChatMsg(String),
    Disconnect,
    Terminate,
}
