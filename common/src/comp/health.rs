use crate::{comp::Body, sync::Uid};
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;

/// Specifies what and how much changed current health
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthChange {
    pub amount: i32,
    pub cause: HealthSource,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthSource {
    Attack { by: Uid }, // TODO: Implement weapon
    Projectile { owner: Option<Uid> },
    Explosion { owner: Option<Uid> },
    Energy { owner: Option<Uid> },
    Buff { owner: Option<Uid> },
    Suicide,
    World,
    Revive,
    Command,
    LevelUp,
    Item,
    Healing { by: Option<Uid> },
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Health {
    base_max: u32,
    current: u32,
    maximum: u32,
    pub last_change: (f64, HealthChange),
    pub is_dead: bool,
}

impl Health {
    pub fn new(body: Body, level: u32) -> Self {
        let mut health = Health::empty();

        health.update_max_hp(Some(body), level);
        health.set_to(health.maximum(), HealthSource::Revive);

        health
    }

    pub fn empty() -> Self {
        Health {
            current: 0,
            maximum: 0,
            base_max: 0,
            last_change: (0.0, HealthChange {
                amount: 0,
                cause: HealthSource::Revive,
            }),
            is_dead: false,
        }
    }

    pub fn current(&self) -> u32 { self.current }

    pub fn maximum(&self) -> u32 { self.maximum }

    pub fn set_to(&mut self, amount: u32, cause: HealthSource) {
        let amount = amount.min(self.maximum);
        self.last_change = (0.0, HealthChange {
            amount: amount as i32 - self.current as i32,
            cause,
        });
        self.current = amount;
    }

    pub fn change_by(&mut self, change: HealthChange) {
        self.current = ((self.current as i32 + change.amount).max(0) as u32).min(self.maximum);
        self.last_change = (0.0, change);
    }

    // This function changes the modified max health value, not the base health
    // value. The modified health value takes into account buffs and other temporary
    // changes to max health.
    pub fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }

    // This is private because max hp is based on the level
    fn set_base_max(&mut self, amount: u32) {
        self.base_max = amount;
        self.current = self.current.min(self.maximum);
    }

    pub fn reset_max(&mut self) { self.maximum = self.base_max; }

    pub fn should_die(&self) -> bool { self.current == 0 }

    pub fn revive(&mut self) {
        self.set_to(self.maximum(), HealthSource::Revive);
        self.is_dead = false;
    }

    // TODO: Delete this once stat points will be a thing
    pub fn update_max_hp(&mut self, body: Option<Body>, level: u32) {
        if let Some(body) = body {
            self.set_base_max(body.base_health() + body.base_health_increase() * level);
            self.set_maximum(body.base_health() + body.base_health_increase() * level);
        }
    }

    pub fn with_max_health(mut self, amount: u32) -> Self {
        self.maximum = amount;
        self.current = amount;
        self
    }
}

impl Component for Health {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dead {
    pub cause: HealthSource,
}

impl Component for Dead {
    type Storage = IdvStorage<Self>;
}
