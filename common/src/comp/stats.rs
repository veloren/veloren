use specs::{Component, FlaggedStorage, VecStorage};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Health {
    pub current: u32,
    pub maximum: u32,
    pub last_change: Option<(i32, f64)>,
}

impl Health {
    pub fn change_by(&mut self, amount: i32, current_time: f64) {
        self.current = (self.current as i32 + amount).max(0) as u32;
        self.last_change = Some((amount, current_time));
    }
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Stats {
    pub hp: Health,
    pub xp: u32,
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
