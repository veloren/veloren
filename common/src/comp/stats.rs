use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{error::Error, fmt};

use crate::combat::{AttackEffect, DamagedEffect, DeathEffect};

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

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct StatsSplit {
    pub pos_mod: f32,
    pub neg_mod: f32,
}

impl Default for StatsSplit {
    fn default() -> Self {
        Self {
            pos_mod: 0.0,
            neg_mod: 0.0,
        }
    }
}

impl StatsSplit {
    pub fn modifier(&self) -> f32 { self.pos_mod + self.neg_mod }
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
    pub damage_reduction: StatsSplit,
    pub poise_reduction: StatsSplit,
    pub heal_multiplier: f32,
    // Note: This is used to counteract agility as a "potion sickness" right now, and otherwise
    // does not impact movement speed
    pub move_speed_multiplier: f32,
    pub max_health_modifiers: StatsModifier,
    pub move_speed_modifier: f32,
    pub jump_modifier: f32,
    pub attack_speed_modifier: f32,
    pub recovery_speed_modifier: f32,
    pub friction_modifier: f32,
    pub max_energy_modifiers: StatsModifier,
    pub poise_damage_modifier: f32,
    pub attack_damage_modifier: f32,
    pub precision_multiplier_override: Option<f32>,
    pub precision_vulnerability_multiplier_override: Option<f32>,
    pub swim_speed_modifier: f32,
    /// This adds effects to any attacks that the entity makes
    pub effects_on_attack: Vec<AttackEffect>,
    /// This is the fraction of damage reduction (from armor and other buffs)
    /// that gets ignored by attacks from this entity
    pub mitigations_penetration: f32,
    pub energy_reward_modifier: f32,
    /// This creates effects when the entity is damaged
    pub effects_on_damaged: Vec<DamagedEffect>,
    /// This creates effects when the entity is killed
    pub effects_on_death: Vec<DeathEffect>,
    pub disable_auxiliary_abilities: bool,
}

impl Stats {
    pub fn new(name: String, body: Body) -> Self {
        Self {
            name,
            original_body: body,
            damage_reduction: StatsSplit::default(),
            poise_reduction: StatsSplit::default(),
            heal_multiplier: 1.0,
            move_speed_multiplier: 1.0,
            max_health_modifiers: StatsModifier::default(),
            move_speed_modifier: 1.0,
            jump_modifier: 1.0,
            attack_speed_modifier: 1.0,
            recovery_speed_modifier: 1.0,
            friction_modifier: 1.0,
            max_energy_modifiers: StatsModifier::default(),
            poise_damage_modifier: 1.0,
            attack_damage_modifier: 1.0,
            precision_multiplier_override: None,
            precision_vulnerability_multiplier_override: None,
            swim_speed_modifier: 1.0,
            effects_on_attack: Vec::new(),
            mitigations_penetration: 0.0,
            energy_reward_modifier: 1.0,
            effects_on_damaged: Vec::new(),
            effects_on_death: Vec::new(),
            disable_auxiliary_abilities: false,
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
