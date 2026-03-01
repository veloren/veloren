use hashbrown::HashSet;
use serde::{Deserialize, Serialize};
use specs::{Component, VecStorage};
use vek::*;

// Distance from fuzzy_chunk before snapping to current chunk
pub const CHUNK_FUZZ: u32 = 2;
// Distance out of the range of a region before removing it from subscriptions
pub const REGION_FUZZ: u32 = 16;

#[derive(Clone, Debug)]
pub struct RegionSubscription {
    pub fuzzy_chunk: Vec2<i32>,
    pub last_entity_view_distance: u32,
    pub regions: HashSet<Vec2<i32>>,
}

impl Component for RegionSubscription {
    type Storage = specs::DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct RepositionToFreeSpace {
    pub needs_ground: bool,
}

impl Component for RepositionToFreeSpace {
    type Storage = VecStorage<Self>;
}
