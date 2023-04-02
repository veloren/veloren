pub use common::rtsim::{Actor, FactionId};
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::ops::{Deref, DerefMut};
use vek::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Faction {
    pub leader: Option<Actor>,
    pub good_or_evil: bool, // TODO: Very stupid, get rid of this
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Factions {
    pub factions: HopSlotMap<FactionId, Faction>,
}

impl Factions {
    pub fn create(&mut self, faction: Faction) -> FactionId { self.factions.insert(faction) }
}

impl Deref for Factions {
    type Target = HopSlotMap<FactionId, Faction>;

    fn deref(&self) -> &Self::Target { &self.factions }
}

impl DerefMut for Factions {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.factions }
}
