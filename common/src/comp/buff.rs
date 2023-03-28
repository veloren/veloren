#![allow(clippy::nonstandard_macro_braces)] //tmp as of false positive !?
use crate::{
    comp::{aura::AuraKey, Health, Stats},
    resources::{Secs, Time},
    uid::Uid,
};
use core::cmp::Ordering;
#[cfg(not(target_arch = "wasm32"))]
use hashbrown::HashMap;
use itertools::Either;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use specs::{Component, DerefFlaggedStorage, VecStorage};
use strum::EnumIter;

use super::Body;

/// De/buff Kind.
/// This is used to determine what effects a buff will have
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize, PartialOrd, Ord, EnumIter,
)]
pub enum BuffKind {
    // Buffs
    /// Restores health/time for some period
    /// Strength should be the healing per second
    Regeneration,
    /// Restores health/time for some period for consumables
    /// Strength should be the healing per second
    Saturation,
    /// Applied when drinking a potion
    /// Strength should be the healing per second
    Potion,
    /// Applied when sitting at a campfire
    /// Strength is fraction of health restored per second
    CampfireHeal,
    /// Restores energy/time for some period
    /// Strength should be the healing per second
    EnergyRegen,
    /// Raises maximum energy
    /// Strength should be 10x the effect to max energy
    IncreaseMaxEnergy,
    /// Raises maximum health
    /// Strength should be the effect to max health
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
    /// Increases movement and attack speed, but removes chance to get critical
    /// hits. Strength scales strength of both effects linearly. 0.5 is a
    /// 50% increase, 1.0 is a 100% increase.
    Hastened,
    /// Increases resistance to incoming poise, and poise damage dealt as health
    /// is lost from the time the buff activated
    /// Strength scales the resistance non-linearly. 0.5 provides 50%, 1.0
    /// provides 67%
    /// Strength scales the poise damage increase linearly, a strength of 1.0
    /// and n health less from activation will cause poise damage to increase by
    /// n%
    Fortitude,
    /// Increases both attack damage and vulnerability to damage
    /// Damage increases linearly with strength, 1.0 is a 100% increase
    /// Damage reduction decreases linearly with strength, 1.0 is a 100%
    /// decrease
    Reckless,
    // Debuffs
    /// Does damage to a creature over time
    /// Strength should be the DPS of the debuff
    Burning,
    /// Lowers health over time for some duration
    /// Strength should be the DPS of the debuff
    Bleeding,
    /// Lower a creature's max health over time
    /// Strength only affects the target max health, 0.5 targets 50% of base
    /// max, 1.0 targets 100% of base max
    Cursed,
    /// Reduces movement speed and causes bleeding damage
    /// Strength scales the movement speed debuff non-linearly. 0.5 is 50%
    /// speed, 1.0 is 33% speed. Bleeding is at 4x the value of the strength.
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
    /// Drain stamina to a creature over time
    /// Strength should be the energy per second of the debuff
    Poisoned,
    /// Results from having an attack parried.
    /// Causes your attack speed to be slower to emulate the recover duration of
    /// an ability being lengthened.
    Parried,
    /// Results from drinking a potion.
    /// Decreases the health gained from subsequent potions.
    PotionSickness,
    // Changed into another body.
    Polymorphed(Body),
}

#[cfg(not(target_arch = "wasm32"))]
impl BuffKind {
    /// Checks if buff is buff or debuff
    pub fn is_buff(self) -> bool {
        match self {
            BuffKind::Regeneration
            | BuffKind::Saturation
            | BuffKind::Potion
            | BuffKind::CampfireHeal
            | BuffKind::Frenzied
            | BuffKind::EnergyRegen
            | BuffKind::IncreaseMaxEnergy
            | BuffKind::IncreaseMaxHealth
            | BuffKind::Invulnerability
            | BuffKind::ProtectingWard
            | BuffKind::Hastened
            | BuffKind::Fortitude
            | BuffKind::Reckless => true,
            BuffKind::Bleeding
            | BuffKind::Cursed
            | BuffKind::Burning
            | BuffKind::Crippled
            | BuffKind::Frozen
            | BuffKind::Wet
            | BuffKind::Ensnared
            | BuffKind::Poisoned
            | BuffKind::Parried
            | BuffKind::PotionSickness
            | BuffKind::Polymorphed(_) => false,
        }
    }

    /// Checks if buff should queue
    pub fn queues(self) -> bool { matches!(self, BuffKind::Saturation) }

    /// Checks if the buff can affect other buff effects applied in the same
    /// tick.
    pub fn affects_subsequent_buffs(self) -> bool { matches!(self, BuffKind::PotionSickness) }

