use crate::uid::Uid;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::{cmp::Ordering, collections::HashMap, time::Duration};

/// De/buff Kind.
/// This is used to determine what effects a buff will have
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum BuffKind {
    /// Restores health/time for some period
    Regeneration,
    /// Restores health/time for some period for consumables
    Saturation,
    /// Lowers health over time for some duration
    Bleeding,
    /// Lower a creature's max health
    /// Currently placeholder buff to show other stuff is possible
    Cursed,
    /// Applied when drinking a potion
    Potion,
    /// Applied when sitting at a campfire
    CampfireHeal,
}

impl BuffKind {
    // Checks if buff is buff or debuff
    pub fn is_buff(self) -> bool {
        match self {
            BuffKind::Regeneration { .. } => true,
            BuffKind::Saturation { .. } => true,
            BuffKind::Bleeding { .. } => false,
            BuffKind::Cursed { .. } => false,
            BuffKind::Potion { .. } => true,
            BuffKind::CampfireHeal { .. } => true,
        }
    }
}

// Struct used to store data relevant to a buff
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuffData {
    pub strength: f32,
    pub duration: Option<Duration>,
}

impl BuffData {
    pub fn new(strength: f32, duration: Option<Duration>) -> Self { Self { strength, duration } }
}

/// De/buff category ID.
/// Similar to `BuffKind`, but to mark a category (for more generic usage, like
/// positive/negative buffs).
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum BuffCategory {
    Natural,
    Physical,
    Magical,
    Divine,
    PersistOnDeath,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ModifierKind {
    Additive,
    Fractional,
}

/// Data indicating and configuring behaviour of a de/buff.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BuffEffect {
    /// Periodically damages or heals entity
    HealthChangeOverTime {
        rate: f32,
        accumulated: f32,
        kind: ModifierKind,
    },
    /// Changes maximum health by a certain amount
    MaxHealthModifier { value: f32, kind: ModifierKind },
}

/// Actual de/buff.
/// Buff can timeout after some time if `time` is Some. If `time` is None,
/// Buff will last indefinitely, until removed manually (by some action, like
/// uncursing).
///
/// Buff has a kind, which is used to determine the effects in a builder
/// function.
///
/// To provide more classification info when needed,
/// buff can be in one or more buff category.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Buff {
    pub kind: BuffKind,
    pub data: BuffData,
    pub cat_ids: Vec<BuffCategory>,
    pub time: Option<Duration>,
    pub effects: Vec<BuffEffect>,
    pub source: BuffSource,
}

/// Information about whether buff addition or removal was requested.
/// This to implement "on_add" and "on_remove" hooks for constant buffs.
#[derive(Clone, Debug)]
pub enum BuffChange {
    /// Adds this buff.
    Add(Buff),
    /// Removes all buffs with this ID.
    RemoveByKind(BuffKind),
    /// Removes all buffs with this ID, but not debuffs.
    RemoveFromController(BuffKind),
    /// Removes buffs of these indices (first vec is for active buffs, second is
    /// for inactive buffs), should only be called when buffs expire
    RemoveById(Vec<BuffId>),
    /// Removes buffs of these categories (first vec is of categories of which
    /// all are required, second vec is of categories of which at least one is
    /// required, third vec is of categories that will not be removed)
    RemoveByCategory {
        all_required: Vec<BuffCategory>,
        any_required: Vec<BuffCategory>,
        none_required: Vec<BuffCategory>,
    },
}

impl Buff {
    /// Builder function for buffs
    pub fn new(
        kind: BuffKind,
        data: BuffData,
        cat_ids: Vec<BuffCategory>,
        source: BuffSource,
    ) -> Self {
        let (effects, time) = match kind {
            BuffKind::Bleeding => (
                vec![BuffEffect::HealthChangeOverTime {
                    rate: -data.strength,
                    accumulated: 0.0,
                    kind: ModifierKind::Additive,
                }],
                data.duration,
            ),
            BuffKind::Regeneration | BuffKind::Saturation | BuffKind::Potion => (
                vec![BuffEffect::HealthChangeOverTime {
                    rate: data.strength,
                    accumulated: 0.0,
                    kind: ModifierKind::Additive,
                }],
                data.duration,
            ),
            BuffKind::CampfireHeal => (
                vec![BuffEffect::HealthChangeOverTime {
                    rate: data.strength,
                    accumulated: 0.0,
                    kind: ModifierKind::Fractional,
                }],
                data.duration,
            ),
            BuffKind::Cursed => (
                vec![BuffEffect::MaxHealthModifier {
                    value: -100. * data.strength,
                    kind: ModifierKind::Additive,
                }],
                data.duration,
            ),
        };
        Buff {
            kind,
            data,
            cat_ids,
            time,
            effects,
            source,
        }
    }
}

