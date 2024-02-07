use crate::{
    comp::item::{DurabilityMultiplier, MaterialStatManifest, Rgb},
    terrain::{Block, BlockKind},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    ops::{Mul, Sub},
};
use strum::{EnumIter, IntoEnumIterator};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
pub enum ArmorKind {
    Shoulder,
    Chest,
    Belt,
    Hand,
    Pants,
    Foot,
    Back,
    Ring,
    Neck,
    Head,
    Tabard,
    Bag,
    Backpack,
}

impl ArmorKind {
    pub fn has_durability(self) -> bool {
        match self {
            ArmorKind::Shoulder => true,
            ArmorKind::Chest => true,
            ArmorKind::Belt => true,
            ArmorKind::Hand => true,
            ArmorKind::Pants => true,
            ArmorKind::Foot => true,
            ArmorKind::Back => true,
            ArmorKind::Backpack => true,
            ArmorKind::Ring => false,
            ArmorKind::Neck => false,
            ArmorKind::Head => true,
            ArmorKind::Tabard => false,
            ArmorKind::Bag => false,
        }
    }
}

impl Armor {
    /// Determines whether two pieces of armour are superficially equivalent to
    /// one another (i.e: one may be substituted for the other in crafting
    /// recipes or item possession checks).
    pub fn superficially_eq(&self, other: &Self) -> bool {
        std::mem::discriminant(&self.kind) == std::mem::discriminant(&other.kind)
    }
}

/// longitudinal and lateral friction, only meaningful for footwear
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Friction {
    Normal,
    Ski,
    Skate,
    // Snowshoe,
    // Spikes,
}

impl Default for Friction {
    fn default() -> Self { Self::Normal }
}

impl Friction {
    pub fn can_skate_on(&self, b: BlockKind) -> bool {
        match self {
            Friction::Ski => matches!(
                b,
                BlockKind::Snow | BlockKind::ArtSnow | BlockKind::Ice | BlockKind::Air
            ),
            Friction::Skate => b == BlockKind::Ice,
            _ => false,
        }
    }

