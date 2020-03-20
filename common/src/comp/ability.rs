use crate::{
    comp::{Body, CharacterState, EnergySource, Item, Projectile, StateUpdate},
    states::*,
    sys::character_behavior::JoinData,
};
use specs::{Component, DenseVecStorage, FlaggedStorage, HashMapStorage};
use std::time::Duration;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbility {
    BasicMelee {
        buildup_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
    },
    BasicRanged {
        recover_duration: Duration,
        projectile: Projectile,
        projectile_body: Body,
    },
    Boost {
        duration: Duration,
        only_up: bool,
    },
    DashMelee {
        buildup_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
    },
    BasicBlock,
    Roll,
    TimedCombo {
        buildup_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
    },
    TripleStrike {
        base_damage: u32,
    },
}

impl CharacterAbility {
    pub fn test_requirements(&self, data: &JoinData, update: &mut StateUpdate) -> bool {
        match self {
            CharacterAbility::Roll => {
                data.physics.on_ground
                    && !data.physics.in_fluid
                    && data.body.is_humanoid()
                    && update
                        .energy
                        .try_change_by(-200, EnergySource::Ability)
                        .is_ok()
            },
            CharacterAbility::DashMelee { .. } => {
                !data.physics.in_fluid
                    && update
                        .energy
                        .try_change_by(-300, EnergySource::Ability)
                        .is_ok()
            },
            _ => true,
        }
    }
}

impl Component for CharacterAbility {
    type Storage = DenseVecStorage<Self>;
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ItemConfig {
    pub item: Item,
    pub primary_ability: Option<CharacterAbility>,
    pub secondary_ability: Option<CharacterAbility>,
    pub block_ability: Option<CharacterAbility>,
    pub dodge_ability: Option<CharacterAbility>,
}

#[derive(Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
pub struct Loadout {
    pub active_item: Option<ItemConfig>,
    pub second_item: Option<ItemConfig>,

    pub shoulder: Option<Item>,
    pub chest: Option<Item>,
    pub belt: Option<Item>,
    pub hand: Option<Item>,
    pub pants: Option<Item>,
    pub foot: Option<Item>,
}

impl From<&CharacterAbility> for CharacterState {
    fn from(ability: &CharacterAbility) -> Self {
        match ability {
            CharacterAbility::BasicMelee {
                buildup_duration,
                recover_duration,
                base_damage,
            } => CharacterState::BasicMelee(basic_melee::Data {
                exhausted: false,
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                base_damage: *base_damage,
            }),
            CharacterAbility::BasicRanged {
                recover_duration,
                projectile,
                projectile_body,
            } => CharacterState::BasicRanged(basic_ranged::Data {
                exhausted: false,
                prepare_timer: Duration::default(),
                recover_duration: *recover_duration,
                projectile: projectile.clone(),
                projectile_body: *projectile_body,
            }),
            CharacterAbility::Boost { duration, only_up } => CharacterState::Boost(boost::Data {
                duration: *duration,
                only_up: *only_up,
            }),
            CharacterAbility::DashMelee {
                buildup_duration,
                recover_duration,
                base_damage,
            } => CharacterState::DashMelee(dash_melee::Data {
                initialize: true,
                exhausted: false,
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                base_damage: *base_damage,
            }),
            CharacterAbility::BasicBlock => CharacterState::BasicBlock,
            CharacterAbility::Roll => CharacterState::Roll(roll::Data {
                remaining_duration: Duration::from_millis(600),
            }),
            CharacterAbility::TimedCombo {
                buildup_duration,
                recover_duration,
                base_damage,
            } => CharacterState::TimedCombo(timed_combo::Data {
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                stage: 0,
                stage_exhausted: false,
                stage_time_active: Duration::default(),
                base_damage: *base_damage,
            }),
            CharacterAbility::TripleStrike { base_damage } => {
                CharacterState::TripleStrike(triple_strike::Data {
                    base_damage: *base_damage,
                    stage: 0,
                    stage_exhausted: false,
                    stage_time_active: Duration::default(),
                    should_transition: true,
                })
            },
        }
    }
}

impl Component for Loadout {
    type Storage = FlaggedStorage<Self, HashMapStorage<Self>>;
}
