use serde::{Deserialize, Serialize};

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
    poise_protection: Protection,
}

impl Stats {
    pub fn new(protection: Protection, poise_protection: Protection) -> Self {
        Self {
            protection,
            poise_protection,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Protection {
    Invincible,
    Normal(f32),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Armor {
    pub kind: ArmorKind,
    pub stats: Stats,
}

impl Armor {
    pub fn new(kind: ArmorKind, protection: Protection, poise_protection: Protection) -> Self {
        Self {
            kind,
            stats: Stats {
                protection,
                poise_protection,
            },
        }
    }

    pub fn get_protection(&self) -> Protection { self.stats.protection }

    pub fn get_poise_protection(&self) -> Protection { self.stats.poise_protection }

    #[cfg(test)]
    pub fn test_armor(kind: ArmorKind, protection: Protection) -> Armor {
        Armor {
            kind,
            stats: Stats { protection },
        }
    }
}
