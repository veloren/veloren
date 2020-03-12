use specs::{Component, DenseVecStorage, FlaggedStorage, HashMapStorage};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum AbilityState {
    BasicAttack {
        /// Amount of energy required to use ability
        cost: i32,
    },
    BasicBlock,
    Roll,
    ChargeAttack,
    TimedCombo {
        /// Amount of energy required to use ability
        cost: i32,
    },
}
impl Default for AbilityState {
    fn default() -> Self { Self::BasicAttack { cost: -100 } }
}

impl Component for AbilityState {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AbilityPool {
    pub primary: Option<AbilityState>,
    pub secondary: Option<AbilityState>,
    pub block: Option<AbilityState>,
    pub dodge: Option<AbilityState>,
}

impl Default for AbilityPool {
    fn default() -> Self {
        Self {
            primary: Some(AbilityState::default()),
            // primary: Some(AbilityState::TimedCombo),
            secondary: Some(AbilityState::BasicBlock),
            block: None,
            dodge: Some(AbilityState::Roll),
        }
    }
}

impl Component for AbilityPool {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
