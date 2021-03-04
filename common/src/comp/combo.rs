use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;

pub const COMBO_DECAY_START: f64 = 5.0; // seconds

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Combo {
    counter: u32,
    last_increase: f64,
}

impl Default for Combo {
    fn default() -> Self {
        Self {
            counter: 0,
            last_increase: 0.0,
        }
    }
}

impl Combo {
    pub fn counter(&self) -> u32 { self.counter }

    pub fn last_increase(&self) -> f64 { self.last_increase }

    pub fn reset(&mut self) { self.counter = 0; }

    pub fn change_by(&mut self, amount: i32, time: f64) {
        if amount > 0 {
            self.counter = self.counter.saturating_add(amount as u32);
        } else {
            self.counter = self.counter.saturating_sub(amount.abs() as u32);
        }
        self.last_increase = time;
    }
}

impl Component for Combo {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
