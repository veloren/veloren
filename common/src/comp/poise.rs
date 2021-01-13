use crate::comp::{
    inventory::item::{armor::Protection, ItemKind},
    Body, Inventory,
};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use vek::*;

/// A change in the poise component. Stores the amount as a signed
/// integer to allow for added or removed poise. Also has a field to
/// label where the poise change came from.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PoiseChange {
    /// Value of the change in poise
    pub amount: i32,
    /// Source of change in poise
    pub source: PoiseSource,
}

impl PoiseChange {
    /// Alters poise damage as a result of armor poise damage reduction
    pub fn modify_poise_damage(self, inventory: Option<&Inventory>) -> PoiseChange {
        let mut poise_damage = self.amount as f32;
        let poise_damage_reduction =
            inventory.map_or(0.0, |inv| Poise::compute_poise_damage_reduction(inv));
        match self.source {
            PoiseSource::Melee => {
                // Armor
                poise_damage *= 1.0 - poise_damage_reduction;
                PoiseChange {
                    amount: poise_damage as i32,
                    source: PoiseSource::Melee,
                }
            },
            PoiseSource::Projectile => {
                // Armor
                poise_damage *= 1.0 - poise_damage_reduction;
                PoiseChange {
                    amount: poise_damage as i32,
                    source: PoiseSource::Projectile,
                }
            },
            PoiseSource::Shockwave => {
                // Armor
                poise_damage *= 1.0 - poise_damage_reduction;
                PoiseChange {
                    amount: poise_damage as i32,
                    source: PoiseSource::Shockwave,
                }
            },
            PoiseSource::Explosion => {
                // Armor
                poise_damage *= 1.0 - poise_damage_reduction;
                PoiseChange {
                    amount: poise_damage as i32,
                    source: PoiseSource::Explosion,
                }
            },
            PoiseSource::Falling => {
                if (poise_damage_reduction - 1.0).abs() < f32::EPSILON {
                    poise_damage = 0.0;
                }
                PoiseChange {
                    amount: poise_damage as i32,
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

/// Sources of poise change
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
    Regen,
    Other,
}

/// Poise component
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Poise {
    /// Base poise amount for this entity
    base_max: u32,
    /// Poise of entity at any given moment
    current: u32,
    /// Maximum poise of entity at a given time
    maximum: u32,
    /// Knockback direction of last change, for use as an effect in sys/stats.rs
    knockback: Vec3<f32>,
    /// Last poise change, storing time since last change
    pub last_change: (f64, PoiseChange),
    /// Rate of poise regeneration per tick. Starts at zero and accelerates.
    pub regen_rate: f32,
}

impl Default for Poise {
    fn default() -> Self {
        Self {
            current: 0,
            maximum: 0,
            base_max: 0,
            knockback: Vec3::zero(),
            last_change: (0.0, PoiseChange {
                amount: 0,
                source: PoiseSource::Revive,
            }),
            regen_rate: 0.0,
        }
    }
}

/// States to define effects of a poise change
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
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
    /// Creates a new poise struct based on the body it is being assigned to
    pub fn new(body: Body) -> Self {
        let mut poise = Poise::default();
        poise.update_base_max(Some(body));
        poise.set_maximum(poise.base_max);
        poise.set_to(poise.maximum, PoiseSource::Revive);

        poise
    }

    /// Returns knockback as a Vec3
    pub fn knockback(&self) -> Vec3<f32> { self.knockback }

    /// Defines the poise states based on fraction of maximum poise
    pub fn poise_state(&self) -> PoiseState {
        if self.current >= 8 * self.maximum / 10 {
            PoiseState::Normal
        } else if self.current >= 7 * self.maximum / 10 {
            PoiseState::Interrupted
        } else if self.current >= 6 * self.maximum / 10 {
            PoiseState::Stunned
        } else if self.current >= 4 * self.maximum / 10 {
            PoiseState::Dazed
        } else {
            PoiseState::KnockedDown
        }
    }

    /// Gets the current poise value
    pub fn current(&self) -> u32 { self.current }

    /// Gets the maximum poise value
    pub fn maximum(&self) -> u32 { self.maximum }

    /// Gets the base_max value
    pub fn base_max(&self) -> u32 { self.base_max }

    /// Sets the poise value to a provided value. First cuts off the value
    /// at the maximum. In most cases change_by() should be used.
    pub fn set_to(&mut self, amount: u32, cause: PoiseSource) {
        let amount = amount.min(self.maximum);
        self.last_change = (0.0, PoiseChange {
            amount: amount as i32 - self.current as i32,
            source: cause,
        });
        self.current = amount;
    }

    /// Changes the current poise due to an in-game effect.
    pub fn change_by(&mut self, change: PoiseChange, impulse: Vec3<f32>) {
        self.current = ((self.current as i32 + change.amount).max(0) as u32).min(self.maximum);
        self.knockback = impulse;
        self.last_change = (0.0, PoiseChange {
            amount: change.amount,
            source: change.source,
        });
    }

    /// Resets current value to maximum
    pub fn reset(&mut self) { self.current = self.maximum; }

    /// Sets the maximum and updates the current value to max out at the new
    /// maximum
    pub fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }

    /// Sets the `Poise` base_max
    fn set_base_max(&mut self, amount: u32) {
        self.base_max = amount;
        self.current = self.current.min(self.maximum);
    }

    /// Resets the maximum to the base_max. Example use would be a potion
    /// wearing off
    pub fn reset_max(&mut self) { self.maximum = self.base_max; }

    /// Sets the base_max based on the entity `Body`
    pub fn update_base_max(&mut self, body: Option<Body>) {
        if let Some(body) = body {
            self.set_base_max(body.base_poise());
        }
    }

    /// Returns the total poise damage reduction provided by all equipped items
    pub fn compute_poise_damage_reduction(inventory: &Inventory) -> f32 {
        let protection = inventory
            .equipped_items()
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind() {
                    Some(armor.get_poise_protection())
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
}

impl Component for Poise {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