    /// Checks if multiple instances of the buff should be processed, instead of
    /// only the strongest.
    pub fn stacks(self) -> bool { matches!(self, BuffKind::PotionSickness) }

    pub fn effects(
        &self,
        data: &BuffData,
        stats: Option<&Stats>,
        health: Option<&Health>,
    ) -> Vec<BuffEffect> {
        // Normalized nonlinear scaling
        let nn_scaling = |a| a / (a + 0.5);
        let instance = rand::random();
        match self {
            BuffKind::Bleeding => vec![BuffEffect::HealthChangeOverTime {
                rate: -data.strength,
                kind: ModifierKind::Additive,
                instance,
                tick_dur: Secs(0.5),
            }],
            BuffKind::Regeneration => vec![BuffEffect::HealthChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Additive,
                instance,
                tick_dur: Secs(1.0),
            }],
            BuffKind::Saturation => vec![BuffEffect::HealthChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Additive,
                instance,
                tick_dur: Secs(3.0),
            }],
            BuffKind::Potion => {
                vec![BuffEffect::HealthChangeOverTime {
                    rate: data.strength * stats.map_or(1.0, |s| s.heal_multiplier),
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(0.1),
                }]
            },
            BuffKind::CampfireHeal => vec![BuffEffect::HealthChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Fractional,
                instance,
                tick_dur: Secs(2.0),
            }],
            BuffKind::Cursed => vec![
                BuffEffect::MaxHealthChangeOverTime {
                    rate: -1.0,
                    kind: ModifierKind::Additive,
                    target_fraction: 1.0 - data.strength,
                },
                BuffEffect::HealthChangeOverTime {
                    rate: -1.0,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(0.5),
                },
            ],
            BuffKind::EnergyRegen => vec![BuffEffect::EnergyChangeOverTime {
                rate: data.strength,
                kind: ModifierKind::Additive,
                tick_dur: Secs(1.0),
            }],
            BuffKind::IncreaseMaxEnergy => vec![BuffEffect::MaxEnergyModifier {
                value: data.strength,
                kind: ModifierKind::Additive,
            }],
            BuffKind::IncreaseMaxHealth => vec![BuffEffect::MaxHealthModifier {
                value: data.strength,
                kind: ModifierKind::Additive,
            }],
            BuffKind::Invulnerability => vec![BuffEffect::DamageReduction(1.0)],
            BuffKind::ProtectingWard => vec![BuffEffect::DamageReduction(
                // Causes non-linearity in effect strength, but necessary
                // to allow for tool power and other things to affect the
                // strength. 0.5 also still provides 50% damage reduction.
                nn_scaling(data.strength),
            )],
            BuffKind::Burning => vec![BuffEffect::HealthChangeOverTime {
                rate: -data.strength,
                kind: ModifierKind::Additive,
                instance,
                tick_dur: Secs(0.25),
            }],
            BuffKind::Poisoned => vec![BuffEffect::EnergyChangeOverTime {
                rate: -data.strength,
                kind: ModifierKind::Additive,
                tick_dur: Secs(0.5),
            }],
            BuffKind::Crippled => vec![
                BuffEffect::MovementSpeed(1.0 - nn_scaling(data.strength)),
                BuffEffect::HealthChangeOverTime {
                    rate: -data.strength * 4.0,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(0.5),
                },
            ],
            BuffKind::Frenzied => vec![
                BuffEffect::MovementSpeed(1.0 + data.strength),
                BuffEffect::HealthChangeOverTime {
                    rate: data.strength * 10.0,
                    kind: ModifierKind::Additive,
                    instance,
                    tick_dur: Secs(1.0),
                },
            ],
            BuffKind::Frozen => vec![
                BuffEffect::MovementSpeed(f32::powf(1.0 - nn_scaling(data.strength), 1.1)),
                BuffEffect::AttackSpeed(1.0 - nn_scaling(data.strength)),
            ],
            BuffKind::Wet => vec![BuffEffect::GroundFriction(1.0 - nn_scaling(data.strength))],
            BuffKind::Ensnared => vec![BuffEffect::MovementSpeed(1.0 - nn_scaling(data.strength))],
            BuffKind::Hastened => vec![
                BuffEffect::MovementSpeed(1.0 + data.strength),
                BuffEffect::AttackSpeed(1.0 + data.strength),
                BuffEffect::CriticalChance(0.0),
            ],
            BuffKind::Fortitude => vec![
                BuffEffect::PoiseReduction(nn_scaling(data.strength)),
                BuffEffect::PoiseDamageFromLostHealth {
                    initial_health: health.map_or(0.0, |h| h.current()),
                    strength: data.strength,
                },
            ],
            BuffKind::Parried => vec![BuffEffect::AttackSpeed(0.5)],
            BuffKind::PotionSickness => vec![BuffEffect::HealReduction(data.strength)],
            BuffKind::Reckless => vec![
                BuffEffect::DamageReduction(-data.strength),
                BuffEffect::AttackDamage(1.0 + data.strength),
            ],
            BuffKind::Polymorphed(body) => vec![BuffEffect::BodyChange(*body)],
        }
    }
}

