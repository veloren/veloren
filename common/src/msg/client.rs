use super::PingMsg;
use crate::{
    character::CharacterId,
    comp,
    comp::{Skill, SkillGroupType},
    terrain::block::Block,
};
use serde::{Deserialize, Serialize};
use vek::*;

///This struct contains all messages the client might send (on different
/// streams though). It's used to verify the correctness of the state in
/// debug_assertions
#[derive(Debug, Clone)]
pub enum ClientMsg {
    ///Send on the first connection ONCE to identify client intention for
    /// server
    Type(ClientType),
    ///Send ONCE to register/auth to the server
    Register(ClientRegister),
    ///Msg only to send while in character screen, e.g. `CreateCharacter`
    CharacterScreen(ClientCharacterScreen),
    ///Msg only to send while playing in game, e.g. `PlayerPositionUpdates`
    InGame(ClientInGame),
    ///Msg that can be send ALWAYS as soon as we are registered, e.g. `Chat`
    General(ClientGeneral),
    Ping(PingMsg),
}

/*
2nd Level Enums
*/

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ClientType {
    /// Regular Client like Voxygen who plays the game
    Game,
    /// A Chatonly client, which doesn't want to connect via its character
    ChatOnly,
    /// A unprivileged bot, e.g. to request world information
    /// Or a privileged bot, e.g. to run admin commands used by server-cli
    Bot { privileged: bool },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientRegister {
    pub token_or_username: String,
}

//messages send by clients only valid when in character screen
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientCharacterScreen {
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
pub enum ClientInGame {
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
pub enum ClientGeneral {
    ChatMsg(String),
    Disconnect,
    Terminate,
}

/*
end of 2nd level Enums
*/

impl Into<ClientMsg> for ClientType {
    fn into(self) -> ClientMsg { ClientMsg::Type(self) }
}

impl Into<ClientMsg> for ClientRegister {
    fn into(self) -> ClientMsg { ClientMsg::Register(self) }
}

impl Into<ClientMsg> for ClientCharacterScreen {
    fn into(self) -> ClientMsg { ClientMsg::CharacterScreen(self) }
}

impl Into<ClientMsg> for ClientInGame {
    fn into(self) -> ClientMsg { ClientMsg::InGame(self) }
}

impl Into<ClientMsg> for ClientGeneral {
    fn into(self) -> ClientMsg { ClientMsg::General(self) }
}

impl Into<ClientMsg> for PingMsg {
    fn into(self) -> ClientMsg { ClientMsg::Ping(self) }
}
