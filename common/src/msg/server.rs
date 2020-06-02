use super::{ClientState, EcsCompPacket};
use crate::{
    character::CharacterItem,
    comp, state, sync,
    sync::Uid,
    terrain::{Block, TerrainChunk},
};
use authc::AuthClientError;
use hashbrown::HashMap;
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub description: String,
    pub git_hash: String,
    pub git_date: String,
    pub auth_provider: Option<String>,
}

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
    pub player_alias: String,
    pub character: Option<CharacterInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub name: String,
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    WaypointSaved,
}

/// Messages sent from the server to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    InitialSync {
        entity_package: sync::EntityPackage<EcsCompPacket>,
        server_info: ServerInfo,
        time_of_day: state::TimeOfDay,
        world_map: (Vec2<u32>, Vec<u32>),
    },
    /// An error occurred while loading character data
    CharacterDataLoadError(String),
    /// A list of characters belonging to the a authenticated player was sent
    CharacterListUpdate(Vec<CharacterItem>),
    /// An error occured while creating or deleting a character
    CharacterActionError(String),
    PlayerListUpdate(PlayerListUpdate),
    StateAnswer(Result<ClientState, (RequestStateError, ClientState)>),
    /// Trigger cleanup for when the client goes back to the `Registered` state
    /// from an ingame state
    ExitIngameCleanup,
    Ping,
    Pong,
    /// A message to go into the client chat box. The client is responsible for
    /// formatting the message.
    ChatMsg(comp::ChatMsg),
    SetPlayerEntity(Uid),
    TimeOfDay(state::TimeOfDay),
    EntitySync(sync::EntitySyncPackage),
    CompSync(sync::CompSyncPackage<EcsCompPacket>),
    CreateEntity(sync::EntityPackage<EcsCompPacket>),
    DeleteEntity(Uid),
    InventoryUpdate(comp::Inventory, comp::InventoryUpdateEvent),
    TerrainChunkUpdate {
        key: Vec2<i32>,
        chunk: Result<Box<TerrainChunk>, ()>,
    },
    TerrainBlockUpdates(HashMap<Vec3<i32>, Block>),
    Disconnect,
    Shutdown,
    TooManyPlayers,
    /// Send a popup notification such as "Waypoint Saved"
    Notification(Notification),
    SetViewDistance(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStateError {
    RegisterDenied(RegisterError),
    Denied,
    Already,
    Impossible,
    WrongMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegisterError {
    AlreadyLoggedIn,
    AuthError(String),
    InvalidCharacter,
    NotOnWhitelist,
    //TODO: InvalidAlias,
}

impl From<AuthClientError> for RegisterError {
    fn from(err: AuthClientError) -> Self { Self::AuthError(err.to_string()) }
}

impl ServerMsg {
    /// Sends either say, world, group, etc. based on the player's current chat
    /// mode.
    pub fn chat(mode: comp::ChatMode, uid: sync::Uid, message: String) -> ServerMsg {
        ServerMsg::ChatMsg(mode.msg_from(uid, message))
    }

    pub fn broadcast(message: String) -> ServerMsg {
        ServerMsg::ChatMsg(comp::ChatMsg {
            chat_type: comp::ChatType::Broadcast,
            message,
        })
    }

    pub fn private(message: String) -> ServerMsg {
        ServerMsg::ChatMsg(comp::ChatMsg {
            chat_type: comp::ChatType::Private,
            message,
        })
    }

    pub fn kill(message: String) -> ServerMsg {
        ServerMsg::ChatMsg(comp::ChatMsg {
            chat_type: comp::ChatType::Kill,
            message,
        })
    }
}
