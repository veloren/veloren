use crate::{
    comp::{Alignment, Body, Group, Player},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{
    ops::Add,
    time::{Duration, Instant},
};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct LootOwner {
    // TODO: Fix this if expiry is needed client-side, Instant is not serializable
    #[serde(skip, default = "Instant::now")]
    expiry: Instant,
    owner: LootOwnerKind,
    /// This field stands as a wish for NPC's to not pick the loot up, they will
    /// however be able to decide whether they want to follow your wishes or not
    /// (players will be able to picke the item up)
    soft: bool,
}

// Loot becomes free-for-all after the initial ownership period
const OWNERSHIP_SECS: u64 = 45;

impl LootOwner {
    pub fn new(kind: LootOwnerKind, soft: bool) -> Self {
        Self {
            expiry: Instant::now().add(Duration::from_secs(OWNERSHIP_SECS)),
            owner: kind,
            soft,
        }
    }

    pub fn uid(&self) -> Option<Uid> {
        match &self.owner {
            LootOwnerKind::Player(uid) => Some(*uid),
            LootOwnerKind::Group(_) => None,
        }
    }

    pub fn owner(&self) -> LootOwnerKind { self.owner }

    pub fn time_until_expiration(&self) -> Duration { self.expiry - Instant::now() }

    pub fn expired(&self) -> bool { self.expiry <= Instant::now() }

    pub fn default_instant() -> Instant { Instant::now() }

    pub fn is_soft(&self) -> bool { self.soft }

    pub fn can_pickup(
        &self,
        uid: Uid,
        group: Option<&Group>,
        alignment: Option<&Alignment>,
        body: Option<&Body>,
        player: Option<&Player>,
    ) -> bool {
        let is_owned = matches!(alignment, Some(Alignment::Owned(_)));
        let is_player = player.is_some();
        let is_pet = is_owned && !is_player;

        let owns_loot = match self.owner {
            LootOwnerKind::Player(loot_uid) => loot_uid.0 == uid.0,
            LootOwnerKind::Group(loot_group) => {
                matches!(group, Some(group) if loot_group == *group)
            },
        };
        let is_humanoid = matches!(body, Some(Body::Humanoid(_)));

        // Pet's can't pick up owned loot
        // Humanoids must own the loot
        // Non-humanoids ignore loot ownership
        !is_pet && (self.soft || owns_loot || !is_humanoid)
    }
}

impl Component for LootOwner {
    type Storage = DerefFlaggedStorage<Self, specs::DenseVecStorage<Self>>;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LootOwnerKind {
    Player(Uid),
    Group(Group),
}
