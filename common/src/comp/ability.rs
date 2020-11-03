use crate::{
    comp::{
        item::{armor::Protection, Item, ItemKind},
        Body, CharacterState, EnergySource, Gravity, LightEmitter, Projectile, StateUpdate,
    },
    states::{
        utils::{AbilityKey, StageSection},
        *,
    },
    sys::character_behavior::JoinData,
    Knockback,
};
use arraygen::Arraygen;
use serde::{Deserialize, Serialize};
use specs::{Component, FlaggedStorage};
use specs_idvs::IdvStorage;
use std::time::Duration;
use vek::Vec3;

#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbilityType {
    BasicMelee,
    BasicRanged,
    Boost,
    ChargedMelee(StageSection),
    ChargedRanged,
    DashMelee(StageSection),
    BasicBlock,
    ComboMelee(StageSection, u32),
    LeapMelee(StageSection),
    SpinMelee(StageSection),
    Shockwave,
    BasicBeam,
    RepeaterRanged,
}

impl From<&CharacterState> for CharacterAbilityType {
    fn from(state: &CharacterState) -> Self {
        match state {
            CharacterState::BasicMelee(_) => Self::BasicMelee,
            CharacterState::BasicRanged(_) => Self::BasicRanged,
            CharacterState::Boost(_) => Self::Boost,
            CharacterState::DashMelee(data) => Self::DashMelee(data.stage_section),
            CharacterState::BasicBlock => Self::BasicBlock,
            CharacterState::LeapMelee(data) => Self::LeapMelee(data.stage_section),
            CharacterState::ComboMelee(data) => Self::ComboMelee(data.stage_section, data.stage),
            CharacterState::SpinMelee(data) => Self::SpinMelee(data.stage_section),
            CharacterState::ChargedMelee(data) => Self::ChargedMelee(data.stage_section),
            CharacterState::ChargedRanged(_) => Self::ChargedRanged,
            CharacterState::Shockwave(_) => Self::ChargedRanged,
            CharacterState::BasicBeam(_) => Self::BasicBeam,
            CharacterState::RepeaterRanged(_) => Self::RepeaterRanged,
            _ => Self::BasicMelee,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbility {
    BasicMelee {
        energy_cost: u32,
        buildup_duration: Duration,
        swing_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
        knockback: f32,
        range: f32,
        max_angle: f32,
    },
    BasicRanged {
        energy_cost: u32,
        buildup_duration: Duration,
        recover_duration: Duration,
        projectile: Projectile,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
        projectile_speed: f32,
    },
    RepeaterRanged {
        energy_cost: u32,
        movement_duration: Duration,
        buildup_duration: Duration,
        shoot_duration: Duration,
        recover_duration: Duration,
        leap: Option<f32>,
        projectile: Projectile,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
        projectile_speed: f32,
        reps_remaining: u32,
    },
    Boost {
        movement_duration: Duration,
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
        swing_duration: Duration,
        recover_duration: Duration,
        infinite_charge: bool,
        is_interruptible: bool,
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
        is_interruptible: bool,
    },
    LeapMelee {
        energy_cost: u32,
        buildup_duration: Duration,
        movement_duration: Duration,
        swing_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
        range: f32,
        max_angle: f32,
        knockback: f32,
        forward_leap_strength: f32,
        vertical_leap_strength: f32,
    },
    SpinMelee {
        buildup_duration: Duration,
        swing_duration: Duration,
        recover_duration: Duration,
        base_damage: u32,
        knockback: f32,
        range: f32,
        energy_cost: u32,
        is_infinite: bool,
        is_helicopter: bool,
        is_interruptible: bool,
        forward_speed: f32,
        num_spins: u32,
    },
    ChargedMelee {
        energy_cost: u32,
        energy_drain: u32,
        initial_damage: u32,
        max_damage: u32,
        initial_knockback: f32,
        max_knockback: f32,
        range: f32,
        max_angle: f32,
        charge_duration: Duration,
        swing_duration: Duration,
        recover_duration: Duration,
    },
    ChargedRanged {
        energy_cost: u32,
        energy_drain: u32,
        initial_damage: u32,
        max_damage: u32,
        initial_knockback: f32,
        max_knockback: f32,
        buildup_duration: Duration,
        charge_duration: Duration,
        recover_duration: Duration,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
        initial_projectile_speed: f32,
        max_projectile_speed: f32,
    },
    Shockwave {
        energy_cost: u32,
        buildup_duration: Duration,
        swing_duration: Duration,
        recover_duration: Duration,
        damage: u32,
        knockback: Knockback,
        shockwave_angle: f32,
        shockwave_vertical_angle: f32,
        shockwave_speed: f32,
        shockwave_duration: Duration,
        requires_ground: bool,
        move_efficiency: f32,
    },
    BasicBeam {
        buildup_duration: Duration,
        recover_duration: Duration,
        beam_duration: Duration,
        base_hps: u32,
        base_dps: u32,
        tick_rate: f32,
        range: f32,
        max_angle: f32,
        lifesteal_eff: f32,
        energy_regen: u32,
        energy_cost: u32,
        energy_drain: u32,
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
            CharacterAbility::LeapMelee { energy_cost, .. } => {
                update.vel.0.z >= 0.0
                    && update
                        .energy
                        .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                        .is_ok()
            },
            CharacterAbility::SpinMelee { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::ChargedRanged { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::ChargedMelee { energy_cost, .. } => update
                .energy
                .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                .is_ok(),
            CharacterAbility::RepeaterRanged {
                energy_cost, leap, ..
            } => {
                (leap.is_none() || update.vel.0.z >= 0.0)
                    && update
                        .energy
                        .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
                        .is_ok()
            },
            CharacterAbility::Shockwave { energy_cost, .. } => update
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

impl From<(&CharacterAbility, AbilityKey)> for CharacterState {
    fn from((ability, key): (&CharacterAbility, AbilityKey)) -> Self {
        match ability {
            CharacterAbility::BasicMelee {
                buildup_duration,
                swing_duration,
                recover_duration,
                base_damage,
                knockback,
                range,
                max_angle,
                energy_cost: _,
            } => CharacterState::BasicMelee(basic_melee::Data {
                static_data: basic_melee::StaticData {
                    buildup_duration: *buildup_duration,
                    swing_duration: *swing_duration,
                    recover_duration: *recover_duration,
                    base_damage: *base_damage,
                    knockback: *knockback,
                    range: *range,
                    max_angle: *max_angle,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicRanged {
                buildup_duration,
                recover_duration,
                projectile,
                projectile_body,
                projectile_light,
                projectile_gravity,
                projectile_speed,
                energy_cost: _,
            } => CharacterState::BasicRanged(basic_ranged::Data {
                static_data: basic_ranged::StaticData {
                    buildup_duration: *buildup_duration,
                    recover_duration: *recover_duration,
                    projectile: projectile.clone(),
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_gravity: *projectile_gravity,
                    projectile_speed: *projectile_speed,
                    ability_key: key,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::Boost {
                movement_duration,
                only_up,
            } => CharacterState::Boost(boost::Data {
                static_data: boost::StaticData {
                    movement_duration: *movement_duration,
                    only_up: *only_up,
                },
                timer: Duration::default(),
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
                swing_duration,
                recover_duration,
                infinite_charge,
                is_interruptible,
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
                    swing_duration: *swing_duration,
                    recover_duration: *recover_duration,
                    is_interruptible: *is_interruptible,
                },
                auto_charge: false,
                timer: Duration::default(),
                refresh_timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicBlock => CharacterState::BasicBlock,
            CharacterAbility::Roll => CharacterState::Roll(roll::Data {
                static_data: roll::StaticData {
                    buildup_duration: Duration::from_millis(100),
                    movement_duration: Duration::from_millis(300),
                    recover_duration: Duration::from_millis(100),
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                was_wielded: false, // false by default. utils might set it to true
                was_sneak: false,
            }),
            CharacterAbility::ComboMelee {
                stage_data,
                initial_energy_gain,
                max_energy_gain,
                energy_increase,
                speed_increase,
                max_speed_increase,
                is_interruptible,
            } => CharacterState::ComboMelee(combo_melee::Data {
                static_data: combo_melee::StaticData {
                    num_stages: stage_data.len() as u32,
                    stage_data: stage_data.clone(),
                    initial_energy_gain: *initial_energy_gain,
                    max_energy_gain: *max_energy_gain,
                    energy_increase: *energy_increase,
                    speed_increase: 1.0 - *speed_increase,
                    max_speed_increase: *max_speed_increase - 1.0,
                    is_interruptible: *is_interruptible,
                },
                stage: 1,
                combo: 0,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                next_stage: false,
            }),
            CharacterAbility::LeapMelee {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                swing_duration,
                recover_duration,
                base_damage,
                knockback,
                range,
                max_angle,
                forward_leap_strength,
                vertical_leap_strength,
            } => CharacterState::LeapMelee(leap_melee::Data {
                static_data: leap_melee::StaticData {
                    buildup_duration: *buildup_duration,
                    movement_duration: *movement_duration,
                    swing_duration: *swing_duration,
                    recover_duration: *recover_duration,
                    base_damage: *base_damage,
                    knockback: *knockback,
                    range: *range,
                    max_angle: *max_angle,
                    forward_leap_strength: *forward_leap_strength,
                    vertical_leap_strength: *vertical_leap_strength,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::SpinMelee {
                buildup_duration,
                swing_duration,
                recover_duration,
                base_damage,
                knockback,
                range,
                energy_cost,
                is_infinite,
                is_helicopter,
                is_interruptible,
                forward_speed,
                num_spins,
            } => CharacterState::SpinMelee(spin_melee::Data {
                static_data: spin_melee::StaticData {
                    buildup_duration: *buildup_duration,
                    swing_duration: *swing_duration,
                    recover_duration: *recover_duration,
                    base_damage: *base_damage,
                    knockback: *knockback,
                    range: *range,
                    energy_cost: *energy_cost,
                    is_infinite: *is_infinite,
                    is_helicopter: *is_helicopter,
                    is_interruptible: *is_interruptible,
                    forward_speed: *forward_speed,
                    num_spins: *num_spins,
                },
                timer: Duration::default(),
                spins_remaining: *num_spins - 1,
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::ChargedMelee {
                energy_cost,
                energy_drain,
                initial_damage,
                max_damage,
                initial_knockback,
                max_knockback,
                charge_duration,
                swing_duration,
                recover_duration,
                range,
                max_angle,
            } => CharacterState::ChargedMelee(charged_melee::Data {
                static_data: charged_melee::StaticData {
                    energy_cost: *energy_cost,
                    energy_drain: *energy_drain,
                    initial_damage: *initial_damage,
                    max_damage: *max_damage,
                    initial_knockback: *initial_knockback,
                    max_knockback: *max_knockback,
                    range: *range,
                    max_angle: *max_angle,
                    charge_duration: *charge_duration,
                    swing_duration: *swing_duration,
                    recover_duration: *recover_duration,
                },
                stage_section: StageSection::Charge,
                timer: Duration::default(),
                exhausted: false,
                charge_amount: 0.0,
            }),
            CharacterAbility::ChargedRanged {
                energy_cost: _,
                energy_drain,
                initial_damage,
                max_damage,
                initial_knockback,
                max_knockback,
                buildup_duration,
                charge_duration,
                recover_duration,
                projectile_body,
                projectile_light,
                projectile_gravity,
                initial_projectile_speed,
                max_projectile_speed,
            } => CharacterState::ChargedRanged(charged_ranged::Data {
                static_data: charged_ranged::StaticData {
                    buildup_duration: *buildup_duration,
                    charge_duration: *charge_duration,
                    recover_duration: *recover_duration,
                    energy_drain: *energy_drain,
                    initial_damage: *initial_damage,
                    max_damage: *max_damage,
                    initial_knockback: *initial_knockback,
                    max_knockback: *max_knockback,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_gravity: *projectile_gravity,
                    initial_projectile_speed: *initial_projectile_speed,
                    max_projectile_speed: *max_projectile_speed,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::RepeaterRanged {
                energy_cost: _,
                movement_duration,
                buildup_duration,
                shoot_duration,
                recover_duration,
                leap,
                projectile,
                projectile_body,
                projectile_light,
                projectile_gravity,
                projectile_speed,
                reps_remaining,
            } => CharacterState::RepeaterRanged(repeater_ranged::Data {
                static_data: repeater_ranged::StaticData {
                    movement_duration: *movement_duration,
                    buildup_duration: *buildup_duration,
                    shoot_duration: *shoot_duration,
                    recover_duration: *recover_duration,
                    leap: *leap,
                    projectile: projectile.clone(),
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_gravity: *projectile_gravity,
                    projectile_speed: *projectile_speed,
                },
                timer: Duration::default(),
                stage_section: StageSection::Movement,
                reps_remaining: *reps_remaining,
            }),
            CharacterAbility::Shockwave {
                energy_cost: _,
                buildup_duration,
                swing_duration,
                recover_duration,
                damage,
                knockback,
                shockwave_angle,
                shockwave_vertical_angle,
                shockwave_speed,
                shockwave_duration,
                requires_ground,
                move_efficiency,
            } => CharacterState::Shockwave(shockwave::Data {
                static_data: shockwave::StaticData {
                    buildup_duration: *buildup_duration,
                    swing_duration: *swing_duration,
                    recover_duration: *recover_duration,
                    damage: *damage,
                    knockback: *knockback,
                    shockwave_angle: *shockwave_angle,
                    shockwave_vertical_angle: *shockwave_vertical_angle,
                    shockwave_speed: *shockwave_speed,
                    shockwave_duration: *shockwave_duration,
                    requires_ground: *requires_ground,
                    move_efficiency: *move_efficiency,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::BasicBeam {
                buildup_duration,
                recover_duration,
                beam_duration,
                base_hps,
                base_dps,
                tick_rate,
                range,
                max_angle,
                lifesteal_eff,
                energy_regen,
                energy_cost,
                energy_drain,
            } => CharacterState::BasicBeam(basic_beam::Data {
                static_data: basic_beam::StaticData {
                    buildup_duration: *buildup_duration,
                    recover_duration: *recover_duration,
                    beam_duration: *beam_duration,
                    base_hps: *base_hps,
                    base_dps: *base_dps,
                    tick_rate: *tick_rate,
                    range: *range,
                    max_angle: *max_angle,
                    lifesteal_eff: *lifesteal_eff,
                    energy_regen: *energy_regen,
                    energy_cost: *energy_cost,
                    energy_drain: *energy_drain,
                    ability_key: key,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                particle_ori: None::<Vec3<f32>>,
                offset: 0.0,
            }),
        }
    }
}

impl Component for Loadout {
    type Storage = FlaggedStorage<Self, IdvStorage<Self>>;
}
