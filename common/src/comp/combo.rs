use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;

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

    pub fn increase_by(&mut self, amount: u32, time: f64) {
        self.counter = self.counter.saturating_add(amount);
        self.last_increase = time;
    }

    pub fn decrease_by(&mut self, amount: u32) {
        self.counter = self.counter.saturating_sub(amount);
    }
}

impl Component for Combo {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}
