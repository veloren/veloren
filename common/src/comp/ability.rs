use specs::{Component, DenseVecStorage, FlaggedStorage, HashMapStorage};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum AbilityState {
    BasicAttack,
    BasicBlock,
    Roll,
    ChargeAttack,
    TripleAttack,
}
impl Default for AbilityState {
    fn default() -> Self { Self::BasicAttack }
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
            primary: Some(AbilityState::BasicAttack),
            // primary: Some(AbilityState::TripleAttack),
            secondary: Some(AbilityState::BasicBlock),
            block: None,
            dodge: Some(AbilityState::Roll),
        }
    }
}

impl Component for AbilityPool {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
