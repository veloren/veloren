use super::{ClientState, EcsCompPacket};
use crate::{
    character::CharacterItem,
    comp, state, sync,
    sync::Uid,
    terrain::{Block, TerrainChunk},
};
use authc::AuthClientError;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
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
/// World map information.  Note that currently, we always send the whole thing
/// in one go, but the structure aims to try to provide information as locally
/// as possible, so that in the future we can split up large maps into multiple
/// WorldMapMsg fragments.
///
/// TODO: Update message format to make fragmentable, allowing us to send more
/// information without running into bandwidth issues.
///
/// TODO: Add information for rivers (currently, we just prerender them on the
/// server, but this is not a great solution for LoD.  The map rendering code is
/// already set up to be able to take advantage of the rivrer rendering being
/// split out, but the format is a little complicated for space reasons and it
/// may take some tweaking to get right, so we avoid sending it for now).
///
/// TODO: measure explicit compression schemes that might save space, e.g.
/// repeating the "small angles" optimization that works well on more detailed
/// shadow maps intended for height maps.
pub struct WorldMapMsg {
    /// World map dimensions (width × height)
    pub dimensions: Vec2<u32>,
    /// Max height (used to scale altitudes).
    pub max_height: f32,
    /// RGB+A; the alpha channel is currently a proxy for altitude.
    /// Entries are in the usual chunk order.
    pub rgba: Vec<u32>,
    /// Horizon mapping.  This is a variant of shadow mapping that is
    /// specifically designed for height maps; it takes advantage of their
    /// regular structure (e.g. no holes) to compress all information needed
    /// to decide when to cast a sharp shadow into a single nagle, the "horizon
    /// angle."  This is the smallest angle with the ground at which light can
    /// pass through any occluders to reach the chunk, in some chosen
    /// horizontal direction.  This would not be sufficient for a more
    /// complicated 3D structure, but it works for height maps since:
    ///
    /// 1. they have no gaps, so as soon as light can shine through it will
    /// always be able to do    so, and
    /// 2. we only care about lighting from the top, and only from the east and
    /// west    (since at a large scale like this we mostly just want to
    /// handle variable sunlight;    moonlight would present more challenges
    /// but we currently have no plans to try to cast    accurate shadows in
    /// moonlight).
    ///
    /// Our chosen format is two pairs of vectors,
    /// with the first pair representing west-facing light (casting shadows on
    /// the left side)  and the second representing east-facing light
    /// (casting shadows on the east side).
    ///
    /// The pair of vectors consists of (with each vector in the usual chunk
    /// order):
    ///
    /// * Horizon angle pointing east (1 byte, scaled so 1 unit = 255° / 360).
    ///   We might consider switching to tangent if that represents the
    ///   information we care about better.
    /// * Approximate (floor) height of maximal occluder. We currently use this
    ///   to try to deliver some approximation of soft shadows, which isn't that
    ///   big a deal on the world map but is probably needed in order to ensure
    ///   smooth transitions between chunks in LoD view.  Additionally, when we
    ///   start using the shadow information to do local lighting on the world
    ///   map, we'll want a quick way to test where we can go out of shadoow at
    ///   arbitrary heights (since the player and other entities cajn find
    ///   themselves far from the ground at times).  While this is only an
    ///   approximation to a proper distance map, hopefully it will give us
    ///   something  that feels reasonable enough for Veloren's style.
    ///
    /// NOTE: On compression.
    ///
    /// Horizon mapping has a lot of advantages for height maps (simple, easy to
    /// understand, doesn't require any fancy math or approximation beyond
    /// precision loss), though it loses a few of them by having to store
    /// distance to occluder as well.  However, just storing tons
    /// and tons of regular shadow maps (153 for a full day cycle, stored at
    /// irregular intervals) combined with clever explicit compression and
    /// avoiding recording sharp local shadows (preferring retracing for
    /// these), yielded a compression rate of under 3 bits per column! Since
    /// we likely want to avoid per-column shadows for worlds of the sizes we
    /// want, we'd still need to store *some* extra information to create
    /// soft shadows, but it would still be nice to try to drive down our
    /// size as much as possible given how compressible shadows of height
    /// maps seem to be in practice.  Therefore, we try to take advantage of the
    /// way existing compression algorithms tend to work to see if we can
    /// achieve significant gains without doing a lot of custom work.
    ///
    /// Specifically, since our rays are cast east/west, we expect that for each
    /// row, the horizon angles in each direction should be sequences of
    /// monotonically increasing values (as chunks approach a tall
    /// occluder), followed by sequences of no shadow, repeated
    /// until the end of the map.  Monotonic sequences and same-byte sequences
    /// are usually easy to compress and existing algorithms are more likely
    /// to be able to deal with them than jumbled data.  If we were to keep
    /// both directions in the same vector, off-the-shelf compression would
    /// probably be less effective.
    ///
    /// For related reasons, rather than storing distances as in a standard
    /// distance map (which would lead to monotonically *decreaing* values
    /// as we approached the occluder from a given direction), we store the
    /// estimated *occluder height.*  The idea here is that we replace the
    /// monotonic sequences with constant sequences, which are extremely
    /// straightforward to compress and mostly handled automatically by anything
    /// that does run-length encoding (i.e. most off-the-shelf compression
    /// algorithms).
    ///
    /// We still need to benchmark this properly, as there's no guarantee our
    /// current compression algorithms will actually work well on this data
    /// in practice.  It's possible that some other permutation (e.g. more
    /// bits reserved for "distance to occluder" in exchange for an even
    /// more predictible sequence) would end up compressing better than storing
    /// angles, or that we don't need as much precision as we currently have
    /// (256 possible angles).
    pub horizons: [(Vec<u8>, Vec<u8>); 2],
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
        world_map: WorldMapMsg,
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
    /// formatting the message and turning it into a speech bubble.
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

impl From<comp::ChatMsg> for ServerMsg {
    fn from(v: comp::ChatMsg) -> Self { ServerMsg::ChatMsg(v) }
}