// Struct used to store data relevant to a buff
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BuffData {
    pub strength: f32,
    pub duration: Option<Secs>,
    pub delay: Option<Secs>,
}

#[cfg(not(target_arch = "wasm32"))]
impl BuffData {
    pub fn new(strength: f32, duration: Option<Secs>, delay: Option<Secs>) -> Self {
        Self {
            strength,
            duration,
            delay,
        }
    }
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
    FromActiveAura(Uid, AuraKey),
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
        kind: ModifierKind,
        instance: u64,
        tick_dur: Secs,
    },
    /// Periodically consume entity energy
    EnergyChangeOverTime {
        rate: f32,
        kind: ModifierKind,
        tick_dur: Secs,
    },
    /// Changes maximum health by a certain amount
    MaxHealthModifier { value: f32, kind: ModifierKind },
    /// Changes maximum energy by a certain amount
    MaxEnergyModifier { value: f32, kind: ModifierKind },
    /// Reduces damage after armor is accounted for by this fraction
    DamageReduction(f32),
    /// Gradually changes an entities max health over time
    MaxHealthChangeOverTime {
        rate: f32,
        kind: ModifierKind,
        target_fraction: f32,
    },
    /// Modifies move speed of target
    MovementSpeed(f32),
    /// Modifies attack speed of target
    AttackSpeed(f32),
    /// Modifies ground friction of target
    GroundFriction(f32),
    /// Reduces poise damage taken after armor is accounted for by this fraction
    PoiseReduction(f32),
    /// Reduces amount healed by consumables
    HealReduction(f32),
    /// Increases poise damage dealt when health is lost
    PoiseDamageFromLostHealth { initial_health: f32, strength: f32 },
    /// Modifier to the amount of damage dealt with attacks
    AttackDamage(f32),
    /// Multiplies crit chance of attacks
    CriticalChance(f32),
    /// Changes body.
    BodyChange(Body),
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
    pub end_time: Option<Time>,
    pub start_time: Time,
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
    // Refreshes durations of all buffs with this kind
    Refresh(BuffKind),
}

#[cfg(not(target_arch = "wasm32"))]
impl Buff {
    /// Builder function for buffs
    pub fn new(
        kind: BuffKind,
        data: BuffData,
        cat_ids: Vec<BuffCategory>,
        source: BuffSource,
        time: Time,
        stats: Option<&Stats>,
        health: Option<&Health>,
    ) -> Self {
        let effects = kind.effects(&data, stats, health);
        let start_time = Time(time.0 + data.delay.map_or(0.0, |delay| delay.0));
        let end_time = if cat_ids
            .iter()
            .any(|cat_id| matches!(cat_id, BuffCategory::FromActiveAura(..)))
        {
            None
        } else {
            data.duration.map(|dur| Time(start_time.0 + dur.0))
        };
        Buff {
            kind,
            data,
            cat_ids,
            start_time,
            end_time,
            effects,
            source,
        }
    }

