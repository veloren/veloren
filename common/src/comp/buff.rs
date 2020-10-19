use crate::sync::Uid;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

/// De/buff Kind.
/// This is used to determine what effects a buff will have, as well as
/// determine the strength and duration of the buff effects using the internal
/// values
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum BuffKind {
    /// Restores health/time for some period
    Regeneration {
        strength: f32,
        duration: Option<Duration>,
    },
    /// Lowers health over time for some duration
    Bleeding {
        strength: f32,
        duration: Option<Duration>,
    },
    /// Prefixes an entity's name with "Cursed"
    /// Currently placeholder buff to show other stuff is possible
    Cursed { duration: Option<Duration> },
}

/// De/buff category ID.
/// Similar to `BuffKind`, but to mark a category (for more generic usage, like
/// positive/negative buffs).
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum BuffCategoryId {
    // Buff and debuff get added in builder function based off of the buff kind
    Debuff,
    Buff,
    Natural,
    Physical,
    Magical,
    Divine,
    PersistOnDeath,
}

/// Data indicating and configuring behaviour of a de/buff.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BuffEffect {
    /// Periodically damages or heals entity
    HealthChangeOverTime { rate: f32, accumulated: f32 },
    /// Changes name on_add/on_remove
    NameChange { prefix: String },
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
    pub cat_ids: Vec<BuffCategoryId>,
    pub time: Option<Duration>,
    pub effects: Vec<BuffEffect>,
    pub source: BuffSource,
}

/// Information about whether buff addition or removal was requested.
/// This to implement "on_add" and "on_remove" hooks for constant buffs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BuffChange {
    /// Adds this buff.
    Add(Buff),
    /// Removes all buffs with this ID.
    RemoveByKind(BuffKind),
    /// Removes all buffs with this ID, but not debuffs.
    RemoveFromClient(BuffKind),
    /// Removes buffs of these indices (first vec is for active buffs, second is
    /// for inactive buffs), should only be called when buffs expire
    RemoveExpiredByIndex(Vec<usize>, Vec<usize>),
    /// Removes buffs of these categories (first vec is of categories of which
    /// all are required, second vec is of categories of which at least one is
    /// required, third vec is of categories that will not be removed)  
    RemoveByCategory {
        required: Vec<BuffCategoryId>,
        optional: Vec<BuffCategoryId>,
        blacklisted: Vec<BuffCategoryId>,
    },
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
    /// Active de/buffs.
    pub active_buffs: Vec<Buff>,
    /// Inactive de/buffs (used so that only 1 buff of a particular type is
    /// active at any time)
    pub inactive_buffs: Vec<Buff>,
}

impl Buff {
    /// Builder function for buffs
    pub fn new(kind: BuffKind, cat_ids: Vec<BuffCategoryId>, source: BuffSource) -> Self {
        let mut cat_ids = cat_ids;
        let (effects, time) = match kind {
            BuffKind::Bleeding { strength, duration } => {
                cat_ids.push(BuffCategoryId::Debuff);
                (
                    vec![BuffEffect::HealthChangeOverTime {
                        rate: -strength,
                        accumulated: 0.0,
                    }],
                    duration,
                )
            },
            BuffKind::Regeneration { strength, duration } => {
                cat_ids.push(BuffCategoryId::Buff);
                (
                    vec![BuffEffect::HealthChangeOverTime {
                        rate: strength,
                        accumulated: 0.0,
                    }],
                    duration,
                )
            },
            BuffKind::Cursed { duration } => {
                cat_ids.push(BuffCategoryId::Debuff);
                (
                    vec![BuffEffect::NameChange {
                        prefix: String::from("Cursed "),
                    }],
                    duration,
                )
            },
        };
        assert_eq!(
            cat_ids
                .iter()
                .any(|cat| *cat == BuffCategoryId::Buff || *cat == BuffCategoryId::Debuff),
            true,
            "Buff must have either buff or debuff category."
        );
        Buff {
            kind,
            cat_ids,
            time,
            effects,
            source,
        }
    }
}

impl Component for Buffs {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
