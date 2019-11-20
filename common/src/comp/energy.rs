use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Energy {
    current: u32,
    maximum: u32,
    pub regen_rate: i32,
    pub last_change: Option<(i32, f64, EnergySource)>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum EnergySource {
    CastSpell,
    LevelUp,
    Regen,
    Unknown,
}

impl Energy {
    pub fn new(amount: u32) -> Energy {
        Energy {
            current: amount,
            maximum: amount,
            regen_rate: 0,
            last_change: None,
        }
    }

    pub fn current(&self) -> u32 {
        self.current
    }

    pub fn maximum(&self) -> u32 {
        self.maximum
    }

    pub fn set_to(&mut self, amount: u32, cause: EnergySource) {
        let amount = amount.min(self.maximum);
        self.last_change = Some((amount as i32 - self.current as i32, 0.0, cause));
        self.current = amount;
    }

    pub fn change_by(&mut self, amount: i32, cause: EnergySource) {
        self.current = ((self.current as i32 + amount).max(0) as u32).min(self.maximum);
        self.last_change = Some((amount, 0.0, cause));
    }

    pub fn set_maximum(&mut self, amount: u32) {
        self.maximum = amount;
        self.current = self.current.min(self.maximum);
    }
}

impl Component for Energy {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