    /// Calculate how much time has elapsed since the buff was applied
    pub fn elapsed(&self, time: Time) -> Secs { Secs(time.0 - self.start_time.0) }
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
        } else if self.data.delay.is_none() && other.data.delay.is_some() {
            Some(Ordering::Greater)
        } else if self.data.delay.is_some() && other.data.delay.is_none() {
            Some(Ordering::Less)
        } else if compare_end_time(self.end_time, other.end_time) {
            Some(Ordering::Greater)
        } else if compare_end_time(other.end_time, self.end_time) {
            Some(Ordering::Less)
        } else {
            None
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn compare_end_time(a: Option<Time>, b: Option<Time>) -> bool {
    a.map_or(true, |time_a| b.map_or(false, |time_b| time_a.0 > time_b.0))
}

#[cfg(not(target_arch = "wasm32"))]
impl PartialEq for Buff {
    fn eq(&self, other: &Self) -> bool {
        self.data.strength == other.data.strength
            && self.end_time == other.end_time
            && self.start_time == other.start_time
    }
}

/// Source of the de/buff
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
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
    /// Maps Kinds of buff to Id's of currently applied buffs of that kind and
    /// the time that the first buff was added (time gets reset if entity no
    /// longer has buffs of that kind)
    pub kinds: HashMap<BuffKind, (Vec<BuffId>, Time)>,
    // All currently applied buffs stored by Id
    pub buffs: HashMap<BuffId, Buff>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Buffs {
    fn sort_kind(&mut self, kind: BuffKind) {
        if let Some(buff_order) = self.kinds.get_mut(&kind) {
            if buff_order.0.is_empty() {
                self.kinds.remove(&kind);
            } else {
                let buffs = &self.buffs;
                // Intentionally sorted in reverse so that the strongest buffs are earlier in
                // the vector
                buff_order
                    .0
                    .sort_by(|a, b| buffs[b].partial_cmp(&buffs[a]).unwrap_or(Ordering::Equal));
            }
        }
    }

    pub fn remove_kind(&mut self, kind: BuffKind) {
        if let Some(buff_ids) = self.kinds.get_mut(&kind) {
            for id in &buff_ids.0 {
                self.buffs.remove(id);
            }
            self.kinds.remove(&kind);
        }
    }

    fn force_insert(&mut self, id: BuffId, buff: Buff, current_time: Time) -> BuffId {
        let kind = buff.kind;
        self.kinds
            .entry(kind)
            .or_insert((Vec::new(), current_time))
            .0
            .push(id);
        self.buffs.insert(id, buff);
        self.sort_kind(kind);
        if kind.queues() {
            self.delay_queueable_buffs(kind, current_time);
        }
        id
    }

    pub fn insert(&mut self, buff: Buff, current_time: Time) -> BuffId {
        self.id_counter += 1;
        self.force_insert(self.id_counter, buff, current_time)
    }

    pub fn contains(&self, kind: BuffKind) -> bool { self.kinds.contains_key(&kind) }

    // Iterate through buffs of a given kind in effect order (most powerful first)
    pub fn iter_kind(&self, kind: BuffKind) -> impl Iterator<Item = (BuffId, &Buff)> + '_ {
        self.kinds
            .get(&kind)
            .map(|ids| ids.0.iter())
            .unwrap_or_else(|| [].iter())
            .map(move |id| (*id, &self.buffs[id]))
    }

    // Iterates through all active buffs (the most powerful buff of each
    // non-stacking kind, and all of the stacking ones)
    pub fn iter_active(&self) -> impl Iterator<Item = impl Iterator<Item = &Buff>> + '_ {
        self.kinds.iter().map(move |(kind, ids)| {
            if kind.stacks() {
                // Iterate stackable buffs in reverse order to show the timer of the soonest one
                // to expire
                Either::Left(ids.0.iter().filter_map(|id| self.buffs.get(id)).rev())
            } else {
                Either::Right(self.buffs.get(&ids.0[0]).into_iter())
            }
        })
    }

    // Gets most powerful buff of a given kind
    pub fn remove(&mut self, buff_id: BuffId) {
        if let Some(kind) = self.buffs.remove(&buff_id) {
            let kind = kind.kind;
            self.kinds
                .get_mut(&kind)
                .map(|ids| ids.0.retain(|id| *id != buff_id));
            self.sort_kind(kind);
        }
    }

    fn delay_queueable_buffs(&mut self, kind: BuffKind, current_time: Time) {
        let mut next_start_time: Option<Time> = None;
        debug_assert!(kind.queues());
        if let Some(buffs) = self.kinds.get(&kind) {
            buffs.0.iter().for_each(|id| {
                if let Some(buff) = self.buffs.get_mut(id) {
                    // End time only being updated when there is some next_start_time will
                    // technically cause buffs to "end early" if they have a weaker strength than a
                    // buff with an infinite duration, but this is fine since those buffs wouldn't
                    // matter anyways
                    if let Some(next_start_time) = next_start_time {
                        // Delays buff so that it has the same progress it has now at the time the
                        // previous buff would end.
                        //
                        // Shift should be relative to current time, unless the buff is delayed and
                        // hasn't started yet
                        let reference_time = current_time.0.max(buff.start_time.0);
                        // If buff has a delay, ensure that queueables shuffling queue does not
                        // potentially allow skipping delay
                        buff.start_time = Time(next_start_time.0.max(buff.start_time.0));
                        buff.end_time = buff.end_time.map(|end| {
                            Time(end.0 + next_start_time.0.max(reference_time) - reference_time)
                        });
                    }
                    next_start_time = buff.end_time;
                }
            })
        }
    }
}

pub type BuffId = u64;

