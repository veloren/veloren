use crate::comp::{self, Body, Inventory};
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

    // This function changes the modified max energy value, not the base energy
    // value. The modified energy value takes into account buffs and other temporary
    // changes to max energy.
    pub fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }

    // Scales the temporary max health by a modifier.
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
            self.set_base_max(body.base_energy() + ENERGY_PER_LEVEL * level as u32);
            self.set_maximum(body.base_energy() + ENERGY_PER_LEVEL * level as u32);
            self.change_by(EnergyChange {
                amount: ENERGY_PER_LEVEL as i32,
                source: EnergySource::LevelUp,
            });
        }
    }

    // This is private because max energy is based on the level
    fn set_base_max(&mut self, amount: u32) {
        self.base_max = amount;
        self.current = self.current.min(self.maximum);
    }

    /// Computes the energy reward modifer from worn armor
    pub fn compute_energy_reward_mod(inventory: Option<&Inventory>) -> f32 {
        use comp::item::ItemKind;
        // Starts with a value of 1.0 when summing the stats from each armor piece, and
        // defaults to a value of 1.0 if no inventory is present
        inventory.map_or(1.0, |inv| {
            inv.equipped_items()
                .filter_map(|item| {
                    if let ItemKind::Armor(armor) = &item.kind() {
                        Some(armor.get_energy_recovery())
                    } else {
                        None
                    }
                })
                .fold(1.0, |a, b| a + b)
        })
    }

    /// Computes the modifier that should be applied to max energy from the
    /// currently equipped items
    pub fn compute_max_energy_mod_from_inv(&self, inventory: Option<&Inventory>) -> f32 {
        use comp::item::ItemKind;
        // Defaults to a value of 0 if no inventory is present
        let energy_increase = inventory.map_or(0, |inv| {
            inv.equipped_items()
                .filter_map(|item| {
                    if let ItemKind::Armor(armor) = &item.kind() {
                        Some(armor.get_energy_max())
                    } else {
                        None
                    }
                })
                .sum()
        });
        // Returns the energy increase divided by base max of energy.
        // This value is then added to the max_energy_modifier field on stats component.
        // Adding is important here, as it ensures that a flat modifier is applied
        // correctly.
        energy_increase as f32 / self.base_max as f32
    }
}

pub struct EnergyChange {
    pub amount: i32,
    pub source: EnergySource,
}

impl Component for Energy {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
