use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{error::Error, fmt};

use super::Body;

#[derive(Debug)]
#[allow(dead_code)] // TODO: remove once trade sim hits master
pub enum StatChangeError {
    Underflow,
    Overflow,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct StatsModifier {
    pub add_mod: f32,
    pub mult_mod: f32,
}

impl Default for StatsModifier {
    fn default() -> Self {
        Self {
            add_mod: 0.0,
            mult_mod: 1.0,
        }
    }
}

impl StatsModifier {
    pub fn compute_maximum(&self, base_value: f32) -> f32 {
        base_value * self.mult_mod + self.add_mod
    }

    // Note: unused for now
    pub fn update_maximum(&self) -> bool {
        self.add_mod.abs() > f32::EPSILON || (self.mult_mod - 1.0).abs() > f32::EPSILON
    }
}

impl fmt::Display for StatChangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Underflow => "insufficient stat quantity",
            Self::Overflow => "stat quantity would overflow",
        })
    }
}
impl Error for StatChangeError {}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub name: String,
    pub original_body: Body,
    pub damage_reduction: f32,
    pub poise_reduction: f32,
    pub heal_multiplier: f32,
    pub max_health_modifiers: StatsModifier,
    pub move_speed_modifier: f32,
    pub attack_speed_modifier: f32,
    pub friction_modifier: f32,
    pub max_energy_modifiers: StatsModifier,
    pub poise_damage_modifier: f32,
    pub attack_damage_modifier: f32,
    pub crit_chance_modifier: f32,
}

impl Stats {
    pub fn new(name: String, body: Body) -> Self {
        Self {
            name,
            original_body: body,
            damage_reduction: 0.0,
            poise_reduction: 0.0,
            heal_multiplier: 1.0,
            max_health_modifiers: StatsModifier::default(),
            move_speed_modifier: 1.0,
            attack_speed_modifier: 1.0,
            friction_modifier: 1.0,
            max_energy_modifiers: StatsModifier::default(),
            poise_damage_modifier: 1.0,
            attack_damage_modifier: 1.0,
            crit_chance_modifier: 1.0,
        }
    }

    /// Creates an empty `Stats` instance - used during character loading from
    /// the database
    pub fn empty(body: Body) -> Self { Self::new("".to_string(), body) }

    /// Resets temporary modifiers to default values
    pub fn reset_temp_modifiers(&mut self) {
        let name = std::mem::take(&mut self.name);
        let body = self.original_body;

        *self = Self::new(name, body);
    }
}

impl Component for Stats {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
