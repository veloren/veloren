use super::{world_msg::SiteId, PingMsg};
use common::{character::CharacterId, comp, comp::Skill, terrain::block::Block, ViewDistances};
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
    ///Msg that can be send ALWAYS as soon as we are registered, e.g. `Chat`
    General(ClientGeneral),
    Ping(PingMsg),
}

/*
2nd Level Enums
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientType {
    /// Regular Client like Voxygen who plays the game
    Game,
    /// A Chat-only client, which doesn't want to connect via its character
    ChatOnly,
    /// A unprivileged bot, e.g. to request world information
    /// Or a privileged bot, e.g. to run admin commands used by server-cli
    Bot { privileged: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        mainhand: Option<String>,
        offhand: Option<String>,
        body: comp::Body,
    },
    DeleteCharacter(CharacterId),
    EditCharacter {
        id: CharacterId,
        alias: String,
        body: comp::Body,
    },
    Character(CharacterId, ViewDistances),
    Spectate(ViewDistances),
    //Only in game
    ControllerInputs(Box<comp::ControllerInputs>),
    ControlEvent(comp::ControlEvent),
    ControlAction(comp::ControlAction),
    SetViewDistance(ViewDistances),
    BreakBlock(Vec3<i32>),
    PlaceBlock(Vec3<i32>, Block),
    ExitInGame,
    PlayerPhysics {
        pos: comp::Pos,
        vel: comp::Vel,
        ori: comp::Ori,
        force_counter: u64,
    },
    UnlockSkill(Skill),
    RequestSiteInfo(SiteId),
    UpdateMapMarker(comp::MapMarkerChange),

    SpectatePosition(Vec3<f32>),
    //Only in Game, via terrain stream
    TerrainChunkRequest {
        key: Vec2<i32>,
    },
    LodZoneRequest {
        key: Vec2<i32>,
    },
    //Always possible
    ChatMsg(String),
    Command(String, Vec<String>),
    Terminate,
    RequestPlayerPhysics {
        server_authoritative: bool,
    },
    RequestLossyTerrainCompression {
        lossy_terrain_compression: bool,
    },
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
                        | ClientGeneral::EditCharacter { .. }
                        | ClientGeneral::DeleteCharacter(_) => {
                            c_type != ClientType::ChatOnly && presence.is_none()
                        },
                        ClientGeneral::Character(_, _) | ClientGeneral::Spectate(_) => {
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
                        | ClientGeneral::LodZoneRequest { .. }
                        | ClientGeneral::UnlockSkill(_)
                        | ClientGeneral::RequestSiteInfo(_)
                        | ClientGeneral::RequestPlayerPhysics { .. }
                        | ClientGeneral::RequestLossyTerrainCompression { .. }
                        | ClientGeneral::UpdateMapMarker(_)
                        | ClientGeneral::SpectatePosition(_) => {
                            c_type == ClientType::Game && presence.is_some()
                        },
                        //Always possible
                        ClientGeneral::ChatMsg(_)
                        | ClientGeneral::Command(_, _)
                        | ClientGeneral::Terminate => true,
                    }
            },
            ClientMsg::Ping(_) => true,
        }
    }
}

/*
end of 2nd level Enums
*/

impl From<ClientType> for ClientMsg {
    fn from(other: ClientType) -> ClientMsg { ClientMsg::Type(other) }
}

impl From<ClientRegister> for ClientMsg {
    fn from(other: ClientRegister) -> ClientMsg { ClientMsg::Register(other) }
}

impl From<ClientGeneral> for ClientMsg {
    fn from(other: ClientGeneral) -> ClientMsg { ClientMsg::General(other) }
}

impl From<PingMsg> for ClientMsg {
    fn from(other: PingMsg) -> ClientMsg { ClientMsg::Ping(other) }
}
