use common::msg::{ClientInGame, ClientType};
use hashbrown::HashSet;
use network::{Participant, Stream};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

// Streams
// we ignore errors on send, and do unified error handling in recv
pub struct GeneralStream(pub Stream);
pub struct PingStream(pub Stream);
pub struct RegisterStream(pub Stream);
pub struct CharacterScreenStream(pub Stream);
pub struct InGameStream(pub Stream);

impl Component for GeneralStream {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
impl Component for PingStream {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
impl Component for RegisterStream {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
impl Component for CharacterScreenStream {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
impl Component for InGameStream {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

pub struct Client {
    pub registered: bool,
    pub client_type: ClientType,
    pub in_game: Option<ClientInGame>,
    pub participant: Option<Participant>,
    pub last_ping: f64,
    pub login_msg_sent: bool,
}

impl Component for Client {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

// Distance from fuzzy_chunk before snapping to current chunk
pub const CHUNK_FUZZ: u32 = 2;
// Distance out of the range of a region before removing it from subscriptions
pub const REGION_FUZZ: u32 = 16;

#[derive(Clone, Debug)]
pub struct RegionSubscription {
    pub fuzzy_chunk: Vec2<i32>,
    pub regions: HashSet<Vec2<i32>>,
}

impl Component for RegionSubscription {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
