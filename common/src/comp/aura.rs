use crate::{
    combat::GroupTarget,
    comp::buff::{BuffCategory, BuffData, BuffKind, BuffSource},
    resources::{Secs, Time},
    uid::Uid,
};
use serde::{Deserialize, Serialize};
use slotmap::{new_key_type, SlotMap};
use specs::{Component, DerefFlaggedStorage, VecStorage};

new_key_type! { pub struct AuraKey; }

/// AuraKind is what kind of effect an aura applies
/// Currently only buffs are implemented
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AuraKind {
    /// The Buff kind is (surprise!) a buff :D
    Buff {
        kind: BuffKind,
        data: BuffData,
        category: BuffCategory,
        source: BuffSource,
    },
    /* TODO: Implement other effects here. Things to think about
     * are terrain/sprite effects, collision and physics, and
     * environmental conditions like temperature and humidity
     * Multiple auras can be given to an entity. */
}

/// Aura
/// Applies a buff to entities in the radius if meeting
/// conditions set forth in the aura system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aura {
    /// The kind of aura applied
    pub aura_kind: AuraKind,
    /// The radius of the aura
    pub radius: f32,
    // None corresponds to an indefinite aura
    pub end_time: Option<Time>,
    /* TODO: Add functionality for fading or a gradient */
    /// Used to filter which entities this aura will apply to. For example,
    /// globally neutral auras which affect all entities will have the type
    /// `AuraTarget::All`. Whereas auras which only affect a player's party
    /// members will have the type `AuraTarget::GroupOf`.
    pub target: AuraTarget,
    /// Contains data about the original state of the aura that does not change
    /// over time
    pub data: AuraData,
}

/// Information about whether aura addition or removal was requested.
/// This to implement "on_add" and "on_remove" hooks for auras
#[derive(Clone, Debug)]
pub enum AuraChange {
    /// Adds this aura
    Add(Aura),
    /// Removes auras of these indices
    RemoveByKey(Vec<AuraKey>),
}

/// Used by the aura system to filter entities when applying an effect.
#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum AuraTarget {
    /// Targets the group of the entity specified by the `Uid`. This is useful
    /// for auras which should only affect a player's party.
    GroupOf(Uid),
    /// Targets everyone not in the group of the entity specified by the `Uid`.
    /// This is useful for auras which should only affect a player's
    /// enemies.
    NotGroupOf(Uid),
    /// Targets all entities. This is for auras which are global or neutral.
    All,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Specifier {
    WardingAura,
    HealingAura,
    Frozen,
    FieryAura,
}

impl From<(Option<GroupTarget>, Option<&Uid>)> for AuraTarget {
    fn from((target, uid): (Option<GroupTarget>, Option<&Uid>)) -> Self {
        match (target, uid) {
            (Some(GroupTarget::InGroup), Some(uid)) => Self::GroupOf(*uid),
            (Some(GroupTarget::OutOfGroup), Some(uid)) => Self::NotGroupOf(*uid),
            _ => Self::All,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuraData {
    pub duration: Option<Secs>,
}

impl AuraData {
    #[must_use]
    fn new(duration: Option<Secs>) -> Self { Self { duration } }
}

impl Aura {
    /// Creates a new Aura to be assigned to an entity
    pub fn new(
        aura_kind: AuraKind,
        radius: f32,
        duration: Option<Secs>,
        target: AuraTarget,
        time: Time,
    ) -> Self {
        Self {
            aura_kind,
            radius,
            end_time: duration.map(|dur| Time(time.0 + dur.0)),
            target,
            data: AuraData::new(duration),
        }
    }
}

/// Component holding all auras emitted by an entity.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Auras {
    pub auras: SlotMap<AuraKey, Aura>,
}

impl Auras {
    pub fn new(auras: Vec<Aura>) -> Self {
        let mut auras_comp: SlotMap<AuraKey, Aura> = SlotMap::with_key();
        for aura in auras {
            auras_comp.insert(aura);
        }
        Self { auras: auras_comp }
    }

    pub fn insert(&mut self, aura: Aura) { self.auras.insert(aura); }

    pub fn remove(&mut self, key: AuraKey) { self.auras.remove(key); }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AuraBuffConstructor {
    pub kind: BuffKind,
    pub strength: f32,
    pub duration: Option<Secs>,
    pub category: BuffCategory,
}

impl AuraBuffConstructor {
    pub fn to_aura(
        self,
        uid: &Uid,
        radius: f32,
        duration: Option<Secs>,
        target: AuraTarget,
        time: Time,
    ) -> Aura {
        let aura_kind = AuraKind::Buff {
            kind: self.kind,
            data: BuffData::new(self.strength, self.duration),
            category: self.category,
            source: BuffSource::Character { by: *uid },
        };
        Aura::new(aura_kind, radius, duration, target, time)
    }
}

impl Component for Auras {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}
