use crate::comp::Body;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Energy {
    current: u32,
    base_max: u32,
    maximum: u32,
    pub regen_rate: f32,
    pub last_change: Option<(i32, f64, EnergySource)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum EnergySource {
    Ability,
    Climb,
    LevelUp,
    HitEnemy,
    Regen,
    Revive,
    Unknown,
}

#[derive(Debug)]
pub enum StatChangeError {
    Underflow,
    Overflow,
}

impl Energy {
    pub fn new(body: Body, level: u16) -> Energy {
        let mut energy = Energy::empty();

        energy.update_max_energy(Some(body), level);
        energy.set_to(energy.maximum(), EnergySource::Revive);

        energy
    }

    pub fn empty() -> Self {
        Energy {
            current: 0,
            maximum: 0,
            base_max: 0,
            regen_rate: 0.0,
            last_change: None,
        }
    }

    pub fn current(&self) -> u32 { self.current }

    pub fn base_max(&self) -> u32 { self.base_max }

    pub fn maximum(&self) -> u32 { self.maximum }

    pub fn set_to(&mut self, amount: u32, cause: EnergySource) {
        let amount = amount.min(self.maximum);
        self.last_change = Some((amount as i32 - self.current as i32, 0.0, cause));
        self.current = amount;
    }

    pub fn change_by(&mut self, change: EnergyChange) {
        self.current = ((self.current as i32 + change.amount).max(0) as u32).min(self.maximum);
        self.last_change = Some((change.amount, 0.0, change.source));
    }

    /// This function changes the modified max energy value, not the base energy
    /// value. The modified energy value takes into account buffs and other
    /// temporary changes to max energy.
    pub fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }

    /// Scales the temporary max energy by a modifier.
    pub fn scale_maximum(&mut self, scaled: f32) {
        let scaled_max = (self.base_max as f32 * scaled) as u32;
        self.set_maximum(scaled_max);
    }

    pub fn try_change_by(
        &mut self,
        amount: i32,
        cause: EnergySource,
    ) -> Result<(), StatChangeError> {
        if self.current as i32 + amount < 0 {
            Err(StatChangeError::Underflow)
        } else if self.current as i32 + amount > self.maximum as i32 {
            Err(StatChangeError::Overflow)
        } else {
            self.change_by(EnergyChange {
                amount,
                source: cause,
            });
            Ok(())
        }
    }

    pub fn update_max_energy(&mut self, body: Option<Body>, level: u16) {
        const ENERGY_PER_LEVEL: u32 = 50;
        if let Some(body) = body {
            // Checks the current difference between maximum and base max
            let current_difference = self.maximum as i32 - self.base_max as i32;
            // Sets base max to new value based off of new level provided
            self.base_max = body.base_energy() + ENERGY_PER_LEVEL * level as u32;
            // Calculates new maximum by adding difference to new base max
            let new_maximum = (self.base_max as i32 + current_difference).max(0) as u32;
            // Sets maximum to calculated value
            self.set_maximum(new_maximum);
            // Awards energy
            self.change_by(EnergyChange {
                amount: ENERGY_PER_LEVEL as i32,
                source: EnergySource::LevelUp,
            });
        }
    }
}

pub struct EnergyChange {
    pub amount: i32,
    pub source: EnergySource,
}

impl Component for Energy {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
