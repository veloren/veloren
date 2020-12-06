use crate::{
    comp::{Body, Loadout},
    sync::Uid,
    DamageSource,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoiseChange {
    pub amount: i32,
    pub source: PoiseSource,
}

impl PoiseChange {
    pub fn modify_poise_damage(self, loadout: Option<&Loadout>, uid: Option<Uid>) -> PoiseChange {
        println!("Pre modified: {:?}", self.amount);
        let mut poise_damage = -self.amount as f32;
        match self.source {
            PoiseSource::Melee => {
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_poise_damage_reduction());
                poise_damage *= 1.0 - damage_reduction;
                PoiseChange {
                    amount: -poise_damage as i32,
                    source: PoiseSource::Melee,
                }
            },
            PoiseSource::Projectile => {
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_poise_damage_reduction());
                poise_damage *= 1.0 - damage_reduction;
                PoiseChange {
                    amount: -poise_damage as i32,
                    source: PoiseSource::Projectile,
                }
            },
            PoiseSource::Shockwave => {
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_poise_damage_reduction());
                poise_damage *= 1.0 - damage_reduction;
                PoiseChange {
                    amount: -poise_damage as i32,
                    source: PoiseSource::Shockwave,
                }
            },
            PoiseSource::Explosion => {
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_poise_damage_reduction());
                poise_damage *= 1.0 - damage_reduction;
                PoiseChange {
                    amount: -poise_damage as i32,
                    source: PoiseSource::Explosion,
                }
            },
            PoiseSource::Falling => {
                // Armor
                let damage_reduction = loadout.map_or(0.0, |l| l.get_poise_damage_reduction());
                if (damage_reduction - 1.0).abs() < f32::EPSILON {
                    poise_damage = 0.0;
                }
                PoiseChange {
                    amount: -poise_damage as i32,
                    source: PoiseSource::Falling,
                }
            },
            _ => PoiseChange {
                amount: self.amount,
                source: PoiseSource::Other,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PoiseSource {
    LevelUp,
    Melee,
    Projectile,
    Explosion,
    Beam,
    Shockwave,
    Falling,
    Revive,
    Other,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Poise {
    base_max: u32,
    current: u32,
    maximum: u32,
    pub is_interrupted: bool,
    pub is_stunned: bool,
    pub is_dazed: bool,
    pub is_knockeddown: bool,
}

impl Default for Poise {
    fn default() -> Self {
        Self {
            current: 0,
            maximum: 0,
            base_max: 0,
            is_interrupted: false,
            is_stunned: false,
            is_dazed: false,
            is_knockeddown: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum PoiseState {
    Normal,
    Interrupted,
    Stunned,
    Dazed,
    KnockedDown,
}

impl Poise {
    pub fn new(body: Body) -> Self {
        let mut poise = Poise::default();
        poise.update_max_poise(Some(body));
        poise.set_to(poise.maximum());

        poise
    }

    pub fn poise_state(&self) -> PoiseState {
        if self.current >= 5 * self.maximum / 10 {
            PoiseState::Normal
        } else if self.current >= 4 * self.maximum / 10 {
            PoiseState::Interrupted
        } else if self.current >= 3 * self.maximum / 10 {
            PoiseState::Stunned
        } else if self.current >= 2 * self.maximum / 10 {
            PoiseState::Dazed
        } else {
            PoiseState::KnockedDown
        }
    }

    pub fn current(&self) -> u32 { self.current }

    pub fn maximum(&self) -> u32 { self.maximum }

    pub fn set_to(&mut self, amount: u32) {
        let amount = amount.min(self.maximum);
        self.current = amount;
    }

    pub fn change_by(&mut self, change: PoiseChange) {
        self.current = ((self.current as i32 + change.amount).max(0) as u32).min(self.maximum);
    }

    pub fn reset(&mut self) { self.current = self.maximum; }

    pub fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }

    fn set_base_max(&mut self, amount: u32) {
        self.base_max = amount;
        self.current = self.current.min(self.maximum);
    }

    pub fn reset_max(&mut self) { self.maximum = self.base_max; }

    pub fn update_max_poise(&mut self, body: Option<Body>) {
        if let Some(body) = body {
            self.set_base_max(body.base_poise());
            self.set_maximum(body.base_poise());
        }
    }

    pub fn with_max_poise(mut self, amount: u32) -> Self {
        self.maximum = amount;
        self.current = amount;
        self
    }
}

impl Component for Poise {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