#[cfg(not(target_arch = "wasm32"))]
impl Component for Buffs {
    type Storage = DerefFlaggedStorage<Self, VecStorage<Self>>;
}

#[cfg(test)]
pub mod tests {
    use crate::comp::buff::*;

    #[cfg(test)]
    fn create_test_queueable_buff(buff_data: BuffData, time: Time) -> Buff {
        // Change to another buff that queues if we ever add one and remove saturation,
        // otherwise maybe add a test buff kind?
        debug_assert!(BuffKind::Saturation.queues());
        Buff::new(
            BuffKind::Saturation,
            buff_data,
            Vec::new(),
            BuffSource::Unknown,
            time,
            None,
            None,
        )
    }

    #[test]
    /// Tests a number of buffs with various progresses that queue to ensure
    /// queue has correct total duration
    fn test_queueable_buffs_three() {
        let mut buff_comp: Buffs = Default::default();
        let buff_data = BuffData::new(1.0, Some(Secs(10.0)), None);
        let time_a = Time(0.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_a), time_a);
        let time_b = Time(6.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_b), time_b);
        let time_c = Time(11.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_c), time_c);
        // Check that all buffs have an end_time less than or equal to 30, and that at
        // least one has an end_time greater than or equal to 30.
        //
        // This should be true because 3 buffs that each lasted for 10 seconds were
        // inserted at various times, so the total duration should be 30 seconds.
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| b.end_time.unwrap().0 < 30.01)
        );
        assert!(
            buff_comp
                .buffs
                .values()
                .any(|b| b.end_time.unwrap().0 > 29.99)
        );
    }

    #[test]
    /// Tests that if a buff had a delay but will start soon, and an immediate
    /// queueable buff is added, delayed buff has correct start time
    fn test_queueable_buff_delay_start() {
        let mut buff_comp: Buffs = Default::default();
        let queued_buff_data = BuffData::new(1.0, Some(Secs(10.0)), Some(Secs(10.0)));
        let buff_data = BuffData::new(1.0, Some(Secs(10.0)), None);
        let time_a = Time(0.0);
        buff_comp.insert(create_test_queueable_buff(queued_buff_data, time_a), time_a);
        let time_b = Time(6.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_b), time_b);
        // Check that all buffs have an end_time less than or equal to 26, and that at
        // least one has an end_time greater than or equal to 26.
        //
        // This should be true because the first buff added had a delay of 10 seconds
        // and a duration of 10 seconds, the second buff added at 6 seconds had no
        // delay, and a duration of 10 seconds. When it finishes at 16 seconds the first
        // buff is past the delay time so should finish at 26 seconds.
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| b.end_time.unwrap().0 < 26.01)
        );
        assert!(
            buff_comp
                .buffs
                .values()
                .any(|b| b.end_time.unwrap().0 > 25.99)
        );
    }

    #[test]
    /// Tests that if a buff had a long delay, a short immediate queueable buff
    /// does not move delayed buff start or end times
    fn test_queueable_buff_long_delay() {
        let mut buff_comp: Buffs = Default::default();
        let queued_buff_data = BuffData::new(1.0, Some(Secs(10.0)), Some(Secs(50.0)));
        let buff_data = BuffData::new(1.0, Some(Secs(10.0)), None);
        let time_a = Time(0.0);
        buff_comp.insert(create_test_queueable_buff(queued_buff_data, time_a), time_a);
        let time_b = Time(10.0);
        buff_comp.insert(create_test_queueable_buff(buff_data, time_b), time_b);
        // Check that all buffs have either an end time less than or equal to 20 seconds
        // XOR a start time greater than or equal to 50 seconds, that all buffs have a
        // start time less than or equal to 50 seconds, that all buffs have an end time
        // less than or equal to 60 seconds, and that at least one buff has an end time
        // greater than or equal to 60 seconds
        //
        // This should be true because the first buff has a delay of 50 seconds, the
        // second buff added has no delay at 10 seconds and lasts 10 seconds, so should
        // end at 20 seconds and not affect the start time of the delayed buff, and
        // since the delayed buff was not affected the end time should be 10 seconds
        // after the start time: 60 seconds != used here to emulate xor
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| (b.end_time.unwrap().0 < 20.01) != (b.start_time.0 > 49.99))
        );
        assert!(buff_comp.buffs.values().all(|b| b.start_time.0 < 50.01));
        assert!(
            buff_comp
                .buffs
                .values()
                .all(|b| b.end_time.unwrap().0 < 60.01)
        );
        assert!(
            buff_comp
                .buffs
                .values()
                .any(|b| b.end_time.unwrap().0 > 59.99)
        );
    }
}
