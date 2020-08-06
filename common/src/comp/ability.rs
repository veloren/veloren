use crate::{
    comp::{
        ability::Stage,
        item::{armor::Protection, Item, ItemKind},
        Body, CharacterState, EnergySource, Gravity, LightEmitter, Projectile, StateUpdate,
    },
    states::{triple_strike::*, *},
    sys::character_behavior::JoinData,
};
use arraygen::Arraygen;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbilityType {
    BasicMelee,
    BasicRanged,
    Boost,
    ChargedRanged,
    DashMelee,
    BasicBlock,
    TripleStrike(Stage),
    LeapMelee,
    SpinMelee,
}

impl From<&CharacterState> for CharacterAbilityType {
    fn from(state: &CharacterState) -> Self {
        match state {
            CharacterState::BasicMelee(_) => Self::BasicMelee,
            CharacterState::BasicRanged(_) => Self::BasicRanged,
            CharacterState::Boost(_) => Self::Boost,
            CharacterState::DashMelee(_) => Self::DashMelee,
            CharacterState::BasicBlock => Self::BasicBlock,
            CharacterState::LeapMelee(_) => Self::LeapMelee,
            CharacterState::TripleStrike(data) => Self::TripleStrike(data.stage),
            CharacterState::SpinMelee(_) => Self::SpinMelee,
            CharacterState::ChargedRanged(_) => Self::ChargedRanged,
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
        energy_cost: u32,
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
    LeapMelee {
        energy_cost: u32,
        movement_duration: Duration,
        buildup_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
    },
    SpinMelee {
        energy_cost: u32,
        buildup_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
    },
    ChargedRanged {
        energy_cost: u32,
        energy_drain: u32,
        initial_damage: u32,
        max_damage: u32,
        initial_knockback: f32,
        max_knockback: f32,
        prepare_duration: Duration,
        charge_duration: Duration,
        recover_duration: Duration,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
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
            CharacterAbility::DashMelee { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::BasicMelee { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::BasicRanged { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::LeapMelee { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::SpinMelee { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::ChargedRanged { energy_cost, .. } => update
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

#[derive(Arraygen, Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
#[gen_array(pub fn get_armor: &Option<Item>)]
pub struct Loadout {
    pub active_item: Option<ItemConfig>,
    pub second_item: Option<ItemConfig>,

    pub lantern: Option<Item>,

    #[in_array(get_armor)]
    pub shoulder: Option<Item>,
    #[in_array(get_armor)]
    pub chest: Option<Item>,
    #[in_array(get_armor)]
    pub belt: Option<Item>,
    #[in_array(get_armor)]
    pub hand: Option<Item>,
    #[in_array(get_armor)]
    pub pants: Option<Item>,
    #[in_array(get_armor)]
    pub foot: Option<Item>,
    #[in_array(get_armor)]
    pub back: Option<Item>,
    #[in_array(get_armor)]
    pub ring: Option<Item>,
    #[in_array(get_armor)]
    pub neck: Option<Item>,
    #[in_array(get_armor)]
    pub head: Option<Item>,
    #[in_array(get_armor)]
    pub tabard: Option<Item>,
}

impl Loadout {
    pub fn get_damage_reduction(&self) -> f32 {
        let protection = self
            .get_armor()
            .iter()
            .flat_map(|armor| armor.as_ref())
            .filter_map(|item| {
                if let ItemKind::Armor(armor) = &item.kind {
                    Some(armor.get_protection())
                } else {
                    None
                }
            })
            .map(|protection| match protection {
                Protection::Normal(protection) => Some(protection),
                Protection::Invincible => None,
            })
            .sum::<Option<f32>>();
        match protection {
            Some(dr) => dr / (60.0 + dr.abs()),
            None => 1.0,
        }
    }
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
                energy_cost: _,
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
                remaining_duration: Duration::from_millis(500),
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
            CharacterAbility::LeapMelee {
                energy_cost: _,
                movement_duration,
                buildup_duration,
                recover_duration,
                base_damage,
            } => CharacterState::LeapMelee(leap_melee::Data {
                initialize: true,
                exhausted: false,
                movement_duration: *movement_duration,
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                base_damage: *base_damage,
            }),
            CharacterAbility::SpinMelee {
                energy_cost,
                buildup_duration,
                recover_duration,
                base_damage,
            } => CharacterState::SpinMelee(spin_melee::Data {
                exhausted: false,
                energy_cost: *energy_cost,
                buildup_duration: *buildup_duration,
                buildup_duration_default: *buildup_duration,
                recover_duration: *recover_duration,
                recover_duration_default: *recover_duration,
                base_damage: *base_damage,
                // This isn't needed for it's continuous implementation, but is left in should this
                // skill be moved to the skillbar
                hits_remaining: 1,
                hits_remaining_default: 1, /* Should be the same value as hits_remaining, also
                                            * this value can be removed if ability moved to
                                            * skillbar */
            }),
            CharacterAbility::ChargedRanged {
                energy_cost: _,
                energy_drain,
                initial_damage,
                max_damage,
                initial_knockback,
                max_knockback,
                prepare_duration,
                charge_duration,
                recover_duration,
                projectile_body,
                projectile_light,
            } => CharacterState::ChargedRanged(charged_ranged::Data {
                exhausted: false,
                energy_drain: *energy_drain,
                initial_damage: *initial_damage,
                max_damage: *max_damage,
                initial_knockback: *initial_knockback,
                max_knockback: *max_knockback,
                prepare_duration: *prepare_duration,
                charge_duration: *charge_duration,
                charge_timer: Duration::default(),
                recover_duration: *recover_duration,
                projectile_body: *projectile_body,
                projectile_light: *projectile_light,
            }),
        }
    }
}

impl Component for Loadout {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
