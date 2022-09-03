use super::Actor;
pub use common::rtsim::FactionId;
use common::{store::Id, uid::Uid};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::ops::{Deref, DerefMut};
use vek::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Faction {
    pub leader: Option<Actor>,
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
