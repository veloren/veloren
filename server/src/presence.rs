use common_net::msg::PresenceKind;
use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Presence {
    pub view_distance: u32,
    pub kind: PresenceKind,
}

impl Presence {
    pub fn new(view_distance: u32, kind: PresenceKind) -> Self {
        Self {
            view_distance,
            kind,
        }
    }
}

impl Component for Presence {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
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
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
