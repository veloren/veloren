#[cfg(not(target_arch = "wasm32"))]
use crate::comp;
use crate::{uid::Uid, DamageSource};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use specs::{Component, DerefFlaggedStorage};
#[cfg(not(target_arch = "wasm32"))]
use specs_idvs::IdvStorage;

/// Specifies what and how much changed current health
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct HealthChange {
    pub amount: f32,
    pub by: Option<Uid>,
    pub cause: Option<DamageSource>,
}

impl HealthChange {
    pub fn damage_by(&self) -> Option<Uid> { self.cause.is_some().then_some(self.by).flatten() }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
/// Health is represented by u32s within the module, but treated as a float by
/// the rest of the game.
// As a general rule, all input and output values to public functions should be
// floats rather than integers.
pub struct Health {
    // Current and base_max are scaled by 256 within this module compared to what is visible to
    // outside this module
    current: u32,
    base_max: u32,
    maximum: u32,
    // Time since last change and what the last change was
    // TODO: Remove the time since last change, either convert to time of last change or just emit
    // an outcome where appropriate. Is currently just used for frontend.
    pub last_change: (f64, HealthChange),
    pub is_dead: bool,
}

impl Health {
    /// The maximum value allowed for current and maximum health
    /// Maximum value is u16:MAX - 1 * 256, which only requires 24 bits. This
    /// can fit into an f32 with no loss to precision
    const MAX_HEALTH: u32 = 16776960;
    /// The amount health is scaled by within this module
    const SCALING_FACTOR_FLOAT: f32 = 256.;
    const SCALING_FACTOR_INT: u32 = 256;

    /// Returns the current value of health casted to a float
    pub fn current(&self) -> f32 { self.current as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the base maximum value of health casted to a float
    pub fn base_max(&self) -> f32 { self.base_max as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the maximum value of health casted to a float
    pub fn maximum(&self) -> f32 { self.maximum as f32 / Self::SCALING_FACTOR_FLOAT }

    /// Returns the fraction of health an entity has remaining
    pub fn fraction(&self) -> f32 { self.current() / self.maximum().max(1.0) }

    /// Updates the maximum value for health
    pub fn update_maximum(&mut self, modifiers: comp::stats::StatsModifier) {
        let maximum = modifiers
            .compute_maximum(self.base_max as f32)
            .min(Self::MAX_HEALTH as f32) as u32;
        self.maximum = maximum;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new(body: comp::Body, level: u16) -> Self {
        let health = u32::from(body.base_health() + body.base_health_increase() * level)
            * Self::SCALING_FACTOR_INT;
        Health {
            current: health,
            base_max: health,
            maximum: health,
            last_change: (0.0, HealthChange {
                amount: 0.0,
                by: None,
                cause: None,
            }),
            is_dead: false,
        }
    }

    // TODO: Delete this once stat points will be a thing
    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_max_hp(&mut self, body: comp::Body, level: u16) {
        let old_max = self.base_max;
        self.base_max = u32::from(body.base_health() + body.base_health_increase() * level)
            * Self::SCALING_FACTOR_INT;
        self.current = (self.current + self.base_max - old_max).min(self.maximum);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn change_by(&mut self, change: HealthChange) {
        self.current = (((self.current() + change.amount) as u32 * Self::SCALING_FACTOR_INT).max(0)
            as u32)
            .min(self.maximum);
        self.last_change = (0.0, change);
    }

    pub fn should_die(&self) -> bool { self.current == 0 }

    pub fn kill(&mut self) {
        self.current = 0;
        self.is_dead = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn revive(&mut self) {
        self.current = self.maximum;
        self.is_dead = false;
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Component for Health {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
