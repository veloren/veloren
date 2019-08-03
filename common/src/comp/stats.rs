use crate::state::Uid;
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum HealthSource {
    Attack { by: Uid }, // TODO: Implement weapon
    Suicide,
    Revive,
    Command,
    LevelUp,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Health {
    current: u32,
    maximum: u32,
    pub last_change: Option<(i32, f64, HealthSource)>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Exp {
    current: f64,
    maximum: f64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Level {
    amount: u32,
}

impl Health {
    pub fn current(&self) -> u32 {
        self.current
    }

    pub fn maximum(&self) -> u32 {
        self.maximum
    }

    pub fn set_to(&mut self, amount: u32, cause: HealthSource) {
        let amount = amount.min(self.maximum);
        self.last_change = Some((amount as i32 - self.current as i32, 0.0, cause));
        self.current = amount;
    }

    pub fn change_by(&mut self, amount: i32, cause: HealthSource) {
        self.current = ((self.current as i32 + amount).max(0) as u32).min(self.maximum);
        self.last_change = Some((amount, 0.0, cause));
    }

    pub fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }
}

impl Exp {
    pub fn current(&self) -> f64 {
        self.current
    }

    pub fn maximum(&self) -> f64 {
        self.maximum
    }

    pub fn set_current(&mut self, current: f64) {
        self.current = current;
    }

    // TODO: Uncomment when needed
    // pub fn set_maximum(&mut self, maximum: f64) {
    // self.maximum = maximum;
    // }

    pub fn change_by(&mut self, current: f64) {
        self.current = self.current + current;
    }

    pub fn change_maximum_by(&mut self, maximum: f64) {
        self.maximum = self.maximum + maximum;
    }
}

impl Level {
    // TODO: Uncomment when needed
    // pub fn set_level(&mut self, level: u32) {
    // self.amount = level;
    // }

    pub fn level(&self) -> u32 {
        self.amount
    }

    pub fn change_by(&mut self, level: u32) {
        self.amount = self.amount + level;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub name: String,
    pub health: Health,
    pub level: Level,
    pub exp: Exp,
    pub is_dead: bool,
}

impl Stats {
    pub fn should_die(&self) -> bool {
        // TODO: Remove
        self.health.current == 0
    }
    pub fn revive(&mut self) {
        self.health
            .set_to(self.health.maximum(), HealthSource::Revive);
        self.is_dead = false;
    }
}

impl Stats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            health: Health {
                current: 100,
                maximum: 100,
                last_change: None,
            },
            level: Level { amount: 1 },
            exp: Exp {
                current: 0.0,
                maximum: 50.0,
            },
            is_dead: false,
        }
    }

    pub fn with_max_health(mut self, amount: u32) -> Self {
        self.health.maximum = amount;
        self.health.current = amount;
        self
    }
}

impl Component for Stats {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Dying {
    pub cause: HealthSource,
}

impl Component for Dying {
    type Storage = IDVStorage<Self>;
}
