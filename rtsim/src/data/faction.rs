use hashbrown::HashMap;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;
use vek::*;
use std::ops::{Deref, DerefMut};
use common::{
    uid::Uid,
    store::Id,
};
use super::Actor;
pub use common::rtsim::FactionId;

#[derive(Clone, Serialize, Deserialize)]
pub struct Faction {
    pub leader: Option<Actor>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Factions {
    pub factions: HopSlotMap<FactionId, Faction>,
}

impl Factions {
    pub fn create(&mut self, faction: Faction) -> FactionId {
        self.factions.insert(faction)
    }
}

impl Deref for Factions {
    type Target = HopSlotMap<FactionId, Faction>;
    fn deref(&self) -> &Self::Target { &self.factions }
}

impl DerefMut for Factions {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.factions }
}
