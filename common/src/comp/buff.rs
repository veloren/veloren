use crate::uid::Uid;
use core::{cmp::Ordering, time::Duration};
#[cfg(not(target_arch = "wasm32"))]
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use specs::{Component, DerefFlaggedStorage};
#[cfg(not(target_arch = "wasm32"))]
use specs_idvs::IdvStorage;
use strum_macros::EnumIter;

/// De/buff Kind.
/// This is used to determine what effects a buff will have
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord, EnumIter,
)]
pub enum BuffKind {
    // Buffs
    /// Restores health/time for some period
    /// Strength should be 10x the healing per second
    Regeneration,
    /// Restores health/time for some period for consumables
    /// Strength should be 10x the healing per second
    Saturation,
    /// Applied when drinking a potion
    /// Strength should be 10x the healing per second
    Potion,
    /// Applied when sitting at a campfire
    /// Strength is fraction of health resotred per second
    CampfireHeal,
    /// Raises maximum stamina
    /// Strength should be 10x the effect to max energy
    IncreaseMaxEnergy,
    /// Raises maximum health
    /// Strength should be 10x the effect to max health
    IncreaseMaxHealth,
    /// Makes you immune to attacks
    /// Strength does not affect this buff
    Invulnerability,
    /// Reduces incoming damage
    /// Strength scales the damage reduction non-linearly. 0.5 provides 50% DR,
    /// 1.0 provides 67% DR
    ProtectingWard,
    /// Increases movement speed and gives health regeneration
    /// Strength scales the movement speed linearly. 0.5 is 150% speed, 1.0 is
    /// 200% speed. Provides regeneration at 10x the value of the strength
    Frenzied,
    // Debuffs
    /// Does damage to a creature over time
    /// Strength should be 10x the DPS of the debuff
    Burning,
    /// Lowers health over time for some duration
    /// Strength should be 10x the DPS of the debuff
    Bleeding,
    /// Lower a creature's max health over time
    /// Strength only affects the target max health, 0.5 targets 50% of base
    /// max, 1.0 targets 100% of base max
    Cursed,
    /// Reduces movement speed and causes bleeding damage
    /// Strength scales the movement speed debuff non-linearly. 0.5 is 50%
    /// speed, 1.0 is 33% speed. Bleeding is at 10x the value of the strength.
    Crippled,
    /// Slows movement and attack speed.
    /// Strength scales the attack speed debuff non-linearly. 0.5 is ~50%
    /// speed, 1.0 is 33% speed. Movement speed debuff is scaled to be slightly
    /// smaller than attack speed debuff.
    Frozen,
    /// Makes you wet and causes you to have reduced friction on the ground.
    /// Strength scales the friction you ignore non-linearly. 0.5 is 50% ground
    /// friction, 1.0 is 33% ground friction.
    Wet,
    /// Makes you move slower.
    /// Strength scales the movement speed debuff non-linearly. 0.5 is 50%
    /// speed, 1.0 is 33% speed.
    Ensnared,
}

#[cfg(not(target_arch = "wasm32"))]
impl BuffKind {
    /// Checks if buff is buff or debuff
    pub fn is_buff(self) -> bool {
        match self {
            BuffKind::Regeneration => true,
            BuffKind::Saturation => true,
            BuffKind::Bleeding => false,
            BuffKind::Cursed => false,
            BuffKind::Potion => true,
            BuffKind::CampfireHeal => true,
            BuffKind::IncreaseMaxEnergy => true,
            BuffKind::IncreaseMaxHealth => true,
            BuffKind::Invulnerability => true,
            BuffKind::ProtectingWard => true,
            BuffKind::Burning => false,
            BuffKind::Crippled => false,
            BuffKind::Frenzied => true,
            BuffKind::Frozen => false,
            BuffKind::Wet => false,
            BuffKind::Ensnared => false,
        }
    }

    /// Checks if buff should queue
    pub fn queues(self) -> bool { matches!(self, BuffKind::Saturation) }
}

// Struct used to store data relevant to a buff
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuffData {
    pub strength: f32,
    pub duration: Option<Duration>,
}

#[cfg(not(target_arch = "wasm32"))]
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
    FromAura(bool), // bool used to check if buff recently set by aura
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
    /// Changes maximum stamina by a certain amount
    MaxEnergyModifier { value: f32, kind: ModifierKind },
    /// Reduces damage after armor is accounted for by this fraction
    DamageReduction(f32),
    /// Gradually changes an entities max health over time
    MaxHealthChangeOverTime {
        rate: f32,
        kind: ModifierKind,
        target_fraction: f32,
        achieved_fraction: Option<f32>,
    },
    /// Modifies move speed of target
    MovementSpeed(f32),
    /// Modifies attack speed of target
    AttackSpeed(f32),
    /// Modifies ground friction of target
    GroundFriction(f32),
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

