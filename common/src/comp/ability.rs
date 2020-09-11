use crate::{
    comp::{
        item::{armor::Protection, Item, ItemKind},
        Body, CharacterState, EnergySource, Gravity, LightEmitter, Projectile, StateUpdate,
    },
    states::{utils::StageSection, *},
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
    ComboMelee,
    LeapMelee,
    SpinMelee,
    GroundShockwave,
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
            CharacterState::ComboMelee(_) => Self::ComboMelee,
            CharacterState::SpinMelee(_) => Self::SpinMelee,
            CharacterState::ChargedRanged(_) => Self::ChargedRanged,
            CharacterState::GroundShockwave(_) => Self::ChargedRanged,
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
        knockback: f32,
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
        projectile_speed: f32,
    },
    Boost {
        duration: Duration,
        only_up: bool,
    },
    DashMelee {
        energy_cost: u32,
        base_damage: u32,
        max_damage: u32,
        base_knockback: f32,
        max_knockback: f32,
        range: f32,
        angle: f32,
        energy_drain: u32,
        forward_speed: f32,
        buildup_duration: Duration,
        charge_duration: Duration,
        infinite_charge: bool,
        recover_duration: Duration,
    },
    BasicBlock,
    Roll,
    ComboMelee {
        stage_data: Vec<combo_melee::Stage>,
        initial_energy_gain: u32,
        max_energy_gain: u32,
        energy_increase: u32,
        speed_increase: f32,
        max_speed_increase: f32,
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
        projectile_gravity: Option<Gravity>,
        initial_projectile_speed: f32,
        max_projectile_speed: f32,
    },
    GroundShockwave {
        energy_cost: u32,
        buildup_duration: Duration,
        recover_duration: Duration,
        damage: u32,
        knockback: f32,
        shockwave_angle: f32,
        shockwave_speed: f32,
        shockwave_duration: Duration,
        requires_ground: bool,
    },
}

impl CharacterAbility {
    /// Attempts to fulfill requirements, mutating `update` (taking energy) if
    /// applicable.
    pub fn requirements_paid(&self, data: &JoinData, update: &mut StateUpdate) -> bool {
        match self {
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
            CharacterAbility::GroundShockwave { energy_cost, .. } => update
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

impl From<Item> for ItemConfig {
    fn from(item: Item) -> Self {
        if let ItemKind::Tool(tool) = &item.kind() {
            let mut abilities = tool.get_abilities();
            let mut ability_drain = abilities.drain(..);

            return ItemConfig {
                item,
                ability1: ability_drain.next(),
                ability2: ability_drain.next(),
                ability3: ability_drain.next(),
                block_ability: Some(CharacterAbility::BasicBlock),
                dodge_ability: Some(CharacterAbility::Roll),
            };
        }

        unimplemented!("ItemConfig is currently only supported for Tools")
    }
}

#[derive(Arraygen, Clone, PartialEq, Default, Debug, Serialize, Deserialize)]
#[gen_array(pub fn get_armor: &Option<Item>)]
pub struct Loadout {
    pub active_item: Option<ItemConfig>,
    pub second_item: Option<ItemConfig>,

    pub lantern: Option<Item>,
    pub glider: Option<Item>,

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
                if let ItemKind::Armor(armor) = &item.kind() {
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
                knockback,
                range,
                max_angle,
                energy_cost: _,
            } => CharacterState::BasicMelee(basic_melee::Data {
                exhausted: false,
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                base_healthchange: *base_healthchange,
                knockback: *knockback,
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
                projectile_speed,
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
                projectile_speed: *projectile_speed,
            }),
            CharacterAbility::Boost { duration, only_up } => CharacterState::Boost(boost::Data {
                duration: *duration,
                only_up: *only_up,
            }),
            CharacterAbility::DashMelee {
                energy_cost: _,
                base_damage,
                max_damage,
                base_knockback,
                max_knockback,
                range,
                angle,
                energy_drain,
                forward_speed,
                buildup_duration,
                charge_duration,
                infinite_charge,
                recover_duration,
            } => CharacterState::DashMelee(dash_melee::Data {
                static_data: dash_melee::StaticData {
                    base_damage: *base_damage,
                    max_damage: *max_damage,
                    base_knockback: *base_knockback,
                    max_knockback: *max_knockback,
                    range: *range,
                    angle: *angle,
                    energy_drain: *energy_drain,
                    forward_speed: *forward_speed,
                    infinite_charge: *infinite_charge,
                    buildup_duration: *buildup_duration,
                    charge_duration: *charge_duration,
                    recover_duration: *recover_duration,
                },
                end_charge: false,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicBlock => CharacterState::BasicBlock,
            CharacterAbility::Roll => CharacterState::Roll(roll::Data {
                remaining_duration: Duration::from_millis(500),
                was_wielded: false, // false by default. utils might set it to true
            }),
            CharacterAbility::ComboMelee {
                stage_data,
                initial_energy_gain,
                max_energy_gain,
                energy_increase,
                speed_increase,
                max_speed_increase,
            } => CharacterState::ComboMelee(combo_melee::Data {
                stage: 1,
                num_stages: stage_data.len() as u32,
                combo: 0,
                stage_data: stage_data.clone(),
                initial_energy_gain: *initial_energy_gain,
                max_energy_gain: *max_energy_gain,
                energy_increase: *energy_increase,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                next_stage: false,
                speed_increase: *speed_increase,
                max_speed_increase: *max_speed_increase,
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
                projectile_gravity,
                initial_projectile_speed,
                max_projectile_speed,
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
                projectile_gravity: *projectile_gravity,
                initial_projectile_speed: *initial_projectile_speed,
                max_projectile_speed: *max_projectile_speed,
            }),
            CharacterAbility::GroundShockwave {
                energy_cost: _,
                buildup_duration,
                recover_duration,
                damage,
                knockback,
                shockwave_angle,
                shockwave_speed,
                shockwave_duration,
                requires_ground,
            } => CharacterState::GroundShockwave(ground_shockwave::Data {
                exhausted: false,
                buildup_duration: *buildup_duration,
                recover_duration: *recover_duration,
                damage: *damage,
                knockback: *knockback,
                shockwave_angle: *shockwave_angle,
                shockwave_speed: *shockwave_speed,
                shockwave_duration: *shockwave_duration,
                requires_ground: *requires_ground,
            }),
        }
    }
}

impl Component for Loadout {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
