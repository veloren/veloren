use crate::state::Uid;
use specs::{Component, FlaggedStorage, VecStorage};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum HealthSource {
    Attack { by: Uid }, // TODO: Implement weapon
    Suicide,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Health {
    current: u32,
    maximum: u32,
    pub last_change: Option<(i32, f64, HealthSource)>,
}

impl Health {
    pub fn get_current(&self) -> u32 {
        self.current
    }
    pub fn get_maximum(&self) -> u32 {
        self.maximum
    }
    pub fn set_to(&mut self, amount: u32, cause: HealthSource) {
        self.last_change = Some((amount as i32 - self.current as i32, 0.0, cause));
        self.current = amount;
    }
    pub fn change_by(&mut self, amount: i32, cause: HealthSource) {
        self.current = (self.current as i32 + amount).max(0) as u32;
        self.last_change = Some((amount, 0.0, cause));
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub hp: Health,
    pub xp: u32,
    pub is_dead: bool,
}

impl Stats {
    pub fn should_die(&self) -> bool {
        // TODO: Remove
        self.hp.current == 0
    }
}

impl Default for Stats {
    fn default() -> Self {
        Self {
            hp: Health {
                current: 100,
                maximum: 100,
                last_change: None,
            },
            xp: 0,
            is_dead: false,
        }
    }
}

impl Component for Stats {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dying {
    pub cause: HealthSource,
}

impl Component for Dying {
    type Storage = VecStorage<Self>;
}
