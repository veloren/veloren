use crate::{
    assets::{self, Asset},
    comp::{
        item::{armor::Protection, tool::AbilityMap, Item, ItemKind},
        projectile::ProjectileConstructor,
        Body, CharacterState, EnergySource, Gravity, LightEmitter, StateUpdate,
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
use std::{fs::File, io::BufReader, time::Duration};
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
        buildup_duration: u64,
        swing_duration: u64,
        recover_duration: u64,
        base_damage: u32,
        knockback: f32,
        range: f32,
        max_angle: f32,
    },
    BasicRanged {
        energy_cost: u32,
        buildup_duration: u64,
        recover_duration: u64,
        projectile: ProjectileConstructor,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
        projectile_speed: f32,
        can_continue: bool,
    },
    RepeaterRanged {
        energy_cost: u32,
        movement_duration: u64,
        buildup_duration: u64,
        shoot_duration: u64,
        recover_duration: u64,
        leap: Option<f32>,
        projectile: ProjectileConstructor,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
        projectile_speed: f32,
        reps_remaining: u32,
    },
    Boost {
        movement_duration: u64,
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
        buildup_duration: u64,
        charge_duration: u64,
        swing_duration: u64,
        recover_duration: u64,
        infinite_charge: bool,
        is_interruptible: bool,
    },
    BasicBlock,
    Roll {
        energy_cost: u32,
        buildup_duration: u64,
        movement_duration: u64,
        recover_duration: u64,
        roll_strength: f32,
    },
    ComboMelee {
        stage_data: Vec<combo_melee::Stage<u64>>,
        initial_energy_gain: u32,
        max_energy_gain: u32,
        energy_increase: u32,
        speed_increase: f32,
        max_speed_increase: f32,
        is_interruptible: bool,
    },
    LeapMelee {
        energy_cost: u32,
        buildup_duration: u64,
        movement_duration: u64,
        swing_duration: u64,
        recover_duration: u64,
        base_damage: u32,
        range: f32,
        max_angle: f32,
        knockback: f32,
        forward_leap_strength: f32,
        vertical_leap_strength: f32,
    },
    SpinMelee {
        buildup_duration: u64,
        swing_duration: u64,
        recover_duration: u64,
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
        speed: f32,
        charge_duration: u64,
        swing_duration: u64,
        recover_duration: u64,
    },
    ChargedRanged {
        energy_cost: u32,
        energy_drain: u32,
        initial_damage: u32,
        max_damage: u32,
        initial_knockback: f32,
        max_knockback: f32,
        speed: f32,
        buildup_duration: u64,
        charge_duration: u64,
        recover_duration: u64,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
        initial_projectile_speed: f32,
        max_projectile_speed: f32,
    },
    Shockwave {
        energy_cost: u32,
        buildup_duration: u64,
        swing_duration: u64,
        recover_duration: u64,
        damage: u32,
        knockback: Knockback,
        shockwave_angle: f32,
        shockwave_vertical_angle: f32,
        shockwave_speed: f32,
        shockwave_duration: u64,
        requires_ground: bool,
        move_efficiency: f32,
    },
    BasicBeam {
        buildup_duration: u64,
        recover_duration: u64,
        beam_duration: u64,
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

impl Default for CharacterAbility {
    fn default() -> Self {
        CharacterAbility::BasicMelee {
            energy_cost: 0,
            buildup_duration: 250,
            swing_duration: 250,
            recover_duration: 500,
            base_damage: 10,
            knockback: 0.0,
            range: 3.5,
            max_angle: 15.0,
        }
    }
}

impl Asset for CharacterAbility {
    const ENDINGS: &'static [&'static str] = &["ron"];

    fn parse(buf_reader: BufReader<File>, _specifier: &str) -> Result<Self, assets::Error> {
        ron::de::from_reader(buf_reader).map_err(assets::Error::parse_error)
    }
}

impl CharacterAbility {
    /// Attempts to fulfill requirements, mutating `update` (taking energy) if
    /// applicable.
    pub fn requirements_paid(&self, data: &JoinData, update: &mut StateUpdate) -> bool {
        match self {
            CharacterAbility::Roll { energy_cost, .. } => {
                data.physics.on_ground
                    && data.vel.0.xy().magnitude_squared() > 0.5
                    && update
                        .energy
                        .try_change_by(-(*energy_cost as i32), EnergySource::Ability)
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

    fn default_roll() -> CharacterAbility {
        CharacterAbility::Roll {
            energy_cost: 100,
            buildup_duration: 100,
            movement_duration: 250,
            recover_duration: 150,
            roll_strength: 2.5,
        }
    }

    pub fn adjusted_by_stats(mut self, power: f32, speed: f32) -> Self {
        use CharacterAbility::*;
        match self {
            BasicMelee {
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut base_damage,
                ..
            } => {
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *swing_duration = (*swing_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
                *base_damage = (*base_damage as f32 * power) as u32;
            },
            BasicRanged {
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut projectile,
                ..
            } => {
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
                *projectile = projectile.modified_projectile(power);
            },
            RepeaterRanged {
                ref mut movement_duration,
                ref mut buildup_duration,
                ref mut shoot_duration,
                ref mut recover_duration,
                ref mut projectile,
                ..
            } => {
                *movement_duration = (*movement_duration as f32 / speed) as u64;
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *shoot_duration = (*shoot_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
                *projectile = projectile.modified_projectile(power);
            },
            Boost {
                ref mut movement_duration,
                ..
            } => {
                *movement_duration = (*movement_duration as f32 / speed) as u64;
            },
            DashMelee {
                ref mut base_damage,
                ref mut max_damage,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ..
            } => {
                *base_damage = (*base_damage as f32 * power) as u32;
                *max_damage = (*max_damage as f32 * power) as u32;
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *swing_duration = (*swing_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
            },
            BasicBlock => {},
            Roll {
                ref mut buildup_duration,
                ref mut movement_duration,
                ref mut recover_duration,
                ..
            } => {
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *movement_duration = (*movement_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
            },
            ComboMelee {
                ref mut stage_data, ..
            } => {
                *stage_data = stage_data
                    .iter_mut()
                    .map(|s| s.adjusted_by_stats(power, speed))
                    .collect();
            },
            LeapMelee {
                ref mut buildup_duration,
                ref mut movement_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut base_damage,
                ..
            } => {
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *movement_duration = (*movement_duration as f32 / speed) as u64;
                *swing_duration = (*swing_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
                *base_damage = (*base_damage as f32 * power) as u32;
            },
            SpinMelee {
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut base_damage,
                ..
            } => {
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *swing_duration = (*swing_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
                *base_damage = (*base_damage as f32 * power) as u32;
            },
            ChargedMelee {
                ref mut initial_damage,
                ref mut max_damage,
                speed: ref mut ability_speed,
                ref mut charge_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ..
            } => {
                *initial_damage = (*initial_damage as f32 * power) as u32;
                *max_damage = (*max_damage as f32 * power) as u32;
                *ability_speed *= speed;
                *charge_duration = (*charge_duration as f32 / speed) as u64;
                *swing_duration = (*swing_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
            },
            ChargedRanged {
                ref mut initial_damage,
                ref mut max_damage,
                speed: ref mut ability_speed,
                ref mut buildup_duration,
                ref mut charge_duration,
                ref mut recover_duration,
                ..
            } => {
                *initial_damage = (*initial_damage as f32 * power) as u32;
                *max_damage = (*max_damage as f32 * power) as u32;
                *ability_speed *= speed;
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *charge_duration = (*charge_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
            },
            Shockwave {
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut damage,
                ..
            } => {
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *swing_duration = (*swing_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
                *damage = (*damage as f32 * power) as u32;
            },
            BasicBeam {
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut base_hps,
                ref mut base_dps,
                ref mut tick_rate,
                ..
            } => {
                *buildup_duration = (*buildup_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
                *base_hps = (*base_hps as f32 * power) as u32;
                *base_dps = (*base_dps as f32 * power) as u32;
                *tick_rate *= speed;
            },
        }
        self
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

impl From<(Item, &AbilityMap)> for ItemConfig {
    fn from((item, map): (Item, &AbilityMap)) -> Self {
        if let ItemKind::Tool(tool) = &item.kind() {
            let abilities = tool.get_abilities(map);

            return ItemConfig {
                item,
                ability1: Some(abilities.primary),
                ability2: Some(abilities.secondary),
                ability3: abilities.skills.get(0).cloned(),
                block_ability: None,
                dodge_ability: Some(CharacterAbility::default_roll()),
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
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    swing_duration: Duration::from_millis(*swing_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
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
                can_continue,
                energy_cost: _,
            } => CharacterState::BasicRanged(basic_ranged::Data {
                static_data: basic_ranged::StaticData {
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    projectile: *projectile,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_gravity: *projectile_gravity,
                    projectile_speed: *projectile_speed,
                    can_continue: *can_continue,
                    ability_key: key,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
                continue_next: false,
            }),
            CharacterAbility::Boost {
                movement_duration,
                only_up,
            } => CharacterState::Boost(boost::Data {
                static_data: boost::StaticData {
                    movement_duration: Duration::from_millis(*movement_duration),
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
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    charge_duration: Duration::from_millis(*charge_duration),
                    swing_duration: Duration::from_millis(*swing_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    is_interruptible: *is_interruptible,
                    ability_key: key,
                },
                auto_charge: false,
                timer: Duration::default(),
                refresh_timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicBlock => CharacterState::BasicBlock,
            CharacterAbility::Roll {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                recover_duration,
                roll_strength,
            } => CharacterState::Roll(roll::Data {
                static_data: roll::StaticData {
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    movement_duration: Duration::from_millis(*movement_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    roll_strength: *roll_strength,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                was_wielded: false, // false by default. utils might set it to true
                was_sneak: false,
                was_combo: None,
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
                    stage_data: stage_data.iter().map(|stage| stage.to_duration()).collect(),
                    initial_energy_gain: *initial_energy_gain,
                    max_energy_gain: *max_energy_gain,
                    energy_increase: *energy_increase,
                    speed_increase: 1.0 - *speed_increase,
                    max_speed_increase: *max_speed_increase - 1.0,
                    is_interruptible: *is_interruptible,
                    ability_key: key,
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
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    movement_duration: Duration::from_millis(*movement_duration),
                    swing_duration: Duration::from_millis(*swing_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
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
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    swing_duration: Duration::from_millis(*swing_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    base_damage: *base_damage,
                    knockback: *knockback,
                    range: *range,
                    energy_cost: *energy_cost,
                    is_infinite: *is_infinite,
                    is_helicopter: *is_helicopter,
                    is_interruptible: *is_interruptible,
                    forward_speed: *forward_speed,
                    num_spins: *num_spins,
                    ability_key: key,
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
                speed,
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
                    speed: *speed,
                    range: *range,
                    max_angle: *max_angle,
                    charge_duration: Duration::from_millis(*charge_duration),
                    swing_duration: Duration::from_millis(*swing_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    ability_key: key,
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
                speed,
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
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    charge_duration: Duration::from_millis(*charge_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    energy_drain: *energy_drain,
                    initial_damage: *initial_damage,
                    max_damage: *max_damage,
                    speed: *speed,
                    initial_knockback: *initial_knockback,
                    max_knockback: *max_knockback,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_gravity: *projectile_gravity,
                    initial_projectile_speed: *initial_projectile_speed,
                    max_projectile_speed: *max_projectile_speed,
                    ability_key: key,
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
                    movement_duration: Duration::from_millis(*movement_duration),
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    shoot_duration: Duration::from_millis(*shoot_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    leap: *leap,
                    projectile: *projectile,
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
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    swing_duration: Duration::from_millis(*swing_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    damage: *damage,
                    knockback: *knockback,
                    shockwave_angle: *shockwave_angle,
                    shockwave_vertical_angle: *shockwave_vertical_angle,
                    shockwave_speed: *shockwave_speed,
                    shockwave_duration: Duration::from_millis(*shockwave_duration),
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
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    beam_duration: Duration::from_millis(*beam_duration),
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
