use crate::comp::CharacterState;
use specs::{Component, DenseVecStorage, FlaggedStorage, HashMapStorage};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub enum CharacterAbility {
    BasicAttack {
        buildup_duration: Duration,
        recover_duration: Duration,
    },
    BasicBlock,
    Roll,
    ChargeAttack,
    TimedCombo {
        /// Amount of energy required to use ability
        cost: i32,
    },
}

impl Component for CharacterAbility {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Copy, Clone, Default, Debug, Serialize, Deserialize)]
pub struct AbilityPool {
    pub primary: Option<CharacterAbility>,
    pub secondary: Option<CharacterAbility>,
    pub block: Option<CharacterAbility>,
    pub dodge: Option<CharacterAbility>,
}

impl From<CharacterAbility> for CharacterState {
    fn from(ability: CharacterAbility) -> Self {
        match ability {
            CharacterAbility::BasicAttack {
                buildup_duration,
                recover_duration,
            } => CharacterState::BasicAttack {
                exhausted: false,
                buildup_duration,
                recover_duration,
            },
            CharacterAbility::BasicBlock { .. } => CharacterState::BasicBlock {},
            CharacterAbility::Roll { .. } => CharacterState::Roll {
                remaining_duration: Duration::from_millis(600),
            },
            CharacterAbility::ChargeAttack { .. } => CharacterState::ChargeAttack {
                remaining_duration: Duration::from_millis(600),
            },
            CharacterAbility::TimedCombo { .. } => CharacterState::TimedCombo {
                stage: 1,
                stage_time_active: Duration::default(),
                stage_exhausted: false,
                can_transition: false,
            },
        }
    }
}

impl Component for AbilityPool {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
