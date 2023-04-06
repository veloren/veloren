use crate::data::Sentiments;
use common::rtsim::Actor;
pub use common::rtsim::FactionId;
use serde::{Deserialize, Serialize};
use slotmap::HopSlotMap;
use std::ops::{Deref, DerefMut};
use vek::*;

#[derive(Clone, Serialize, Deserialize)]
pub struct Faction {
    pub seed: u32,
    pub leader: Option<Actor>,
    pub good_or_evil: bool, // TODO: Very stupid, get rid of this

    #[serde(default)]
    pub sentiments: Sentiments,
}

impl Faction {
    pub fn cleanup(&mut self) {
        self.sentiments
            .cleanup(crate::data::sentiment::FACTION_MAX_SENTIMENTS);
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
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
