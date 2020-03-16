use crate::{
    comp,
    comp::{body::humanoid::Race, Body},
    sync::Uid,
};
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthChange {
    pub amount: i32,
    pub cause: HealthSource,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthSource {
    Attack { by: Uid }, // TODO: Implement weapon
    Suicide,
    World,
    Revive,
    Command,
    LevelUp,
    Item,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Health {
    current: u32,
    maximum: u32,
    pub last_change: (f64, HealthChange),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Exp {
    current: u32,
    maximum: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Level {
    amount: u32,
}

impl Health {
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

    // This is private because max hp is based on the level
    fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }
}
#[derive(Debug)]
pub enum StatChangeError {
    Underflow,
    Overflow,
}
use std::{error::Error, fmt};
impl fmt::Display for StatChangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            Self::Underflow => "insufficient stat quantity",
            Self::Overflow => "stat quantity would overflow",
        })
    }
}
impl Error for StatChangeError {}

impl Exp {
    pub fn current(&self) -> u32 { self.current }

    pub fn maximum(&self) -> u32 { self.maximum }

    pub fn set_current(&mut self, current: u32) { self.current = current; }

    // TODO: Uncomment when needed
    // pub fn set_maximum(&mut self, maximum: u32) {
    // self.maximum = maximum;
    // }

    pub fn change_by(&mut self, current: i64) {
        self.current = ((self.current as i64) + current) as u32;
    }

    pub fn change_maximum_by(&mut self, maximum: i64) {
        self.maximum = ((self.maximum as i64) + maximum) as u32;
    }
}

impl Level {
    pub fn set_level(&mut self, level: u32) { self.amount = level; }

    pub fn level(&self) -> u32 { self.amount }

    pub fn change_by(&mut self, level: u32) { self.amount += level; }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub name: String,
    pub health: Health,
    pub level: Level,
    pub exp: Exp,
    pub endurance: u32,
    pub fitness: u32,
    pub willpower: u32,
    pub is_dead: bool,
}

impl Stats {
    pub fn should_die(&self) -> bool { self.health.current == 0 }

    pub fn revive(&mut self) {
        self.health
            .set_to(self.health.maximum(), HealthSource::Revive);
        self.is_dead = false;
    }

    // TODO: Delete this once stat points will be a thing
    pub fn update_max_hp(&mut self) { self.health.set_maximum(21 + 3 * self.level.amount); }
}

impl Stats {
    pub fn new(name: String, body: Body) -> Self {
        let race = if let comp::Body::Humanoid(hbody) = body {
            Some(hbody.race)
        } else {
            None
        };

        let (endurance, fitness, willpower) = match race {
            Some(Race::Danari) => (0, 2, 3), // Small, flexible, intelligent, physically weak
            Some(Race::Dwarf) => (2, 2, 1),  // phyiscally strong, intelligent, slow reflexes
            Some(Race::Elf) => (1, 2, 2),    // Intelligent, quick, physically weak
            Some(Race::Human) => (2, 1, 2),  // Perfectly balanced
            Some(Race::Orc) => (3, 2, 0),    // Physically strong, non intelligent, medium reflexes
            Some(Race::Undead) => (1, 3, 1), // Very good reflexes, equally intelligent and strong
            None => (0, 0, 0),
        };

        let mut stats = Self {
            name,
            health: Health {
                current: 0,
                maximum: 0,
                last_change: (0.0, HealthChange {
                    amount: 0,
                    cause: HealthSource::Revive,
                }),
            },
            level: Level { amount: 1 },
            exp: Exp {
                current: 0,
                maximum: 50,
            },
            endurance,
            fitness,
            willpower,
            is_dead: false,
        };

        stats.update_max_hp();
        stats
            .health
            .set_to(stats.health.maximum(), HealthSource::Revive);

        stats
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
