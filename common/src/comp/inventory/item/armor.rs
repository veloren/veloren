use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, ops::Sub};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArmorKind {
    Shoulder(String),
    Chest(String),
    Belt(String),
    Hand(String),
    Pants(String),
    Foot(String),
    Back(String),
    Ring(String),
    Neck(String),
    Head(String),
    Tabard(String),
    Bag(String),
}

impl Armor {
    /// Determines whether two pieces of armour are superficially equivalent to
    /// one another (i.e: one may be substituted for the other in crafting
    /// recipes or item possession checks).
    pub fn superficially_eq(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.kind) == std::mem::discriminant(&other.kind)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    protection: Protection,
    poise_resilience: Protection,
}

impl Stats {
    // DO NOT USE UNLESS YOU KNOW WHAT YOU ARE DOING
    // Added for csv import of stats
    pub fn new(protection: Protection, poise_resilience: Protection) -> Self {
        Self {
            protection,
            poise_resilience,
        }
    }

    pub fn get_protection(&self) -> Protection { self.protection }

    pub fn get_poise_resilience(&self) -> Protection { self.poise_resilience }
}

impl Sub<Stats> for Stats {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            protection: self.protection - other.protection,
            poise_resilience: self.poise_resilience - other.poise_resilience,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Protection {
    Invincible,
    Normal(f32),
}

impl Sub for Protection {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        let diff = match (self, other) {
            (Protection::Invincible, Protection::Normal(_)) => f32::INFINITY,
            (Protection::Invincible, Protection::Invincible) => 0_f32,
            (Protection::Normal(_), Protection::Invincible) => -f32::INFINITY,
            (Protection::Normal(a), Protection::Normal(b)) => a - b,
        };
        Protection::Normal(diff)
    }
}

impl PartialOrd for Protection {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (*self, *other) {
            (Protection::Invincible, Protection::Invincible) => Some(Ordering::Equal),
            (Protection::Invincible, _) => Some(Ordering::Greater),
            (_, Protection::Invincible) => Some(Ordering::Less),
            (Protection::Normal(a), Protection::Normal(b)) => f32::partial_cmp(&a, &b),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Armor {
    pub kind: ArmorKind,
    pub stats: Stats,
}

impl Armor {
    pub fn new(kind: ArmorKind, protection: Protection, poise_resilience: Protection) -> Self {
        Self {
            kind,
            stats: Stats {
                protection,
                poise_resilience,
            },
        }
    }

    pub fn get_protection(&self) -> Protection { self.stats.protection }

    pub fn get_poise_resilience(&self) -> Protection { self.stats.poise_resilience }

    #[cfg(test)]
    pub fn test_armor(
        kind: ArmorKind,
        protection: Protection,
        poise_resilience: Protection,
    ) -> Armor {
        Armor {
            kind,
            stats: Stats {
                protection,
                poise_resilience,
            },
        }
    }
}
