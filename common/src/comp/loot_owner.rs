use crate::{
    comp::{Alignment, Body, Player},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::{
    ops::Add,
    time::{Duration, Instant},
};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct LootOwner {
    // TODO: Fix this if expiry is needed client-side, Instant is not serializable
    #[serde(skip, default = "Instant::now")]
    expiry: Instant,
    owner_uid: Uid,
}

// Loot becomes free-for-all after the initial ownership period
const OWNERSHIP_SECS: u64 = 45;

impl LootOwner {
    pub fn new(uid: Uid) -> Self {
        Self {
            expiry: Instant::now().add(Duration::from_secs(OWNERSHIP_SECS)),
            owner_uid: uid,
        }
    }

    pub fn uid(&self) -> Uid { self.owner_uid }

    pub fn time_until_expiration(&self) -> Duration { self.expiry - Instant::now() }

    pub fn expired(&self) -> bool { self.expiry <= Instant::now() }

    pub fn default_instant() -> Instant { Instant::now() }

    pub fn can_pickup(
        &self,
        uid: Uid,
        alignment: Option<&Alignment>,
        body: Option<&Body>,
        player: Option<&Player>,
    ) -> bool {
        let is_owned = matches!(alignment, Some(Alignment::Owned(_)));
        let is_player = player.is_some();
        let is_pet = is_owned && !is_player;

        let owns_loot = self.uid().0 == uid.0;
        let is_humanoid = matches!(body, Some(Body::Humanoid(_)));

        // Pet's can't pick up owned loot
        // Humanoids must own the loot
        // Non-humanoids ignore loot ownership
        !is_pet && (owns_loot || !is_humanoid)
    }
}

impl Component for LootOwner {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
