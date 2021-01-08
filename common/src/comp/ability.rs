use crate::{
    assets::{self, Asset},
    comp::{
        inventory::item::tool::ToolKind, projectile::ProjectileConstructor, skills, Body,
        CharacterState, EnergySource, Gravity, LightEmitter, StateUpdate,
    },
    states::{
        behavior::JoinData,
        utils::{AbilityKey, StageSection},
        *,
    },
    Knockback,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
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
            CharacterState::Shockwave(_) => Self::Shockwave,
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
        scaled_damage: u32,
        base_knockback: f32,
        scaled_knockback: f32,
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
        immune_melee: bool,
    },
    ComboMelee {
        stage_data: Vec<combo_melee::Stage<u64>>,
        initial_energy_gain: u32,
        max_energy_gain: u32,
        energy_increase: u32,
        speed_increase: f32,
        max_speed_increase: f32,
        scales_from_combo: u32,
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
        scaled_damage: u32,
        initial_knockback: f32,
        scaled_knockback: f32,
        range: f32,
        max_angle: f32,
        speed: f32,
        charge_duration: u64,
        swing_duration: u64,
        hit_timing: f32,
        recover_duration: u64,
    },
    ChargedRanged {
        energy_cost: u32,
        energy_drain: u32,
        initial_damage: u32,
        scaled_damage: u32,
        initial_knockback: f32,
        scaled_knockback: f32,
        speed: f32,
        buildup_duration: u64,
        charge_duration: u64,
        recover_duration: u64,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_gravity: Option<Gravity>,
        initial_projectile_speed: f32,
        scaled_projectile_speed: f32,
        move_speed: f32,
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
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
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

    pub fn default_roll() -> CharacterAbility {
        CharacterAbility::Roll {
            energy_cost: 150,
            buildup_duration: 100,
            movement_duration: 250,
            recover_duration: 150,
            roll_strength: 2.5,
            immune_melee: false,
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
                *projectile = projectile.modified_projectile(power, 1_f32, 1_f32, power);
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
                *projectile = projectile.modified_projectile(power, 1_f32, 1_f32, power);
            },
            Boost {
                ref mut movement_duration,
                ..
            } => {
                *movement_duration = (*movement_duration as f32 / speed) as u64;
            },
            DashMelee {
                ref mut base_damage,
                ref mut scaled_damage,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ..
            } => {
                *base_damage = (*base_damage as f32 * power) as u32;
                *scaled_damage = (*scaled_damage as f32 * power) as u32;
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
                ref mut scaled_damage,
                speed: ref mut ability_speed,
                ref mut charge_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ..
            } => {
                *initial_damage = (*initial_damage as f32 * power) as u32;
                *scaled_damage = (*scaled_damage as f32 * power) as u32;
                *ability_speed *= speed;
                *charge_duration = (*charge_duration as f32 / speed) as u64;
                *swing_duration = (*swing_duration as f32 / speed) as u64;
                *recover_duration = (*recover_duration as f32 / speed) as u64;
            },
            ChargedRanged {
                ref mut initial_damage,
                ref mut scaled_damage,
                speed: ref mut ability_speed,
                ref mut buildup_duration,
                ref mut charge_duration,
                ref mut recover_duration,
                ..
            } => {
                *initial_damage = (*initial_damage as f32 * power) as u32;
                *scaled_damage = (*scaled_damage as f32 * power) as u32;
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

    pub fn get_energy_cost(&self) -> u32 {
        use CharacterAbility::*;
        match self {
            BasicMelee { energy_cost, .. }
            | BasicRanged { energy_cost, .. }
            | RepeaterRanged { energy_cost, .. }
            | DashMelee { energy_cost, .. }
            | Roll { energy_cost, .. }
            | LeapMelee { energy_cost, .. }
            | SpinMelee { energy_cost, .. }
            | ChargedMelee { energy_cost, .. }
            | ChargedRanged { energy_cost, .. }
            | Shockwave { energy_cost, .. }
            | BasicBeam { energy_cost, .. } => *energy_cost,
            BasicBlock | Boost { .. } | ComboMelee { .. } => 0,
        }
    }

    pub fn adjusted_by_skills(
        mut self,
        skills: &HashMap<skills::Skill, skills::Level>,
        tool: Option<ToolKind>,
    ) -> Self {
        use skills::Skill::{self, *};
        use CharacterAbility::*;
        match tool {
            Some(ToolKind::Sword) => {
                use skills::SwordSkill::*;
                match self {
                    ComboMelee {
                        ref mut is_interruptible,
                        ref mut speed_increase,
                        ref mut max_speed_increase,
                        ref stage_data,
                        ref mut max_energy_gain,
                        ref mut scales_from_combo,
                        ..
                    } => {
                        *is_interruptible = skills.contains_key(&Sword(InterruptingAttacks));
                        let speed_segments =
                            Sword(TsSpeed).get_max_level().map_or(1, |l| l + 1) as f32;
                        let speed_level = if skills.contains_key(&Sword(TsCombo)) {
                            skills
                                .get(&Sword(TsSpeed))
                                .copied()
                                .flatten()
                                .map_or(1, |l| l + 1) as f32
                        } else {
                            0.0
                        };
                        {
                            *speed_increase *= speed_level / speed_segments;
                            *max_speed_increase *= speed_level / speed_segments;
                        }
                        let energy_level =
                            if let Some(level) = skills.get(&Sword(TsRegen)).copied().flatten() {
                                level
                            } else {
                                0
                            };
                        {
                            *max_energy_gain = (*max_energy_gain as f32
                                * ((energy_level + 1) * stage_data.len() as u16 - 1) as f32
                                / ((Sword(TsRegen).get_max_level().unwrap() + 1)
                                    * stage_data.len() as u16
                                    - 1) as f32)
                                as u32;
                        }
                        *scales_from_combo = skills
                            .get(&Sword(TsDamage))
                            .copied()
                            .flatten()
                            .unwrap_or(0)
                            .into();
                    },
                    DashMelee {
                        ref mut is_interruptible,
                        ref mut energy_cost,
                        ref mut energy_drain,
                        ref mut base_damage,
                        ref mut scaled_damage,
                        ref mut forward_speed,
                        ref mut infinite_charge,
                        ..
                    } => {
                        *is_interruptible = skills.contains_key(&Sword(InterruptingAttacks));
                        if let Some(level) = skills.get(&Sword(DCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sword(DDrain)).copied().flatten() {
                            *energy_drain =
                                (*energy_drain as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sword(DDamage)).copied().flatten() {
                            *base_damage =
                                (*base_damage as f32 * 1.2_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sword(DScaling)).copied().flatten() {
                            *scaled_damage =
                                (*scaled_damage as f32 * 1.2_f32.powi(level.into())) as u32;
                        }
                        if skills.contains_key(&Sword(DSpeed)) {
                            *forward_speed *= 1.3;
                        }
                        *infinite_charge = skills.contains_key(&Sword(DInfinite));
                    },
                    SpinMelee {
                        ref mut is_interruptible,
                        ref mut base_damage,
                        ref mut swing_duration,
                        ref mut energy_cost,
                        ref mut num_spins,
                        ..
                    } => {
                        *is_interruptible = skills.contains_key(&Sword(InterruptingAttacks));
                        if let Some(level) = skills.get(&Sword(SDamage)).copied().flatten() {
                            *base_damage =
                                (*base_damage as f32 * 1.4_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sword(SSpeed)).copied().flatten() {
                            *swing_duration =
                                (*swing_duration as f32 * 0.8_f32.powi(level.into())) as u64;
                        }
                        if let Some(level) = skills.get(&Sword(SCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                        *num_spins =
                            skills.get(&Sword(SSpins)).copied().flatten().unwrap_or(0) as u32 + 1;
                    },
                    _ => {},
                }
            },
            Some(ToolKind::Axe) => {
                use skills::AxeSkill::*;
                match self {
                    ComboMelee {
                        ref mut speed_increase,
                        ref mut max_speed_increase,
                        ref mut stage_data,
                        ref mut max_energy_gain,
                        ref mut scales_from_combo,
                        ..
                    } => {
                        if !skills.contains_key(&Axe(DsCombo)) {
                            stage_data.pop();
                        }
                        let speed_segments = Axe(DsSpeed).get_max_level().unwrap_or(1) as f32;
                        let speed_level =
                            skills.get(&Axe(DsSpeed)).copied().flatten().unwrap_or(0) as f32;
                        {
                            *speed_increase *= speed_level / speed_segments;
                            *max_speed_increase *= speed_level / speed_segments;
                        }
                        let energy_level =
                            if let Some(level) = skills.get(&Axe(DsRegen)).copied().flatten() {
                                level
                            } else {
                                0
                            };
                        {
                            *max_energy_gain = (*max_energy_gain as f32
                                * ((energy_level + 1) * stage_data.len() as u16 - 1) as f32
                                / ((Axe(DsRegen).get_max_level().unwrap() + 1)
                                    * stage_data.len() as u16
                                    - 1) as f32)
                                as u32;
                        }
                        *scales_from_combo = skills
                            .get(&Axe(DsDamage))
                            .copied()
                            .flatten()
                            .unwrap_or(0)
                            .into();
                    },
                    SpinMelee {
                        ref mut base_damage,
                        ref mut swing_duration,
                        ref mut energy_cost,
                        ref mut is_infinite,
                        ref mut is_helicopter,
                        ..
                    } => {
                        *is_infinite = skills.contains_key(&Axe(SInfinite));
                        *is_helicopter = skills.contains_key(&Axe(SHelicopter));
                        if let Some(level) = skills.get(&Axe(SDamage)).copied().flatten() {
                            *base_damage =
                                (*base_damage as f32 * 1.3_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Axe(SSpeed)).copied().flatten() {
                            *swing_duration =
                                (*swing_duration as f32 * 0.8_f32.powi(level.into())) as u64;
                        }
                        if let Some(level) = skills.get(&Axe(SCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                    },
                    LeapMelee {
                        ref mut base_damage,
                        ref mut knockback,
                        ref mut energy_cost,
                        ref mut forward_leap_strength,
                        ref mut vertical_leap_strength,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Axe(LDamage)).copied().flatten() {
                            *base_damage =
                                (*base_damage as f32 * 1.35_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Axe(LKnockback)).copied().flatten() {
                            *knockback *= 1.4_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Axe(LCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Axe(LDistance)).copied().flatten() {
                            *forward_leap_strength *= 1.2_f32.powi(level.into());
                            *vertical_leap_strength *= 1.2_f32.powi(level.into());
                        }
                    },
                    _ => {},
                }
            },
            Some(ToolKind::Hammer) => {
                use skills::HammerSkill::*;
                match self {
                    ComboMelee {
                        ref mut speed_increase,
                        ref mut max_speed_increase,
                        ref mut stage_data,
                        ref mut max_energy_gain,
                        ref mut scales_from_combo,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Hammer(SsKnockback)).copied().flatten() {
                            *stage_data = (*stage_data)
                                .iter()
                                .map(|s| s.modify_strike(1.5_f32.powi(level.into())))
                                .collect::<Vec<combo_melee::Stage<u64>>>();
                        }
                        let speed_segments = Hammer(SsSpeed).get_max_level().unwrap_or(1) as f32;
                        let speed_level =
                            skills.get(&Hammer(SsSpeed)).copied().flatten().unwrap_or(0) as f32;
                        {
                            *speed_increase *= speed_level / speed_segments;
                            *max_speed_increase *= speed_level / speed_segments;
                        }
                        let energy_level =
                            if let Some(level) = skills.get(&Hammer(SsRegen)).copied().flatten() {
                                level
                            } else {
                                0
                            };
                        {
                            *max_energy_gain = (*max_energy_gain as f32
                                * ((energy_level + 1) * stage_data.len() as u16) as f32
                                / ((Hammer(SsRegen).get_max_level().unwrap() + 1)
                                    * stage_data.len() as u16)
                                    as f32) as u32;
                        }
                        *scales_from_combo = skills
                            .get(&Hammer(SsDamage))
                            .copied()
                            .flatten()
                            .unwrap_or(0)
                            .into();
                    },
                    ChargedMelee {
                        ref mut scaled_damage,
                        ref mut scaled_knockback,
                        ref mut energy_drain,
                        ref mut speed,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Hammer(CDamage)).copied().flatten() {
                            *scaled_damage =
                                (*scaled_damage as f32 * 1.25_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Hammer(CKnockback)).copied().flatten() {
                            *scaled_knockback *= 1.5_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Hammer(CDrain)).copied().flatten() {
                            *energy_drain =
                                (*energy_drain as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Hammer(CSpeed)).copied().flatten() {
                            *speed *= 1.25_f32.powi(level.into());
                        }
                    },
                    LeapMelee {
                        ref mut base_damage,
                        ref mut knockback,
                        ref mut energy_cost,
                        ref mut forward_leap_strength,
                        ref mut vertical_leap_strength,
                        ref mut range,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Hammer(LDamage)).copied().flatten() {
                            *base_damage =
                                (*base_damage as f32 * 1.4_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Hammer(LKnockback)).copied().flatten() {
                            *knockback *= 1.5_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Hammer(LCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Hammer(LDistance)).copied().flatten() {
                            *forward_leap_strength *= 1.25_f32.powi(level.into());
                            *vertical_leap_strength *= 1.25_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Hammer(LRange)).copied().flatten() {
                            *range += 1.0 * level as f32;
                        }
                    },
                    _ => {},
                }
            },
            Some(ToolKind::Bow) => {
                use skills::BowSkill::*;
                match self {
                    BasicRanged {
                        ref mut projectile,
                        ref mut projectile_speed,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Bow(ProjSpeed)).copied().flatten() {
                            *projectile_speed *= 1.5_f32.powi(level.into());
                        }
                        {
                            let damage_level =
                                skills.get(&Bow(BDamage)).copied().flatten().unwrap_or(0);
                            let regen_level =
                                skills.get(&Bow(BRegen)).copied().flatten().unwrap_or(0);
                            let power = 1.3_f32.powi(damage_level.into());
                            let regen = 1.5_f32.powi(regen_level.into());
                            *projectile =
                                projectile.modified_projectile(power, regen, 1_f32, 1_f32);
                        }
                    },
                    ChargedRanged {
                        ref mut scaled_damage,
                        ref mut scaled_knockback,
                        ref mut energy_drain,
                        ref mut speed,
                        ref mut initial_projectile_speed,
                        ref mut scaled_projectile_speed,
                        ref mut move_speed,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Bow(ProjSpeed)).copied().flatten() {
                            *initial_projectile_speed *= 1.5_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Bow(CDamage)).copied().flatten() {
                            *scaled_damage =
                                (*scaled_damage as f32 * 1.25_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Bow(CKnockback)).copied().flatten() {
                            *scaled_knockback *= 1.5_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Bow(CProjSpeed)).copied().flatten() {
                            *scaled_projectile_speed *= 1.2_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Bow(CDrain)).copied().flatten() {
                            *energy_drain =
                                (*energy_drain as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Bow(CSpeed)).copied().flatten() {
                            *speed *= 1.25_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Bow(CMove)).copied().flatten() {
                            *move_speed *= 1.25_f32.powi(level.into());
                        }
                    },
                    RepeaterRanged {
                        ref mut energy_cost,
                        ref mut buildup_duration,
                        ref mut projectile,
                        ref mut reps_remaining,
                        ref mut projectile_speed,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Bow(ProjSpeed)).copied().flatten() {
                            *projectile_speed *= 1.5_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Bow(RDamage)).copied().flatten() {
                            let power = 1.3_f32.powi(level.into());
                            *projectile =
                                projectile.modified_projectile(power, 1_f32, 1_f32, 1_f32);
                        }
                        if !skills.contains_key(&Bow(RGlide)) {
                            *buildup_duration = 1;
                        }
                        if let Some(level) = skills.get(&Bow(RArrows)).copied().flatten() {
                            *reps_remaining += level as u32;
                        }
                        if let Some(level) = skills.get(&Bow(RCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.75_f32.powi(level.into())) as u32;
                        }
                    },
                    _ => {},
                }
            },
            Some(ToolKind::Staff) => {
                use skills::StaffSkill::*;
                match self {
                    BasicRanged {
                        ref mut projectile, ..
                    } => {
                        if !skills.contains_key(&Staff(BExplosion)) {
                            *projectile = projectile.fireball_to_firebolt();
                        }
                        {
                            let damage_level =
                                skills.get(&Staff(BDamage)).copied().flatten().unwrap_or(0);
                            let regen_level =
                                skills.get(&Staff(BRegen)).copied().flatten().unwrap_or(0);
                            let range_level =
                                skills.get(&Staff(BRadius)).copied().flatten().unwrap_or(0);
                            let power = 1.2_f32.powi(damage_level.into());
                            let regen = 1.2_f32.powi(regen_level.into());
                            let range = 1.1_f32.powi(range_level.into());
                            *projectile =
                                projectile.modified_projectile(power, regen, range, 1_f32);
                        }
                    },
                    BasicBeam {
                        ref mut base_dps,
                        ref mut range,
                        ref mut energy_drain,
                        ref mut beam_duration,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Staff(FDamage)).copied().flatten() {
                            *base_dps = (*base_dps as f32 * 1.3_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Staff(FRange)).copied().flatten() {
                            let range_mod = 1.25_f32.powi(level.into());
                            *range *= range_mod;
                            // Duration modified to keep velocity constant
                            *beam_duration = (*beam_duration as f32 * range_mod) as u64;
                        }
                        if let Some(level) = skills.get(&Staff(FDrain)).copied().flatten() {
                            *energy_drain =
                                (*energy_drain as f32 * 0.8_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Staff(FVelocity)).copied().flatten() {
                            let velocity_increase = 1.25_f32.powi(level.into());
                            let duration_mod = 1.0 / (1.0 + velocity_increase);
                            *beam_duration = (*beam_duration as f32 * duration_mod) as u64;
                        }
                    },
                    Shockwave {
                        ref mut damage,
                        ref mut knockback,
                        ref mut shockwave_duration,
                        ref mut energy_cost,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Staff(SDamage)).copied().flatten() {
                            *damage = (*damage as f32 * 1.3_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Staff(SKnockback)).copied().flatten() {
                            *knockback = knockback.modify_strength(1.3_f32.powi(level.into()));
                        }
                        if let Some(level) = skills.get(&Staff(SRange)).copied().flatten() {
                            *shockwave_duration =
                                (*shockwave_duration as f32 * 1.2_f32.powi(level.into())) as u64;
                        }
                        if let Some(level) = skills.get(&Staff(SCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.8_f32.powi(level.into())) as u32;
                        }
                    },
                    _ => {},
                }
            },
            Some(ToolKind::Sceptre) => {
                use skills::SceptreSkill::*;
                match self {
                    BasicBeam {
                        ref mut base_hps,
                        ref mut base_dps,
                        ref mut lifesteal_eff,
                        ref mut range,
                        ref mut energy_regen,
                        ref mut energy_cost,
                        ref mut beam_duration,
                        ..
                    } => {
                        if let Some(level) = skills.get(&Sceptre(BHeal)).copied().flatten() {
                            *base_hps = (*base_hps as f32 * 1.2_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sceptre(BDamage)).copied().flatten() {
                            *base_dps = (*base_dps as f32 * 1.3_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sceptre(BRange)).copied().flatten() {
                            let range_mod = 1.25_f32.powi(level.into());
                            *range *= range_mod;
                            // Duration modified to keep velocity constant
                            *beam_duration = (*beam_duration as f32 * range_mod) as u64;
                        }
                        if let Some(level) = skills.get(&Sceptre(BLifesteal)).copied().flatten() {
                            *lifesteal_eff *= 1.5_f32.powi(level.into());
                        }
                        if let Some(level) = skills.get(&Sceptre(BRegen)).copied().flatten() {
                            *energy_regen =
                                (*energy_regen as f32 * 1.1_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sceptre(BCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.9_f32.powi(level.into())) as u32;
                        }
                    },
                    BasicRanged {
                        ref mut energy_cost,
                        ref mut projectile,
                        ref mut projectile_speed,
                        ..
                    } => {
                        {
                            let heal_level =
                                skills.get(&Sceptre(PHeal)).copied().flatten().unwrap_or(0);
                            let damage_level = skills
                                .get(&Sceptre(PDamage))
                                .copied()
                                .flatten()
                                .unwrap_or(0);
                            let range_level = skills
                                .get(&Sceptre(PRadius))
                                .copied()
                                .flatten()
                                .unwrap_or(0);
                            let heal = 1.2_f32.powi(heal_level.into());
                            let power = 1.2_f32.powi(damage_level.into());
                            let range = 1.4_f32.powi(range_level.into());
                            *projectile = projectile.modified_projectile(power, 1_f32, range, heal);
                        }
                        if let Some(level) = skills.get(&Sceptre(PCost)).copied().flatten() {
                            *energy_cost =
                                (*energy_cost as f32 * 0.8_f32.powi(level.into())) as u32;
                        }
                        if let Some(level) = skills.get(&Sceptre(PProjSpeed)).copied().flatten() {
                            *projectile_speed *= 1.25_f32.powi(level.into());
                        }
                    },
                    _ => {},
                }
            },
            None => {
                use skills::RollSkill::*;
                if let CharacterAbility::Roll {
                    ref mut immune_melee,
                    ref mut energy_cost,
                    ref mut roll_strength,
                    ref mut movement_duration,
                    ..
                } = self
                {
                    *immune_melee = skills.contains_key(&Skill::Roll(ImmuneMelee));
                    if let Some(level) = skills.get(&Skill::Roll(Cost)).copied().flatten() {
                        *energy_cost = (*energy_cost as f32 * 0.8_f32.powi(level.into())) as u32;
                    }
                    if let Some(level) = skills.get(&Skill::Roll(Strength)).copied().flatten() {
                        *roll_strength *= 1.3_f32.powi(level.into());
                    }
                    if let Some(level) = skills.get(&Skill::Roll(Duration)).copied().flatten() {
                        *movement_duration =
                            (*movement_duration as f32 * 1.2_f32.powi(level.into())) as u64;
                    }
                }
            },
            _ => {},
        }
        self
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
                    ability_key: key,
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
                scaled_damage,
                base_knockback,
                scaled_knockback,
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
                    scaled_damage: *scaled_damage,
                    base_knockback: *base_knockback,
                    scaled_knockback: *scaled_knockback,
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
                immune_melee,
            } => CharacterState::Roll(roll::Data {
                static_data: roll::StaticData {
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    movement_duration: Duration::from_millis(*movement_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    roll_strength: *roll_strength,
                    immune_melee: *immune_melee,
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
                scales_from_combo,
                is_interruptible,
            } => CharacterState::ComboMelee(combo_melee::Data {
                static_data: combo_melee::StaticData {
                    num_stages: stage_data.len() as u32,
                    stage_data: stage_data.iter().map(|stage| stage.to_duration()).collect(),
                    initial_energy_gain: *initial_energy_gain,
                    max_energy_gain: *max_energy_gain,
                    energy_increase: *energy_increase,
                    speed_increase: 1.0 - *speed_increase,
                    max_speed_increase: *max_speed_increase,
                    scales_from_combo: *scales_from_combo,
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
                    ability_key: key,
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
                scaled_damage,
                initial_knockback,
                scaled_knockback,
                speed,
                charge_duration,
                swing_duration,
                hit_timing,
                recover_duration,
                range,
                max_angle,
            } => CharacterState::ChargedMelee(charged_melee::Data {
                static_data: charged_melee::StaticData {
                    energy_cost: *energy_cost,
                    energy_drain: *energy_drain,
                    initial_damage: *initial_damage,
                    scaled_damage: *scaled_damage,
                    initial_knockback: *initial_knockback,
                    scaled_knockback: *scaled_knockback,
                    speed: *speed,
                    range: *range,
                    max_angle: *max_angle,
                    charge_duration: Duration::from_millis(*charge_duration),
                    swing_duration: Duration::from_millis(*swing_duration),
                    hit_timing: *hit_timing,
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
                scaled_damage,
                initial_knockback,
                scaled_knockback,
                speed,
                buildup_duration,
                charge_duration,
                recover_duration,
                projectile_body,
                projectile_light,
                projectile_gravity,
                initial_projectile_speed,
                scaled_projectile_speed,
                move_speed,
            } => CharacterState::ChargedRanged(charged_ranged::Data {
                static_data: charged_ranged::StaticData {
                    buildup_duration: Duration::from_millis(*buildup_duration),
                    charge_duration: Duration::from_millis(*charge_duration),
                    recover_duration: Duration::from_millis(*recover_duration),
                    energy_drain: *energy_drain,
                    initial_damage: *initial_damage,
                    scaled_damage: *scaled_damage,
                    speed: *speed,
                    initial_knockback: *initial_knockback,
                    scaled_knockback: *scaled_knockback,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_gravity: *projectile_gravity,
                    initial_projectile_speed: *initial_projectile_speed,
                    scaled_projectile_speed: *scaled_projectile_speed,
                    move_speed: *move_speed,
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
                    ability_key: key,
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
                    ability_key: key,
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
                offset: Vec3::zero(),
            }),
        }
    }
}