impl PartialOrd for Buff {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self.data.strength > other.data.strength {
            Some(Ordering::Greater)
        } else if self.data.strength < other.data.strength {
            Some(Ordering::Less)
        } else if compare_duration(self.time, other.time) {
            Some(Ordering::Greater)
        } else if compare_duration(other.time, self.time) {
            Some(Ordering::Less)
        } else {
            None
        }
    }
}

fn compare_duration(a: Option<Duration>, b: Option<Duration>) -> bool {
    a.map_or(true, |dur_a| b.map_or(false, |dur_b| dur_a > dur_b))
}

impl PartialEq for Buff {
    fn eq(&self, other: &Self) -> bool {
        self.data.strength == other.data.strength && self.time == other.time
    }
}

/// Source of the de/buff
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum BuffSource {
    /// Applied by a character
    Character { by: Uid },
    /// Applied by world, like a poisonous fumes from a swamp
    World,
    /// Applied by command
    Command,
    /// Applied by an item
    Item,
    /// Applied by another buff (like an after-effect)
    Buff,
    /// Some other source
    Unknown,
}

/// Component holding all de/buffs that gets resolved each tick.
/// On each tick, remaining time of buffs get lowered and
/// buff effect of each buff is applied or not, depending on the `BuffEffect`
/// (specs system will decide based on `BuffEffect`, to simplify
/// implementation). TODO: Something like `once` flag for `Buff` to remove the
/// dependence on `BuffEffect` enum?
///
/// In case of one-time buffs, buff effects will be applied on addition
/// and undone on removal of the buff (by the specs system).
/// Example could be decreasing max health, which, if repeated each tick,
/// would be probably an undesired effect).
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Buffs {
    /// Uid used for synchronization
    id_counter: u64,
    /// Maps Kinds of buff to Id's of currently applied buffs of that kind
    pub kinds: HashMap<BuffKind, Vec<BuffId>>,
    // All currently applied buffs stored by Id
    pub buffs: HashMap<BuffId, Buff>,
}

impl Buffs {
    fn sort_kind(&mut self, kind: BuffKind) {
        if let Some(buff_order) = self.kinds.get_mut(&kind) {
            if buff_order.is_empty() {
                self.kinds.remove(&kind);
            } else {
                let buffs = &self.buffs;
                // Intentionally sorted in reverse so that the strongest buffs are earlier in
                // the vector
                buff_order.sort_by(|a, b| buffs[&b].partial_cmp(&buffs[&a]).unwrap());
            }
        }
    }

    pub fn remove_kind(&mut self, kind: BuffKind) {
        if let Some(buff_ids) = self.kinds.get_mut(&kind) {
            for id in buff_ids {
                self.buffs.remove(id);
            }
            self.kinds.remove(&kind);
        }
    }

    pub fn force_insert(&mut self, id: BuffId, buff: Buff) -> BuffId {
        let kind = buff.kind;
        self.kinds.entry(kind).or_default().push(id);
        self.buffs.insert(id, buff);
        self.sort_kind(kind);
        id
    }

    pub fn insert(&mut self, buff: Buff) -> BuffId {
        self.id_counter += 1;
        self.force_insert(self.id_counter, buff)
    }

    pub fn contains(&self, kind: BuffKind) -> bool { self.kinds.contains_key(&kind) }

    // Iterate through buffs of a given kind in effect order (most powerful first)
    pub fn iter_kind(&self, kind: BuffKind) -> impl Iterator<Item = (BuffId, &Buff)> + '_ {
        self.kinds
            .get(&kind)
            .map(|ids| ids.iter())
            .unwrap_or_else(|| (&[]).iter())
            .map(move |id| (*id, &self.buffs[id]))
    }

    // Iterates through all active buffs (the most powerful buff of each kind)
    pub fn iter_active(&self) -> impl Iterator<Item = &Buff> + '_ {
        self.kinds
            .values()
            .map(move |ids| self.buffs.get(&ids[0]))
            .filter(|buff| buff.is_some())
            .map(|buff| buff.unwrap())
    }

    // Gets most powerful buff of a given kind
    // pub fn get_active_kind(&self, kind: BuffKind) -> Buff

    pub fn remove(&mut self, buff_id: BuffId) {
        let kind = self.buffs.remove(&buff_id).unwrap().kind;
        self.kinds
            .get_mut(&kind)
            .map(|ids| ids.retain(|id| *id != buff_id));
        self.sort_kind(kind);
    }
}

pub type BuffId = u64;

impl Component for Buffs {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
