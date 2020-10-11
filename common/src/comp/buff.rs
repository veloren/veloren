use crate::sync::Uid;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

/// De/buff ID.
/// ID can be independant of an actual type/config of a `BuffEffect`.
/// Therefore, information provided by `BuffId` can be incomplete/incorrect.
///
/// For example, there could be two regeneration buffs, each with
/// different strength, but they could use the same `BuffId`,
/// making it harder to recognize which is which.
///
/// Also, this should be dehardcoded eventually.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum BuffId {
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
/// Similar to `BuffId`, but to mark a category (for more generic usage, like
/// positive/negative buffs).
#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum BuffCategoryId {
    Natural,
    Physical,
    Magical,
    Divine,
    Debuff,
    Buff,
}

/// Data indicating and configuring behaviour of a de/buff.
///
/// NOTE: Contents of this enum are WIP/Placeholder
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
/// uncursing). The `time` field might be moved into the `Buffs` component
/// (so that `Buff` does not own this information).
///
/// Buff has an id and data, which can be independent on each other.
/// This makes it hard to create buff stacking "helpers", as the system
/// does not assume that the same id is always the same behaviour (data).
/// Therefore id=behaviour relationship has to be enforced elsewhere (if
/// desired).
///
/// To provide more classification info when needed,
/// buff can be in one or more buff category.
///
/// `data` is separate, to make this system more flexible
/// (at the cost of the fact that id=behaviour relationship might not apply).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Buff {
    pub id: BuffId,
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
    RemoveById(BuffId),
    /// Removes buffs of these indices (first vec is for active buffs, second is
    /// for inactive buffs)
    RemoveByIndex(Vec<usize>, Vec<usize>),
    /// Removes buffs of these categories (first vec is of categories of which
    /// all are required, second vec is of categories of which at least one is
    /// required) Note that this functionality is currently untested and
    /// should be tested when doing so is possible
    RemoveByCategory(Vec<BuffCategoryId>, Vec<BuffCategoryId>),
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

impl Buffs {
    /// This is a primitive check if a specific buff is present and active.
    /// (for purposes like blocking usage of abilities or something like this).
    pub fn has_buff_id(&self, id: &BuffId) -> bool {
        self.active_buffs.iter().any(|buff| buff.id == *id)
    }
}

impl Buff {
    pub fn new(id: BuffId, cat_ids: Vec<BuffCategoryId>, source: BuffSource) -> Self {
        let (effects, time) = match id {
            BuffId::Bleeding { strength, duration } => (
                vec![
                    BuffEffect::HealthChangeOverTime {
                        rate: -strength,
                        accumulated: 0.0,
                    },
                    // This effect is for testing purposes
                    BuffEffect::NameChange {
                        prefix: String::from("Injured "),
                    },
                ],
                duration,
            ),
            BuffId::Regeneration { strength, duration } => (
                vec![BuffEffect::HealthChangeOverTime {
                    rate: strength,
                    accumulated: 0.0,
                }],
                duration,
            ),
            BuffId::Cursed { duration } => (
                vec![BuffEffect::NameChange {
                    prefix: String::from("Cursed "),
                }],
                duration,
            ),
        };
        Buff {
            id,
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
