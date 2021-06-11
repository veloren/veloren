use crate::{
    combat::{Attack, AttackDamage, AttackEffect, CombatBuff, CombatEffect, CombatRequirement},
    comp::{tool::ToolKind, CharacterState, Melee, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    Damage, DamageKind, DamageSource, GroupTarget, Knockback, KnockbackDir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stage<T> {
    /// Specifies which stage the combo attack is in
    pub stage: u32,
    /// Initial damage of stage
    pub base_damage: f32,
    /// Damage scaling per combo
    pub damage_increase: f32,
    /// Initial poise damage of stage
    pub base_poise_damage: f32,
    /// Poise damage scaling per combo
    pub poise_damage_increase: f32,
    /// Knockback of stage
    pub knockback: f32,
    /// Range of attack
    pub range: f32,
    /// Angle of attack
    pub angle: f32,
    /// Initial buildup duration of stage (how long until state can deal damage)
    pub base_buildup_duration: T,
    /// Duration of stage spent in swing (controls animation stuff, and can also
    /// be used to handle movement separately to buildup)
    pub base_swing_duration: T,
    /// At what fraction of the swing duration to apply the melee "hit"
    pub hit_timing: f32,
    /// Initial recover duration of stage (how long until character exits state)
    pub base_recover_duration: T,
    /// How much forward movement there is in the swing portion of the stage
    pub forward_movement: f32,
    /// What kind of damage this stage of the attack does
    pub damage_kind: DamageKind,
}

impl Stage<f32> {
    pub fn to_duration(self) -> Stage<Duration> {
        Stage::<Duration> {
            stage: self.stage,
            base_damage: self.base_damage,
            damage_increase: self.damage_increase,
            base_poise_damage: self.base_poise_damage,
            poise_damage_increase: self.poise_damage_increase,
            knockback: self.knockback,
            range: self.range,
            angle: self.angle,
            base_buildup_duration: Duration::from_secs_f32(self.base_buildup_duration),
            hit_timing: self.hit_timing,
            base_swing_duration: Duration::from_secs_f32(self.base_swing_duration),
            base_recover_duration: Duration::from_secs_f32(self.base_recover_duration),
            forward_movement: self.forward_movement,
            damage_kind: self.damage_kind,
        }
    }

    pub fn adjusted_by_stats(mut self, power: f32, poise_strength: f32, speed: f32) -> Self {
        self.base_damage *= power;
        self.damage_increase *= power;
        self.base_poise_damage *= poise_strength;
        self.poise_damage_increase *= poise_strength;
        self.base_buildup_duration /= speed;
        self.base_swing_duration /= speed;
        self.base_recover_duration /= speed;
        self
    }

    pub fn modify_strike(mut self, knockback_mult: f32) -> Self {
        self.knockback *= knockback_mult;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// Indicates number of stages in combo
    pub num_stages: u32,
    /// Data for each stage
    pub stage_data: Vec<Stage<Duration>>,
    /// Initial energy gain per strike
    pub initial_energy_gain: f32,
    /// Max energy gain per strike
    pub max_energy_gain: f32,
    /// Energy gain increase per combo
    pub energy_increase: f32,
    /// (100% - speed_increase) is percentage speed increases from current to
    /// max per combo increase
    pub speed_increase: f32,
    /// This value is the highest percentage speed can increase from the base
    /// speed
    pub max_speed_increase: f32,
    /// Number of times damage scales with combo
    pub scales_from_combo: u32,
    /// Whether the state can be interrupted by other abilities
    pub is_interruptible: bool,
    /// Adjusts turning rate during the attack
    pub ori_modifier: f32,
    /// What key is used to press ability
    pub ability_info: AbilityInfo,
}
/// A sequence of attacks that can incrementally become faster and more
/// damaging.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Whether the attack was executed already
    pub exhausted: bool,
    /// Indicates what stage the combo is in
    pub stage: u32,
    /// Timer for each stage
    pub timer: Duration,
    /// Checks what section a stage is in
    pub stage_section: StageSection,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.4);

        let stage_index = (self.stage - 1) as usize;

        let speed_modifer = 1.0
            + self.static_data.max_speed_increase
                * (1.0
                    - self
                        .static_data
                        .speed_increase
                        .powi(data.combo.counter() as i32));

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.stage_data[stage_index].base_buildup_duration {
                    handle_orientation(data, &mut update, 0.4 * self.static_data.ori_modifier);

                    // Build up
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, Some(speed_modifer)),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        ..*self
                    });
                }
            },
            StageSection::Swing => {
                if self.timer.as_secs_f32()
                    > self.static_data.stage_data[stage_index].hit_timing
                        * self.static_data.stage_data[stage_index]
                            .base_swing_duration
                            .as_secs_f32()
                    && !self.exhausted
                {
                    // Swing
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, None),
                        exhausted: true,
                        ..*self
                    });

                    let damage = self.static_data.stage_data[stage_index].base_damage
                        + (self
                            .static_data
                            .scales_from_combo
                            .min(data.combo.counter() / self.static_data.num_stages)
                            as f32)
                            * self.static_data.stage_data[stage_index].damage_increase;

                    let poise = self.static_data.stage_data[stage_index].base_poise_damage
                        + (self
                            .static_data
                            .scales_from_combo
                            .min(data.combo.counter() / self.static_data.num_stages)
                            as f32)
                            * self.static_data.stage_data[stage_index].poise_damage_increase;
                    let poise = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Poise(poise),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);

                    let knockback = AttackEffect::new(
                        Some(GroupTarget::OutOfGroup),
                        CombatEffect::Knockback(Knockback {
                            strength: self.static_data.stage_data[stage_index].knockback,
                            direction: KnockbackDir::Away,
                        }),
                    )
                    .with_requirement(CombatRequirement::AnyDamage);

                    let energy = self.static_data.max_energy_gain.min(
                        self.static_data.initial_energy_gain
                            + data.combo.counter() as f32 * self.static_data.energy_increase,
                    );

                    let energy = AttackEffect::new(None, CombatEffect::EnergyReward(energy))
                        .with_requirement(CombatRequirement::AnyDamage);

                    let buff = CombatEffect::Buff(CombatBuff::default_physical());

                    let damage = AttackDamage::new(
                        Damage {
                            source: DamageSource::Melee,
                            kind: self.static_data.stage_data[stage_index].damage_kind,
                            value: damage as f32,
                        },
                        Some(GroupTarget::OutOfGroup),
                    )
                    .with_effect(buff);

                    let (crit_chance, crit_mult) =
                        get_crit_data(data, self.static_data.ability_info);

                    let attack = Attack::default()
                        .with_damage(damage)
                        .with_crit(crit_chance, crit_mult)
                        .with_effect(energy)
                        .with_effect(poise)
                        .with_effect(knockback)
                        .with_combo_increment();

                    data.updater.insert(data.entity, Melee {
                        attack,
                        range: self.static_data.stage_data[stage_index].range,
                        max_angle: self.static_data.stage_data[stage_index].angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                        break_block: data
                            .inputs
                            .select_pos
                            .map(|p| {
                                (
                                    p.map(|e| e.floor() as i32),
                                    self.static_data.ability_info.tool,
                                )
                            })
                            .filter(|(_, tool)| tool == &Some(ToolKind::Pick)),
                    });
                } else if self.timer < self.static_data.stage_data[stage_index].base_swing_duration
                {
                    handle_orientation(data, &mut update, 0.4 * self.static_data.ori_modifier);

                    // Forward movement
                    handle_forced_movement(data, &mut update, ForcedMovement::Forward {
                        strength: self.static_data.stage_data[stage_index].forward_movement,
                    });

                    // Swings
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, Some(speed_modifer)),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.stage_data[stage_index].base_recover_duration {
                    handle_orientation(data, &mut update, 0.8 * self.static_data.ori_modifier);
                    // Recovers
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, Some(speed_modifer)),
                        ..*self
                    });
                } else {
                    // Done
                    if input_is_pressed(data, self.static_data.ability_info.input) {
                        reset_state(self, data, &mut update);
                    } else {
                        update.character = CharacterState::Wielding;
                    }
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<Melee>(data.entity);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        if !input_is_pressed(data, self.static_data.ability_info.input) {
            handle_state_interrupt(data, &mut update, self.static_data.is_interruptible);
        }

        update
    }
}

fn reset_state(data: &Data, join: &JoinData, update: &mut StateUpdate) {
    handle_input(join, update, data.static_data.ability_info.input);

    if let CharacterState::ComboMelee(c) = &mut update.character {
        c.stage = (data.stage % data.static_data.num_stages) + 1;
    }
}
