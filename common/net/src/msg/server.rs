use super::{
    world_msg::EconomyInfo, ClientType, CompressedData, EcsCompPacket, PingMsg, QuadPngEncoding,
    TriPngEncoding, WidePacking, WireChonk,
};
use crate::sync;
use common::{
    calendar::Calendar,
    character::{self, CharacterItem},
    comp::{self, invite::InviteKind, item::MaterialStatManifest},
    event::UpdateCharacterMetadata,
    lod,
    outcome::Outcome,
    recipe::{ComponentRecipeBook, RecipeBook},
    resources::{Time, TimeOfDay},
    shared_server_config::ServerConstants,
    terrain::{Block, TerrainChunk, TerrainChunkMeta, TerrainChunkSize},
    trade::{PendingTrade, SitePrices, TradeId, TradeResult},
    uid::Uid,
    uuid::Uuid,
    weather::WeatherGrid,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::warn;
use vek::*;

///This struct contains all messages the server might send (on different
/// streams though)
#[derive(Debug, Clone)]
pub enum ServerMsg {
    /// Basic info about server, send ONCE, clients need it to Register
    Info(ServerInfo),
    /// Initial data package, send BEFORE Register ONCE. Not Register relevant
    Init(Box<ServerInit>),
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
#[allow(clippy::large_enum_variant)]
pub enum ServerInit {
    GameSync {
        entity_package: sync::EntityPackage<EcsCompPacket>,
        time_of_day: TimeOfDay,
        max_group_size: u32,
        client_timeout: Duration,
        world_map: crate::msg::world_msg::WorldMapMsg,
        recipe_book: RecipeBook,
        component_recipe_book: ComponentRecipeBook,
        material_stats: MaterialStatManifest,
        ability_map: comp::item::tool::AbilityMap,
        server_constants: ServerConstants,
    },
}

pub type ServerRegisterAnswer = Result<(), RegisterError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SerializedTerrainChunk {
    DeflatedChonk(CompressedData<TerrainChunk>),
    QuadPng(WireChonk<QuadPngEncoding<4>, WidePacking<true>, TerrainChunkMeta, TerrainChunkSize>),
    TriPng(WireChonk<TriPngEncoding<false>, WidePacking<true>, TerrainChunkMeta, TerrainChunkSize>),
}

impl SerializedTerrainChunk {
    pub fn approx_len(&self) -> usize {
        match self {
            SerializedTerrainChunk::DeflatedChonk(data) => data.data.len(),
            SerializedTerrainChunk::QuadPng(data) => data.data.data.len(),
            SerializedTerrainChunk::TriPng(data) => data.data.data.len(),
        }
    }

    pub fn via_heuristic(chunk: &TerrainChunk, lossy_compression: bool) -> Self {
        if lossy_compression && (chunk.get_max_z() - chunk.get_min_z() <= 128) {
            Self::quadpng(chunk)
        } else {
            Self::deflate(chunk)
        }
    }

    pub fn deflate(chunk: &TerrainChunk) -> Self {
        Self::DeflatedChonk(CompressedData::compress(chunk, 1))
    }

    pub fn quadpng(chunk: &TerrainChunk) -> Self {
        if let Some(wc) = WireChonk::from_chonk(QuadPngEncoding(), WidePacking(), chunk) {
            Self::QuadPng(wc)
        } else {
            warn!("Image encoding failure occurred, falling back to deflate");
            Self::deflate(chunk)
        }
    }

    pub fn tripng(chunk: &TerrainChunk) -> Self {
        if let Some(wc) = WireChonk::from_chonk(TriPngEncoding(), WidePacking(), chunk) {
            Self::TriPng(wc)
        } else {
            warn!("Image encoding failure occurred, falling back to deflate");
            Self::deflate(chunk)
        }
    }

    pub fn to_chunk(&self) -> Option<TerrainChunk> {
        match self {
            Self::DeflatedChonk(chonk) => chonk.decompress(),
            Self::QuadPng(wc) => wc.to_chonk(),
            Self::TriPng(wc) => wc.to_chonk(),
        }
    }
}

/// Messages sent from the server to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerGeneral {
    //Character Screen related
    /// Result of loading character data
    CharacterDataLoadResult(Result<UpdateCharacterMetadata, String>),
    /// A list of characters belonging to the a authenticated player was sent
    CharacterListUpdate(Vec<CharacterItem>),
    /// An error occurred while creating or deleting a character
    CharacterActionError(String),
    /// A new character was created
    CharacterCreated(character::CharacterId),
    CharacterEdited(character::CharacterId),
    CharacterSuccess,
    SpectatorSuccess(Vec3<f32>),
    //Ingame related
    GroupUpdate(comp::group::ChangeNotification<Uid>),
    /// Indicate to the client that they are invited to join a group
    Invite {
        inviter: Uid,
        timeout: Duration,
        kind: InviteKind,
    },
    /// Indicate to the client that their sent invite was not invalid and is
    /// currently pending
    InvitePending(Uid),
    /// Update the HUD of the clients in the group
    GroupInventoryUpdate(comp::Item, String, Uid),
    /// Note: this could potentially include all the failure cases such as
    /// inviting yourself in which case the `InvitePending` message could be
    /// removed and the client could consider their invite pending until
    /// they receive this message Indicate to the client the result of their
    /// invite
    InviteComplete {
        target: Uid,
        answer: InviteAnswer,
        kind: InviteKind,
    },
    /// Trigger cleanup for when the client goes back to the `Registered` state
    /// from an ingame state
    ExitInGameSuccess,
    InventoryUpdate(comp::Inventory, comp::InventoryUpdateEvent),
    /// NOTE: The client can infer that entity view distance will be at most the
    /// terrain view distance that we send here (and if lower it won't be
    /// modified). So we just need to send the terrain VD back to the client
    /// if corrections are made.
    SetViewDistance(u32),
    Outcomes(Vec<Outcome>),
    Knockback(Vec3<f32>),
    // Ingame related AND terrain stream
    TerrainChunkUpdate {
        key: Vec2<i32>,
        chunk: Result<SerializedTerrainChunk, ()>,
    },
    LodZoneUpdate {
        key: Vec2<i32>,
        zone: lod::Zone,
    },
    TerrainBlockUpdates(CompressedData<HashMap<Vec3<i32>, Block>>),
    // Always possible
    PlayerListUpdate(PlayerListUpdate),
    /// A message to go into the client chat box. The client is responsible for
    /// formatting the message and turning it into a speech bubble.
    ChatMsg(comp::ChatMsg),
    ChatMode(comp::ChatMode),
    SetPlayerEntity(Uid),
    TimeOfDay(TimeOfDay, Calendar, Time),
    EntitySync(sync::EntitySyncPackage),
    CompSync(sync::CompSyncPackage<EcsCompPacket>, u64),
    CreateEntity(sync::EntityPackage<EcsCompPacket>),
    DeleteEntity(Uid),
    Disconnect(DisconnectReason),
    /// Send a popup notification such as "Waypoint Saved"
    Notification(Notification),
    UpdatePendingTrade(TradeId, PendingTrade, Option<SitePrices>),
    FinishedTrade(TradeResult),
    /// Economic information about sites
    SiteEconomy(EconomyInfo),
    MapMarker(comp::MapMarkerUpdate),
    WeatherUpdate(WeatherGrid),
    /// Suggest the client to spectate a position. Called after client has
    /// requested teleport etc.
    SpectatePosition(Vec3<f32>),
}

impl ServerGeneral {
    pub fn server_msg<S>(chat_type: comp::ChatType<String>, msg: S) -> Self
    where
        S: Into<String>,
    {
        ServerGeneral::ChatMsg(chat_type.chat_msg(msg))
    }
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
    Moderator(Uid, bool),
    Remove(Uid),
    Alias(Uid, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub is_moderator: bool,
    pub is_online: bool,
    pub player_alias: String,
    pub character: Option<CharacterInfo>,
    pub uuid: Uuid,
}

/// used for localisation, filled by client and used by i18n code
pub struct ChatTypeContext {
    pub you: Uid,
    pub player_alias: HashMap<Uid, PlayerInfo>,
    pub entity_name: HashMap<Uid, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub name: String,
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
    /// Client was kicked
    Kicked(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegisterError {
    AuthError(String),
    Banned(String),
    Kicked(String),
    InvalidCharacter,
    NotOnWhitelist,
    TooManyPlayers,
    //TODO: InvalidAlias,
}

impl ServerMsg {
    pub fn verify(
        &self,
        c_type: ClientType,
        registered: bool,
        presence: Option<super::PresenceKind>,
    ) -> bool {
        match self {
            ServerMsg::Info(_) | ServerMsg::Init(_) | ServerMsg::RegisterAnswer(_) => {
                !registered && presence.is_none()
            },
            ServerMsg::General(g) => {
                registered
                    && match g {
                        //Character Screen related
                        ServerGeneral::CharacterDataLoadResult(_)
                        | ServerGeneral::CharacterListUpdate(_)
                        | ServerGeneral::CharacterActionError(_)
                        | ServerGeneral::CharacterEdited(_)
                        | ServerGeneral::CharacterCreated(_) => {
                            c_type != ClientType::ChatOnly && presence.is_none()
                        },
                        ServerGeneral::CharacterSuccess | ServerGeneral::SpectatorSuccess(_) => {
                            c_type == ClientType::Game && presence.is_none()
                        },
                        //Ingame related
                        ServerGeneral::GroupUpdate(_)
                        | ServerGeneral::Invite { .. }
                        | ServerGeneral::InvitePending(_)
                        | ServerGeneral::InviteComplete { .. }
                        | ServerGeneral::ExitInGameSuccess
                        | ServerGeneral::InventoryUpdate(_, _)
                        | ServerGeneral::GroupInventoryUpdate(_, _, _)
                        | ServerGeneral::TerrainChunkUpdate { .. }
                        | ServerGeneral::LodZoneUpdate { .. }
                        | ServerGeneral::TerrainBlockUpdates(_)
                        | ServerGeneral::SetViewDistance(_)
                        | ServerGeneral::Outcomes(_)
                        | ServerGeneral::Knockback(_)
                        | ServerGeneral::UpdatePendingTrade(_, _, _)
                        | ServerGeneral::FinishedTrade(_)
                        | ServerGeneral::SiteEconomy(_)
                        | ServerGeneral::MapMarker(_)
                        | ServerGeneral::WeatherUpdate(_)
                        | ServerGeneral::SpectatePosition(_) => {
                            c_type == ClientType::Game && presence.is_some()
                        },
                        // Always possible
                        ServerGeneral::PlayerListUpdate(_)
                        | ServerGeneral::ChatMsg(_)
                        | ServerGeneral::ChatMode(_)
                        | ServerGeneral::SetPlayerEntity(_)
                        | ServerGeneral::TimeOfDay(_, _, _)
                        | ServerGeneral::EntitySync(_)
                        | ServerGeneral::CompSync(_, _)
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

impl From<comp::ChatMsg> for ServerGeneral {
    fn from(v: comp::ChatMsg) -> Self { ServerGeneral::ChatMsg(v) }
}

impl From<ServerInfo> for ServerMsg {
    fn from(o: ServerInfo) -> ServerMsg { ServerMsg::Info(o) }
}

impl From<ServerInit> for ServerMsg {
    fn from(o: ServerInit) -> ServerMsg { ServerMsg::Init(Box::new(o)) }
}

impl From<ServerRegisterAnswer> for ServerMsg {
    fn from(o: ServerRegisterAnswer) -> ServerMsg { ServerMsg::RegisterAnswer(o) }
}

impl From<ServerGeneral> for ServerMsg {
    fn from(o: ServerGeneral) -> ServerMsg { ServerMsg::General(o) }
}

impl From<PingMsg> for ServerMsg {
    fn from(o: PingMsg) -> ServerMsg { ServerMsg::Ping(o) }
}
