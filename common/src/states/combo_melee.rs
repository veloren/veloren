use crate::{
    combat::{Attack, AttackEffect, DamageComponent},
    comp::{
        CharacterState, EnergyChange, EnergySource, MeleeAttack, PoiseChange, PoiseSource,
        StateUpdate,
    },
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
    Damage, DamageSource, GroupTarget, Knockback, KnockbackDir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stage<T> {
    /// Specifies which stage the combo attack is in
    pub stage: u32,
    /// Initial damage of stage
    pub base_damage: u32,
    /// Damage scaling per combo
    pub damage_increase: u32,
    /// Initial poise damage of stage
    pub base_poise_damage: u32,
    /// Poise damage scaling per combo
    pub poise_damage_increase: u32,
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
    /// Initial recover duration of stage (how long until character exits state)
    pub base_recover_duration: T,
    /// How much forward movement there is in the swing portion of the stage
    pub forward_movement: f32,
}

impl Stage<u64> {
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
            base_buildup_duration: Duration::from_millis(self.base_buildup_duration),
            base_swing_duration: Duration::from_millis(self.base_swing_duration),
            base_recover_duration: Duration::from_millis(self.base_recover_duration),
            forward_movement: self.forward_movement,
        }
    }

    pub fn adjusted_by_stats(mut self, power: f32, poise_strength: f32, speed: f32) -> Self {
        self.base_damage = (self.base_damage as f32 * power) as u32;
        self.damage_increase = (self.damage_increase as f32 * power) as u32;
        self.base_poise_damage = (self.base_poise_damage as f32 * poise_strength) as u32;
        self.poise_damage_increase = (self.poise_damage_increase as f32 * poise_strength) as u32;
        self.base_buildup_duration = (self.base_buildup_duration as f32 / speed) as u64;
        self.base_swing_duration = (self.base_swing_duration as f32 / speed) as u64;
        self.base_recover_duration = (self.base_recover_duration as f32 / speed) as u64;
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
    pub initial_energy_gain: u32,
    /// Max energy gain per strike
    pub max_energy_gain: u32,
    /// Energy gain increase per combo
    pub energy_increase: u32,
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
    /// What key is used to press ability
    pub ability_key: AbilityKey,
}
/// A sequence of attacks that can incrementally become faster and more
/// damaging.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Indicates what stage the combo is in
    pub stage: u32,
    /// Number of consecutive strikes
    pub combo: u32,
    /// Timer for each stage
    pub timer: Duration,
    /// Checks what section a stage is in
    pub stage_section: StageSection,
    /// Whether the state should go onto the next stage
    pub next_stage: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 0.3);
        if !ability_key_is_pressed(data, self.static_data.ability_key) {
            handle_interrupt(data, &mut update, self.static_data.is_interruptible);
            if let CharacterState::Roll(roll) = &mut update.character {
                roll.was_combo = Some((self.stage, self.combo));
            }
            match update.character {
                CharacterState::ComboMelee(_) => {},
                _ => {
                    return update;
                },
            }
        }

        let stage_index = (self.stage - 1) as usize;

        let speed_modifer = 1.0
            + self.static_data.max_speed_increase
                * (1.0 - self.static_data.speed_increase.powi(self.combo as i32));

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.stage_data[stage_index].base_buildup_duration {
                    // Build up
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0 * speed_modifer))
                            .unwrap_or_default(),
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

                    // Hit attempt
                    let damage = self.static_data.stage_data[stage_index].base_damage
                        + self
                            .static_data
                            .scales_from_combo
                            .min(self.combo / self.static_data.num_stages)
                            * self.static_data.stage_data[stage_index].damage_increase;

                    let poise_damage = self.static_data.stage_data[stage_index].base_poise_damage
                        + self
                            .static_data
                            .scales_from_combo
                            .min(self.combo / self.static_data.num_stages)
                            * self.static_data.stage_data[stage_index].poise_damage_increase;

                    let damage = Damage {
                        source: DamageSource::Melee,
                        value: damage as f32,
                    };
                    let knockback = AttackEffect::Knockback(Knockback {
                        strength: self.static_data.stage_data[stage_index].knockback,
                        direction: KnockbackDir::Away,
                    });
                    let damage = DamageComponent::new(damage, Some(GroupTarget::OutOfGroup))
                        .with_effect(knockback);
                    let attack = Attack::default().with_damage(damage).with_crit(0.5, 1.3);

                    data.updater.insert(data.entity, MeleeAttack {
                        attack,
                        range: self.static_data.stage_data[stage_index].range,
                        max_angle: self.static_data.stage_data[stage_index].angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                    });
                }
            },
            StageSection::Swing => {
                if self.timer < self.static_data.stage_data[stage_index].base_swing_duration {
                    // Forward movement
                    handle_forced_movement(
                        data,
                        &mut update,
                        ForcedMovement::Forward {
                            strength: self.static_data.stage_data[stage_index].forward_movement,
                        },
                        0.3,
                    );

                    // Swings
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0 * speed_modifer))
                            .unwrap_or_default(),
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
                    // Recovers
                    if ability_key_is_pressed(data, self.static_data.ability_key) {
                        // Checks if state will transition to next stage after recover
                        update.character = CharacterState::ComboMelee(Data {
                            static_data: self.static_data.clone(),
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0 * speed_modifer))
                                .unwrap_or_default(),
                            next_stage: true,
                            ..*self
                        });
                    } else {
                        update.character = CharacterState::ComboMelee(Data {
                            static_data: self.static_data.clone(),
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(data.dt.0 * speed_modifer))
                                .unwrap_or_default(),
                            ..*self
                        });
                    }
                } else if self.next_stage {
                    // Transitions to buildup section of next stage
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        stage: (self.stage % self.static_data.num_stages) + 1,
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        next_stage: false,
                        ..*self
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<MeleeAttack>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<MeleeAttack>(data.entity);
            },
        }

        // Grant energy on successful hit
        if let Some(attack) = data.melee_attack {
            if attack.applied && attack.hit_count > 0 {
                let energy = self.static_data.max_energy_gain.min(
                    self.static_data.initial_energy_gain
                        + self.combo * self.static_data.energy_increase,
                ) as i32;
                update.character = CharacterState::ComboMelee(Data {
                    static_data: self.static_data.clone(),
                    stage: self.stage,
                    combo: self.combo + 1,
                    timer: self.timer,
                    stage_section: self.stage_section,
                    next_stage: self.next_stage,
                });
                data.updater.remove::<MeleeAttack>(data.entity);
                update.energy.change_by(EnergyChange {
                    amount: energy,
                    source: EnergySource::HitEnemy,
                });
            }
        }

        update
    }
}
