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

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AbilityPool {
    pub primary: Option<comp::CharacterState>,
    pub secondary: Option<comp::CharacterState>,
    pub block: Option<comp::CharacterState>,
    pub dodge: Option<comp::CharacterState>,
}

impl Default for AbilityPool {
    fn default() -> Self {
        Self {
            primary: Some(comp::CharacterState::BasicAttack(None)),
            secondary: None,
            block: None,
            dodge: Some(comp::CharacterState::Roll(None)),
        }
    }
}

impl Component for AbilityPool {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
