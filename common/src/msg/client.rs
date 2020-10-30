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
#[allow(clippy::clippy::large_enum_variant)]
pub enum ClientMsg {
    ///Send on the first connection ONCE to identify client intention for
    /// server
    Type(ClientType),
    ///Send ONCE to register/auth to the server
    Register(ClientRegister),
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

/// Messages sent from the client to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientGeneral {
    //Only in Character Screen
    RequestCharacterList,
    CreateCharacter {
        alias: String,
        tool: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(CharacterId),
    Character(CharacterId),
    Spectate,
    //Only in game
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
    //Always possible
    ChatMsg(String),
    Terminate,
}

impl ClientMsg {
    pub fn verify(
        &self,
        c_type: ClientType,
        registered: bool,
        presence: Option<super::PresenceKind>,
    ) -> bool {
        match self {
            ClientMsg::Type(t) => c_type == *t,
            ClientMsg::Register(_) => !registered && presence.is_none(),
            ClientMsg::General(g) => {
                registered
                    && match g {
                        ClientGeneral::RequestCharacterList
                        | ClientGeneral::CreateCharacter { .. }
                        | ClientGeneral::DeleteCharacter(_) => {
                            c_type != ClientType::ChatOnly && presence.is_none()
                        },
                        ClientGeneral::Character(_) | ClientGeneral::Spectate => {
                            c_type == ClientType::Game && presence.is_none()
                        },
                        //Only in game
                        ClientGeneral::ControllerInputs(_)
                        | ClientGeneral::ControlEvent(_)
                        | ClientGeneral::ControlAction(_)
                        | ClientGeneral::SetViewDistance(_)
                        | ClientGeneral::BreakBlock(_)
                        | ClientGeneral::PlaceBlock(_, _)
                        | ClientGeneral::ExitInGame
                        | ClientGeneral::PlayerPhysics { .. }
                        | ClientGeneral::TerrainChunkRequest { .. }
                        | ClientGeneral::UnlockSkill(_)
                        | ClientGeneral::RefundSkill(_)
                        | ClientGeneral::UnlockSkillGroup(_) => {
                            c_type == ClientType::Game && presence.is_some()
                        },
                        //Always possible
                        ClientGeneral::ChatMsg(_) | ClientGeneral::Terminate => true,
                    }
            },
            ClientMsg::Ping(_) => true,
        }
    }
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

impl Into<ClientMsg> for ClientGeneral {
    fn into(self) -> ClientMsg { ClientMsg::General(self) }
}

impl Into<ClientMsg> for PingMsg {
    fn into(self) -> ClientMsg { ClientMsg::Ping(self) }
}
