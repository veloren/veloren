use super::{ClientType, EcsCompPacket, PingMsg};
use crate::{
    character::CharacterItem,
    comp,
    outcome::Outcome,
    recipe::RecipeBook,
    state, sync,
    sync::Uid,
    terrain::{Block, TerrainChunk},
};
use authc::AuthClientError;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::*;

///This struct contains all messages the server might send (on different
/// streams though)
#[derive(Debug, Clone)]
pub enum ServerMsg {
    /// Basic info about server, send ONCE, clients need it to Register
    Info(ServerInfo),
    /// Initial data package, send BEFORE Register ONCE. Not Register relevant
    Init(ServerInit),
    /// Result to `ClientMsg::Register`. send ONCE
    RegisterAnswer(ServerRegisterAnswer),
    ///Msg that can be send ALWAYS as soon as client is registered, e.g. `Chat`
    General(ServerGeneral),
    Ping(PingMsg),
}

/*
2nd Level Enums
*/

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub description: String,
    pub git_hash: String,
    pub git_date: String,
    pub auth_provider: Option<String>,
}

/// Reponse To ClientType
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::clippy::large_enum_variant)]
pub enum ServerInit {
    TooManyPlayers,
    GameSync {
        entity_package: sync::EntityPackage<EcsCompPacket>,
        time_of_day: state::TimeOfDay,
        max_group_size: u32,
        client_timeout: Duration,
        world_map: crate::msg::world_packet::WorldMapMsg,
        recipe_book: RecipeBook,
    },
}

pub type ServerRegisterAnswer = Result<(), RegisterError>;

/// Messages sent from the server to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerGeneral {
    //Character Screen related
    /// An error occurred while loading character data
    CharacterDataLoadError(String),
    /// A list of characters belonging to the a authenticated player was sent
    CharacterListUpdate(Vec<CharacterItem>),
    /// An error occurred while creating or deleting a character
    CharacterActionError(String),
    CharacterSuccess,
    //Ingame related
    GroupUpdate(comp::group::ChangeNotification<sync::Uid>),
    /// Indicate to the client that they are invited to join a group
    GroupInvite {
        inviter: sync::Uid,
        timeout: std::time::Duration,
    },
    /// Indicate to the client that their sent invite was not invalid and is
    /// currently pending
    InvitePending(sync::Uid),
    /// Note: this could potentially include all the failure cases such as
    /// inviting yourself in which case the `InvitePending` message could be
    /// removed and the client could consider their invite pending until
    /// they receive this message Indicate to the client the result of their
    /// invite
    InviteComplete {
        target: sync::Uid,
        answer: InviteAnswer,
    },
    /// Trigger cleanup for when the client goes back to the `Registered` state
    /// from an ingame state
    ExitInGameSuccess,
    InventoryUpdate(comp::Inventory, comp::InventoryUpdateEvent),
    TerrainChunkUpdate {
        key: Vec2<i32>,
        chunk: Result<Box<TerrainChunk>, ()>,
    },
    TerrainBlockUpdates(HashMap<Vec3<i32>, Block>),
    SetViewDistance(u32),
    Outcomes(Vec<Outcome>),
    Knockback(Vec3<f32>),
    // Always possible
    PlayerListUpdate(PlayerListUpdate),
    /// A message to go into the client chat box. The client is responsible for
    /// formatting the message and turning it into a speech bubble.
    ChatMsg(comp::ChatMsg),
    SetPlayerEntity(Uid),
    TimeOfDay(state::TimeOfDay),
    EntitySync(sync::EntitySyncPackage),
    CompSync(sync::CompSyncPackage<EcsCompPacket>),
    CreateEntity(sync::EntityPackage<EcsCompPacket>),
    DeleteEntity(Uid),
    Disconnect(DisconnectReason),
    /// Send a popup notification such as "Waypoint Saved"
    Notification(Notification),
}

/*
end of 2nd level Enums
*/

