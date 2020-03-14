use crate::{
    comp::{CharacterState, Item, ToolData},
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ItemConfig {
    pub item: Item,
    pub primary_ability: Option<CharacterAbility>,
    pub secondary_ability: Option<CharacterAbility>,
    pub block_ability: Option<CharacterAbility>,
    pub dodge_ability: Option<CharacterAbility>,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Loadout {
    pub active_item: Option<ItemConfig>,
    pub second_item: Option<ItemConfig>,
    // armor
}

impl From<CharacterAbility> for CharacterState {
    fn from(ability: CharacterAbility) -> Self {
        match ability {
            CharacterAbility::BasicAttack {
                buildup_duration,
                recover_duration,
            } => CharacterState::BasicAttack(basic_attack::Data {
                exhausted: false,
                buildup_duration,
                recover_duration,
            }),
            CharacterAbility::BasicBlock { .. } => CharacterState::BasicBlock,
            CharacterAbility::Roll { .. } => CharacterState::Roll(roll::Data {
                remaining_duration: Duration::from_millis(600),
            }),
            CharacterAbility::ChargeAttack { .. } => {
                CharacterState::ChargeAttack(charge_attack::Data {
                    remaining_duration: Duration::from_millis(600),
                })
            },
            CharacterAbility::TimedCombo {
                tool,
                buildup_duration,
                recover_duration,
            } => CharacterState::TimedCombo(timed_combo::Data {
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

impl Component for Loadout {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