    /// longitudinal (forward) and lateral (side) friction
    pub fn get_friction(&self, b: BlockKind) -> (f32, f32) {
        match (self, b) {
            (Friction::Ski, BlockKind::Snow) => (0.01, 0.95),
            (Friction::Ski, BlockKind::ArtSnow) => (0.01, 0.95),
            (Friction::Ski, BlockKind::Ice) => (0.001, 0.5),
            (Friction::Ski, BlockKind::Water) => (0.1, 0.7),
            (Friction::Ski, BlockKind::Air) => (0.0, 0.0),
            (Friction::Skate, BlockKind::Ice) => (0.001, 0.99),
            _ => {
                let non_directional_friction = Block::new(b, Rgb::new(0, 0, 0)).get_friction();
                (non_directional_friction, non_directional_friction)
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    /// Protection is non-linearly transformed (following summation) to a damage
    /// reduction using (prot / (60 + prot))
    pub protection: Option<Protection>,
    /// Poise protection is non-linearly transformed (following summation) to a
    /// poise damage reduction using (prot / (60 + prot))
    pub poise_resilience: Option<Protection>,
    /// Energy max is summed, and then applied directly to the max energy stat
    pub energy_max: Option<f32>,
    /// Energy recovery is summed, and then added to 1.0. When attacks reward
    /// energy, it is then multiplied by this value before the energy is
    /// rewarded.
    pub energy_reward: Option<f32>,
    /// Precision power is summed, and then added to the default precision
    /// multiplier of 1.1.
    pub precision_power: Option<f32>,
    /// Stealth is summed along with the base stealth bonus (2.0), and then
    /// the agent's perception distance is divided by this value
    pub stealth: Option<f32>,
    /// Ground contact type, mostly for shoes
    #[serde(default)]
    pub ground_contact: Friction,
}

impl Stats {
    fn none() -> Self {
        Stats {
            protection: None,
            poise_resilience: None,
            energy_max: None,
            energy_reward: None,
            precision_power: None,
            stealth: None,
            ground_contact: Friction::Normal,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum StatsSource {
    Direct(Stats),
    FromSet(String),
}

impl Mul<f32> for Stats {
    type Output = Self;

    fn mul(self, val: f32) -> Self::Output {
        Stats {
            protection: self.protection.map(|a| a * val),
            poise_resilience: self.poise_resilience.map(|a| a * val),
            energy_max: self.energy_max.map(|a| a * val),
            energy_reward: self.energy_reward.map(|a| a * val),
            precision_power: self.precision_power.map(|a| a * val),
            stealth: self.stealth.map(|a| a * val),
            // There is nothing to multiply, it is just an enum
            ground_contact: self.ground_contact,
        }
    }
}

impl Sub<Stats> for Stats {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        Self {
            protection: self.protection.zip(other.protection).map(|(a, b)| a - b),
            poise_resilience: self
                .poise_resilience
                .zip(other.poise_resilience)
                .map(|(a, b)| a - b),
            energy_max: self.energy_max.zip(other.energy_max).map(|(a, b)| a - b),
            energy_reward: self
                .energy_reward
                .zip(other.energy_reward)
                .map(|(a, b)| a - b),
            precision_power: self
                .precision_power
                .zip(other.precision_power)
                .map(|(a, b)| a - b),
            stealth: self.stealth.zip(other.stealth).map(|(a, b)| a - b),
            ground_contact: Friction::Normal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Protection {
    Invincible,
    Normal(f32),
}

impl Default for Protection {
    fn default() -> Self { Self::Normal(0.0) }
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

impl Mul<f32> for Protection {
    type Output = Self;

    fn mul(self, val: f32) -> Self::Output {
        match self {
            Protection::Invincible => Protection::Invincible,
            Protection::Normal(a) => Protection::Normal(a * val),
        }
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
#[serde(deny_unknown_fields)]
pub struct Armor {
    pub kind: ArmorKind,
    pub stats: StatsSource,
}

impl Armor {
    pub fn new(kind: ArmorKind, stats: StatsSource) -> Self { Self { kind, stats } }

    pub fn stats(
        &self,
        msm: &MaterialStatManifest,
        durability_multiplier: DurabilityMultiplier,
    ) -> Stats {
        let base_stats = match &self.stats {
            StatsSource::Direct(stats) => *stats,
            StatsSource::FromSet(set) => {
                let set_stats = msm.armor_stats(set).unwrap_or_else(Stats::none);
                let armor_kind_weight = |kind| match kind {
                    ArmorKind::Shoulder => 2.0,
                    ArmorKind::Chest => 3.0,
                    ArmorKind::Belt => 0.5,
                    ArmorKind::Hand => 1.0,
                    ArmorKind::Pants => 2.0,
                    ArmorKind::Foot => 1.0,
                    ArmorKind::Back => 0.5,
                    ArmorKind::Backpack => 0.0,
                    ArmorKind::Ring => 0.0,
                    ArmorKind::Neck => 0.0,
                    ArmorKind::Head => 0.0,
                    ArmorKind::Tabard => 0.0,
                    ArmorKind::Bag => 0.0,
                };

                let armor_weights_sum: f32 = ArmorKind::iter().map(armor_kind_weight).sum();
                let multiplier = armor_kind_weight(self.kind) / armor_weights_sum;

                set_stats * multiplier
            },
        };
        base_stats * durability_multiplier.0
    }

    #[cfg(test)]
    pub fn test_armor(
        kind: ArmorKind,
        protection: Protection,
        poise_resilience: Protection,
    ) -> Armor {
        Armor {
            kind,
            stats: StatsSource::Direct(Stats {
                protection: Some(protection),
                poise_resilience: Some(poise_resilience),
                energy_max: None,
                energy_reward: None,
                precision_power: None,
                stealth: None,
                ground_contact: Friction::Normal,
            }),
        }
    }
}