/// Inform the client of updates to the player list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerListUpdate {
    Init(HashMap<Uid, PlayerInfo>),
    Add(Uid, PlayerInfo),
    SelectedCharacter(Uid, CharacterInfo),
    LevelChange(Uid, u32),
    Admin(Uid, bool),
    Remove(Uid),
    Alias(Uid, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub is_admin: bool,
    pub is_online: bool,
    pub player_alias: String,
    pub character: Option<CharacterInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub name: String,
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InviteAnswer {
    Accepted,
    Declined,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    WaypointSaved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisconnectReason {
    /// Server shut down
    Shutdown,
    /// Client sent disconnect message
    Requested,
    /// Client was kicked
    Kicked(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegisterError {
    AlreadyLoggedIn,
    AuthError(String),
    Banned(String),
    InvalidCharacter,
    NotOnWhitelist,
    //TODO: InvalidAlias,
}

impl ServerMsg {
    pub fn verify(
        &self,
        c_type: ClientType,
        registered: bool,
        in_game: Option<super::ClientInGame>,
    ) -> bool {
        match self {
            ServerMsg::Info(_) | ServerMsg::Init(_) | ServerMsg::RegisterAnswer(_) => {
                !registered && in_game.is_none()
            },
            ServerMsg::General(g) => {
                registered
                    && match g {
                        //Character Screen related
                        ServerGeneral::CharacterDataLoadError(_)
                        | ServerGeneral::CharacterListUpdate(_)
                        | ServerGeneral::CharacterActionError(_) => {
                            c_type != ClientType::ChatOnly && in_game.is_none()
                        },
                        ServerGeneral::CharacterSuccess => {
                            c_type == ClientType::Game && in_game.is_none()
                        },
                        //Ingame related
                        ServerGeneral::GroupUpdate(_)
                        | ServerGeneral::GroupInvite { .. }
                        | ServerGeneral::InvitePending(_)
                        | ServerGeneral::InviteComplete { .. }
                        | ServerGeneral::ExitInGameSuccess
                        | ServerGeneral::InventoryUpdate(_, _)
                        | ServerGeneral::TerrainChunkUpdate { .. }
                        | ServerGeneral::TerrainBlockUpdates(_)
                        | ServerGeneral::SetViewDistance(_)
                        | ServerGeneral::Outcomes(_)
                        | ServerGeneral::Knockback(_) => {
                            c_type == ClientType::Game && in_game.is_some()
                        },
                        // Always possible
                        ServerGeneral::PlayerListUpdate(_)
                        | ServerGeneral::ChatMsg(_)
                        | ServerGeneral::SetPlayerEntity(_)
                        | ServerGeneral::TimeOfDay(_)
                        | ServerGeneral::EntitySync(_)
                        | ServerGeneral::CompSync(_)
                        | ServerGeneral::CreateEntity(_)
                        | ServerGeneral::DeleteEntity(_)
                        | ServerGeneral::Disconnect(_)
                        | ServerGeneral::Notification(_) => true,
                    }
            },
            ServerMsg::Ping(_) => true,
        }
    }
}

impl From<AuthClientError> for RegisterError {
    fn from(err: AuthClientError) -> Self { Self::AuthError(err.to_string()) }
}

impl From<comp::ChatMsg> for ServerGeneral {
    fn from(v: comp::ChatMsg) -> Self { ServerGeneral::ChatMsg(v) }
}

impl Into<ServerMsg> for ServerInfo {
    fn into(self) -> ServerMsg { ServerMsg::Info(self) }
}

impl Into<ServerMsg> for ServerInit {
    fn into(self) -> ServerMsg { ServerMsg::Init(self) }
}

impl Into<ServerMsg> for ServerRegisterAnswer {
    fn into(self) -> ServerMsg { ServerMsg::RegisterAnswer(self) }
}

impl Into<ServerMsg> for ServerGeneral {
    fn into(self) -> ServerMsg { ServerMsg::General(self) }
}

impl Into<ServerMsg> for PingMsg {
    fn into(self) -> ServerMsg { ServerMsg::Ping(self) }
}
