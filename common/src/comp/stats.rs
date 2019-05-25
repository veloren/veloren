use crate::state::Time;
use specs::{Component, FlaggedStorage, NullStorage, VecStorage};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Health {
    pub current: u32,
    pub maximum: u32,
    pub last_change: Option<(i32, f64)>,
}

impl Health {
    pub fn change_by(&mut self, amount: i32) {
        self.current = (self.current as i32 + amount).max(0) as u32;
        self.last_change = Some((amount, 0.0));
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub hp: Health,
    pub xp: u32,
}

impl Stats {
    pub fn is_dead(&self) -> bool {
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
        }
    }
}

impl Component for Stats {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Dying;

impl Component for Dying {
    type Storage = NullStorage<Self>;
}
