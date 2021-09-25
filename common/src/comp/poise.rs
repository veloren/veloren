use crate::{
    comp::{
        self,
        inventory::item::{armor::Protection, ItemKind},
        Inventory,
    },
    util::Dir,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::ops::Mul;
use vek::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
/// Poise is represented by u32s within the module, but treated as a float by
/// the rest of the game.
// As a general rule, all input and output values to public functions should be
// floats rather than integers.
pub struct Poise {
    // Current and base_max are scaled by 256 within this module compared to what is visible to
    // outside this module. The scaling is done to allow poise to function as a fixed point while
    // still having the advantages of being an integer. The scaling of 256 was chosen so that max
    // poise could be u16::MAX - 1, and then the scaled poise could fit inside an f32 with no
    // precision loss
    /// Current poise is how much poise the entity currently has
    current: u32,
    /// Base max is the amount of poise the entity has without considering
    /// temporary modifiers such as buffs
    base_max: u32,
    /// Maximum is the amount of poise the entity has after temporary modifiers
    /// are considered
    maximum: u32,
    /// Direction that the last poise change came from
    pub last_change: Dir,
    /// Rate of poise regeneration per tick. Starts at zero and accelerates.
    pub regen_rate: f32,
}

/// States to define effects of a poise change
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Eq, Hash)]
pub enum PoiseState {
    /// No effect applied
    Normal,
    /// Poise reset, and target briefly stunned
    Interrupted,
    /// Poise reset, target stunned and knocked back horizontally
    Stunned,
    /// Poise reset, target staggered
    Dazed,
    /// Poise reset, target staggered and knocked back further
    KnockedDown,
}

impl Poise {
    /// Maximum value allowed for poise before scaling
    const MAX_POISE: u16 = u16::MAX - 1;
    /// The maximum value allowed for current and maximum poise
    /// Maximum value is (u16:MAX - 1) * 256, which only requires 24 bits. This
    /// can fit into an f32 with no loss to precision
    // Cast to u32 done as u32::from cannot be called inside constant
    const MAX_SCALED_POISE: u32 = Self::MAX_POISE as u32 * Self::SCALING_FACTOR_INT;
    /// Used when comparisons to poise are needed outside this module.
    // This value is chosen as anything smaller than this is more precise than our
    // units of poise.
    pub const POISE_EPSILON: f32 = 0.5 / Self::MAX_SCALED_POISE as f32;
    /// The amount poise is scaled by within this module
    const SCALING_FACTOR_FLOAT: f32 = 256.;
    const SCALING_FACTOR_INT: u32 = Self::SCALING_FACTOR_FLOAT as u32;

    /// Returns the current value of poise casted to a float
    pub fn current(&self) -> f32 { self.current as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the base maximum value of poise casted to a float
    pub fn base_max(&self) -> f32 { self.base_max as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the maximum value of poise casted to a float
    pub fn maximum(&self) -> f32 { self.maximum as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the fraction of poise an entity has remaining
    pub fn fraction(&self) -> f32 { self.current() / self.maximum().max(1.0) }

    /// Updates the maximum value for poise
    pub fn update_maximum(&mut self, modifiers: comp::stats::StatsModifier) {
        let maximum = modifiers
            .compute_maximum(self.base_max())
            .mul(Self::SCALING_FACTOR_FLOAT)
            // NaN does not need to be handled here as rust will automatically change to 0 when casting to u32
            .clamp(0.0, Self::MAX_SCALED_POISE as f32) as u32;
        self.maximum = maximum;
        self.current = self.current.min(self.maximum);
    }

    pub fn new(body: comp::Body) -> Self {
        let poise = u32::from(body.base_poise()) * Self::SCALING_FACTOR_INT;
        Poise {
            current: poise,
            base_max: poise,
            maximum: poise,
            last_change: Dir::default(),
            regen_rate: 0.0,
        }
    }

    pub fn change_by(&mut self, change: f32, impulse: Vec3<f32>) {
        self.current = (((self.current() + change).clamp(0.0, f32::from(Self::MAX_POISE))
            * Self::SCALING_FACTOR_FLOAT) as u32)
            .min(self.maximum);
        self.last_change = Dir::from_unnormalized(impulse).unwrap_or_default();
    }

    pub fn reset(&mut self) { self.current = self.maximum; }

    /// Returns knockback as a Dir
    /// Kept as helper function should additional fields ever be added to last
    /// change
    pub fn knockback(&self) -> Dir { self.last_change }

    /// Defines the poise states based on current poise value
    pub fn poise_state(&self) -> PoiseState {
        match self.current() {
            x if x > 70.0 => PoiseState::Normal,
            x if x > 50.0 => PoiseState::Interrupted,
            x if x > 40.0 => PoiseState::Stunned,
            x if x > 20.0 => PoiseState::Dazed,
            _ => PoiseState::KnockedDown,
        }
    }

    /// Returns the total poise damage reduction provided by all equipped items
    pub fn compute_poise_damage_reduction(inventory: &Inventory) -> f32 {
        let protection = inventory
            .equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.poise_resilience())
                } else {
                    None
                }
            })
            .map(|protection| match protection {
                Protection::Normal(protection) => Some(protection),
                Protection::Invincible => None,
            })
            .sum::<Option<f32>>();
        match protection {
            Some(dr) => dr / (60.0 + dr.abs()),
            None => 1.0,
        }
    }

    /// Modifies a poise change when optionally given an inventory to aid in
    /// calculation of poise damage reduction
    pub fn apply_poise_reduction(value: f32, inventory: Option<&Inventory>) -> f32 {
        inventory.map_or(value, |inv| {
            value * (1.0 - Poise::compute_poise_damage_reduction(inv))
        })
    }
}

impl Component for Poise {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
