use crate::comp;
use specs::{Component, FlaggedStorage, HashMapStorage, VecStorage};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum AbilityActionKind {
    Primary,
    Secondary,
    Dodge,
    Block,
    // UpdatePool?
}
impl Default for AbilityActionKind {
    fn default() -> Self {
        Self::Primary
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct AbilityAction(pub AbilityActionKind);

impl Component for AbilityAction {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Copy, Clone, Debug, Default, Serialize, Deserialize)]
pub struct AbilityPool {
    pub primary: Option<comp::AttackKind>,
    pub secondary: Option<comp::AttackKind>,
    pub block: Option<comp::BlockKind>,
    pub dodge: Option<comp::DodgeKind>,
}

impl Component for AbilityPool {
    type Storage = HashMapStorage<Self>;
}