#[cfg(not(target_arch = "wasm32"))]
impl Buff {
    /// Builder function for buffs
    pub fn new(
        kind: BuffKind,
        data: BuffData,
        cat_ids: Vec<BuffCategory>,
        source: BuffSource,
    ) -> Self {
        // Normalized nonlinear scaling
        let nn_scaling = |a| a / (a + 0.5);
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
                vec![
                    BuffEffect::MaxHealthChangeOverTime {
                        rate: -10.0,
                        kind: ModifierKind::Additive,
                        target_fraction: 1.0 - data.strength,
                        achieved_fraction: None,
                    },
                    BuffEffect::HealthChangeOverTime {
                        rate: -10.0,
                        accumulated: 0.0,
                        kind: ModifierKind::Additive,
                    },
                ],
                data.duration,
            ),
            BuffKind::IncreaseMaxEnergy => (
                vec![BuffEffect::MaxEnergyModifier {
                    value: data.strength,
                    kind: ModifierKind::Additive,
                }],
                data.duration,
            ),
            BuffKind::IncreaseMaxHealth => (
                vec![BuffEffect::MaxHealthModifier {
                    value: data.strength,
                    kind: ModifierKind::Additive,
                }],
                data.duration,
            ),
            BuffKind::Invulnerability => (vec![BuffEffect::DamageReduction(1.0)], data.duration),
            BuffKind::ProtectingWard => (
                vec![BuffEffect::DamageReduction(
                    // Causes non-linearity in effect strength, but necessary to allow for tool
                    // power and other things to affect the strength. 0.5 also still provides 50%
                    // damage reduction.
                    nn_scaling(data.strength),
                )],
                data.duration,
            ),
            BuffKind::Burning => (
                vec![BuffEffect::HealthChangeOverTime {
                    rate: -data.strength,
                    accumulated: 0.0,
                    kind: ModifierKind::Additive,
                }],
                data.duration,
            ),
            BuffKind::Crippled => (
                vec![
                    BuffEffect::MovementSpeed(1.0 - nn_scaling(data.strength)),
                    BuffEffect::HealthChangeOverTime {
                        rate: -data.strength * 40.0,
                        accumulated: 0.0,
                        kind: ModifierKind::Additive,
                    },
                ],
                data.duration,
            ),
            BuffKind::Frenzied => (
                vec![
                    BuffEffect::MovementSpeed(1.0 + data.strength),
                    BuffEffect::HealthChangeOverTime {
                        rate: data.strength * 100.0,
                        accumulated: 0.0,
                        kind: ModifierKind::Additive,
                    },
                ],
                data.duration,
            ),
            BuffKind::Frozen => (
                vec![
                    BuffEffect::MovementSpeed(f32::powf(1.0 - nn_scaling(data.strength), 1.1)),
                    BuffEffect::AttackSpeed(1.0 - nn_scaling(data.strength)),
                ],
                data.duration,
            ),
            BuffKind::Wet => (
                vec![BuffEffect::GroundFriction(1.0 - nn_scaling(data.strength))],
                data.duration,
            ),
            BuffKind::Ensnared => (
                vec![BuffEffect::MovementSpeed(1.0 - nn_scaling(data.strength))],
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
fn compare_duration(a: Option<Duration>, b: Option<Duration>) -> bool {
    a.map_or(true, |dur_a| b.map_or(false, |dur_b| dur_a > dur_b))
}

#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Buffs {
    /// Uid used for synchronization
    id_counter: u64,
    /// Maps Kinds of buff to Id's of currently applied buffs of that kind
    pub kinds: HashMap<BuffKind, Vec<BuffId>>,
    // All currently applied buffs stored by Id
    pub buffs: HashMap<BuffId, Buff>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Buffs {
    fn sort_kind(&mut self, kind: BuffKind) {
        if let Some(buff_order) = self.kinds.get_mut(&kind) {
            if buff_order.is_empty() {
                self.kinds.remove(&kind);
            } else {
                let buffs = &self.buffs;
                // Intentionally sorted in reverse so that the strongest buffs are earlier in
                // the vector
                buff_order
                    .sort_by(|a, b| buffs[&b].partial_cmp(&buffs[&a]).unwrap_or(Ordering::Equal));
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
            .filter_map(move |ids| self.buffs.get(&ids[0]))
    }

    // Gets most powerful buff of a given kind
    // pub fn get_active_kind(&self, kind: BuffKind) -> Buff
    pub fn remove(&mut self, buff_id: BuffId) {
        if let Some(kind) = self.buffs.remove(&buff_id) {
            let kind = kind.kind;
            self.kinds
                .get_mut(&kind)
                .map(|ids| ids.retain(|id| *id != buff_id));
            self.sort_kind(kind);
        }
    }

    /// Returns an immutable reference to the buff kinds on an entity, and a
    /// mutable reference to the buffs
    pub fn parts(&mut self) -> (&HashMap<BuffKind, Vec<BuffId>>, &mut HashMap<BuffId, Buff>) {
        (&self.kinds, &mut self.buffs)
    }
}

pub type BuffId = u64;

#[cfg(not(target_arch = "wasm32"))]
impl Component for Buffs {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
