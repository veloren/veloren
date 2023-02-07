use crate::{comp, consts::ENERGY_PER_LEVEL};
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::ops::Mul;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
/// Energy is represented by u32s within the module, but treated as a float by
/// the rest of the game.
// As a general rule, all input and output values to public functions should be
// floats rather than integers.
pub struct Energy {
    // Current and base_max are scaled by 256 within this module compared to what is visible to
    // outside this module. The scaling is done to allow energy to function as a fixed point while
    // still having the advantages of being an integer. The scaling of 256 was chosen so that max
    // energy could be u16::MAX - 1, and then the scaled energy could fit inside an f32 with no
    // precision loss
    /// Current energy is how much energy the entity currently has
    current: u32,
    /// Base max is the amount of energy the entity has without considering
    /// temporary modifiers such as buffs
    base_max: u32,
    /// Maximum is the amount of energy the entity has after temporary modifiers
    /// are considered
    maximum: u32,
    /// Rate of regeneration per tick. Starts at zero and accelerates.
    regen_rate: f32,
}

impl Energy {
    /// Used when comparisons to energy are needed outside this module.
    // This value is chosen as anything smaller than this is more precise than our
    // units of energy.
    pub const ENERGY_EPSILON: f32 = 0.5 / Self::MAX_SCALED_ENERGY as f32;
    /// Maximum value allowed for energy before scaling
    const MAX_ENERGY: u16 = u16::MAX - 1;
    /// The maximum value allowed for current and maximum energy
    /// Maximum value is (u16:MAX - 1) * 256, which only requires 24 bits. This
    /// can fit into an f32 with no loss to precision
    // Cast to u32 done as u32::from cannot be called inside constant
    const MAX_SCALED_ENERGY: u32 = Self::MAX_ENERGY as u32 * Self::SCALING_FACTOR_INT;
    /// The amount energy is scaled by within this module
    const SCALING_FACTOR_FLOAT: f32 = 256.;
    const SCALING_FACTOR_INT: u32 = Self::SCALING_FACTOR_FLOAT as u32;

    /// Returns the current value of energy casted to a float
    pub fn current(&self) -> f32 { self.current as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the base maximum value of energy casted to a float
    pub fn base_max(&self) -> f32 { self.base_max as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the maximum value of energy casted to a float
    pub fn maximum(&self) -> f32 { self.maximum as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the fraction of energy an entity has remaining
    pub fn fraction(&self) -> f32 { self.current() / self.maximum().max(1.0) }

    /// Calculates a new maximum value and returns it if the value differs from
    /// the current maximum.
    ///
    /// Note: The returned value uses an internal format so don't expect it to
    /// be useful for anything other than a parameter to
    /// [`Self::update_maximum`].
    pub fn needs_maximum_update(&self, modifiers: comp::stats::StatsModifier) -> Option<u32> {
        let maximum = modifiers
            .compute_maximum(self.base_max())
            .mul(Self::SCALING_FACTOR_FLOAT)
            // NaN does not need to be handled here as rust will automatically change to 0 when casting to u32
            .clamp(0.0, Self::MAX_SCALED_ENERGY as f32) as u32;

        (maximum != self.maximum).then_some(maximum)
    }

    /// Updates the maximum value for energy.
    ///
    /// Note: The accepted `u32` value is in the internal format of this type.
    /// So attempting to pass values that weren't returned from
    /// [`Self::needs_maximum_update`] can produce strange or unexpected
    /// results.
    pub fn update_internal_integer_maximum(&mut self, maximum: u32) {
        self.maximum = maximum;
        // Clamp the current energy to enforce the current <= maximum invariant.
        self.current = self.current.min(self.maximum);
    }

    pub fn new(body: comp::Body, level: u16) -> Self {
        let energy = u32::from(
            body.base_energy()
                .saturating_add(ENERGY_PER_LEVEL.saturating_mul(level)),
        ) * Self::SCALING_FACTOR_INT;
        Energy {
            current: energy,
            base_max: energy,
            maximum: energy,
            regen_rate: 0.0,
        }
    }

    /// Returns `true` if the current value is less than the maximum
    pub fn needs_regen(&self) -> bool { self.current < self.maximum }

    /// Regenerates energy based on the provided acceleration
    pub fn regen(&mut self, accel: f32, dt: f32) {
        if self.current < self.maximum {
            self.change_by(self.regen_rate * dt);
            self.regen_rate = (self.regen_rate + accel * dt).min(10.0);
        }
    }

    /// Checks whether the `regen_rate` is zero or not. Returns true if the
    /// value is anything other than `0.0`.
    pub fn needs_regen_rate_reset(&self) -> bool { self.regen_rate != 0.0 }

    /// Resets the energy regeneration rate to zero
    pub fn reset_regen_rate(&mut self) { self.regen_rate = 0.0 }

    pub fn change_by(&mut self, change: f32) {
        self.current = (((self.current() + change).clamp(0.0, f32::from(Self::MAX_ENERGY))
            * Self::SCALING_FACTOR_FLOAT) as u32)
            .min(self.maximum);
    }

    #[allow(clippy::result_unit_err)]
    pub fn try_change_by(&mut self, change: f32) -> Result<(), ()> {
        let new_val = self.current() + change;
        if new_val < 0.0 || new_val > self.maximum() {
            Err(())
        } else {
            self.change_by(change);
            Ok(())
        }
    }

    pub fn update_max_energy(&mut self, body: comp::Body, level: u16) {
        let old_max = self.base_max;
        self.base_max = u32::from(
            body.base_energy()
                .saturating_add(ENERGY_PER_LEVEL.saturating_mul(level)),
        ) * Self::SCALING_FACTOR_INT;
        self.current = (self.current + self.base_max - old_max).min(self.maximum);
    }

    pub fn refresh(&mut self) { self.current = self.maximum; }
}

impl Component for Energy {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
