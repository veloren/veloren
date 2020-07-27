use crate::comp::Loadout;
use serde::{Deserialize, Serialize};

pub const BLOCK_EFFICIENCY: f32 = 0.9;

pub struct Damage {
    pub healthchange: f32,
    pub source: DamageSource,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageSource {
    Melee,
    Healing,
    Projectile,
    Explosion,
    Falling,
}

impl Damage {
    pub fn modify_damage(&mut self, block: bool, loadout: &Loadout) {
        match self.source {
            DamageSource::Melee => {
                // Critical hit
                if rand::random() {
                    self.healthchange *= 1.2;
                }
                // Block
                if block {
                    self.healthchange *= 1.0 - BLOCK_EFFICIENCY
                }
                // Armor
                self.healthchange *= 1.0 - loadout.get_damage_reduction();

                // Min damage
                if self.healthchange > -1.0 {
                    self.healthchange = -1.0;
                }
            },
            DamageSource::Projectile => {
                // Critical hit
                if rand::random() {
                    self.healthchange *= 1.2;
                }
                // Block
                if block {
                    self.healthchange *= 1.0 - BLOCK_EFFICIENCY
                }
                // Armor
                self.healthchange *= 1.0 - loadout.get_damage_reduction();

                // Min damage
                if self.healthchange > -1.0 {
                    self.healthchange = -1.0;
                }
            },
            DamageSource::Explosion => {
                // Critical hit
                if rand::random() {
                    self.healthchange *= 1.2;
                }
                // Block
                if block {
                    self.healthchange *= 1.0 - BLOCK_EFFICIENCY
                }
                // Armor
                self.healthchange *= 1.0 - loadout.get_damage_reduction();

                // Min damage
                if self.healthchange > -1.0 {
                    self.healthchange = -1.0;
                }
            },
            _ => {},
        }
    }
}
