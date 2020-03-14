use crate::{
    comp::{CharacterState, ToolData},
    states::*,
};
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
        tool: ToolData,
        buildup_duration: Duration,
        recover_duration: Duration,
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
            } => CharacterState::BasicAttack(basic_attack::State {
                exhausted: false,
                buildup_duration,
                recover_duration,
            }),
            CharacterAbility::BasicBlock { .. } => CharacterState::BasicBlock {},
            CharacterAbility::Roll { .. } => CharacterState::Roll {
                remaining_duration: Duration::from_millis(600),
            },
            CharacterAbility::ChargeAttack { .. } => CharacterState::ChargeAttack {
                remaining_duration: Duration::from_millis(600),
            },
            CharacterAbility::TimedCombo {
                tool,
                buildup_duration,
                recover_duration,
            } => CharacterState::TimedCombo(timed_combo::State {
                tool,
                buildup_duration,
                recover_duration,
                stage: 0,
                stage_exhausted: false,
                stage_time_active: Duration::default(),
            }),
        }
    }
}

impl Component for AbilityPool {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
