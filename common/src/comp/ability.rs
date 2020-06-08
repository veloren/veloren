use crate::{
    comp::{
        ability::Stage, item::Item, Body, CharacterState, EnergySource, Gravity, LightEmitter,
        Projectile, StateUpdate,
    },
    states::{triple_strike::*, *},
    sys::character_behavior::JoinData,
};
use specs::{Component, FlaggedStorage};
use specs_idvs::IDVStorage;
use std::time::Duration;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbilityType {
    BasicMelee,
    BasicRanged,
    Boost,
    DashMelee,
    BasicBlock,
    TripleStrike(Stage),
}

impl From<&CharacterState> for CharacterAbilityType {
    fn from(state: &CharacterState) -> Self {
        match state {
            CharacterState::BasicMelee(_) => Self::BasicMelee,
            CharacterState::BasicRanged(_) => Self::BasicRanged,
            CharacterState::Boost(_) => Self::Boost,
            CharacterState::DashMelee(_) => Self::DashMelee,
            CharacterState::BasicBlock => Self::BasicBlock,
            CharacterState::TripleStrike(data) => Self::TripleStrike(data.stage),
            _ => Self::BasicMelee,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbility {
    BasicMelee {
        energy_cost: u32,
        buildup_duration: Duration,
        recover_duration: Duration,
        base_healthchange: i32,
        range: f32,
        max_angle: f32,
    },
    BasicRanged {
        energy_cost: u32,
        holdable: bool,
        prepare_duration: Duration,
        recover_duration: Duration,
        projectile: Projectile,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
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
    TripleStrike {
        base_damage: u32,
        needs_timing: bool,
    },
}

impl CharacterAbility {
    /// Attempts to fulfill requirements, mutating `update` (taking energy) if
    /// applicable.
    pub fn requirements_paid(&self, data: &JoinData, update: &mut StateUpdate) -> bool {
        match self {
            CharacterAbility::TripleStrike { .. } => {
                data.physics.on_ground
                    && data.body.is_humanoid()
                    && data.inputs.look_dir.xy().magnitude_squared() > 0.01
            },
            CharacterAbility::Roll => {
                data.physics.on_ground
                    && data.body.is_humanoid()
                    && data.vel.0.xy().magnitude_squared() > 0.5
                    && update
                        .energy
                        .try_change_by(-220, EnergySource::Ability)
                        .is_ok()
            },
            CharacterAbility::DashMelee { .. } => update
                .energy
                .try_change_by(-700, EnergySource::Ability)
                .is_ok(),
            CharacterAbility::BasicMelee { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::BasicRanged { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            _ => true,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ItemConfig {
    pub item: Item,
    pub ability1: Option<CharacterAbility>,
    pub ability2: Option<CharacterAbility>,
    pub ability3: Option<CharacterAbility>,
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
    pub back: Option<Item>,
    pub ring: Option<Item>,
    pub neck: Option<Item>,
    pub lantern: Option<Item>,
    pub head: Option<Item>,
    pub tabard: Option<Item>,
}

impl From<&CharacterAbility> for CharacterState {
    fn from(ability: &CharacterAbility) -> Self {
        match ability {
            CharacterAbility::BasicMelee {
                buildup_duration,
                recover_duration,
                base_healthchange,
                range,
                max_angle,
                energy_cost: _,
            } => CharacterState::BasicMelee(basic_melee::Data {
                exhausted: false,
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                base_healthchange: *base_healthchange,
                range: *range,
                max_angle: *max_angle,
            }),
            CharacterAbility::BasicRanged {
                holdable,
                prepare_duration,
                recover_duration,
                projectile,
                projectile_body,
                projectile_light,
                projectile_gravity,
                energy_cost: _,
            } => CharacterState::BasicRanged(basic_ranged::Data {
                exhausted: false,
                prepare_timer: Duration::default(),
                holdable: *holdable,
                prepare_duration: *prepare_duration,
                recover_duration: *recover_duration,
                projectile: projectile.clone(),
                projectile_body: *projectile_body,
                projectile_light: *projectile_light,
                projectile_gravity: *projectile_gravity,
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
                remaining_duration: Duration::from_millis(700),
                was_wielded: false, // false by default. utils might set it to true
            }),
            CharacterAbility::TripleStrike {
                base_damage,
                needs_timing,
            } => CharacterState::TripleStrike(triple_strike::Data {
                base_damage: *base_damage,
                stage: triple_strike::Stage::First,
                stage_exhausted: false,
                stage_time_active: Duration::default(),
                initialized: false,
                transition_style: if *needs_timing {
                    TransitionStyle::Timed(TimingState::NotPressed)
                } else {
                    TransitionStyle::Hold(HoldingState::Holding)
                },
            }),
        }
    }
}

impl Component for Loadout {
    type Storage = FlaggedStorage<Self, IDVStorage<Self>>;
}
