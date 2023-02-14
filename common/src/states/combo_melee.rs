use crate::{
    combat::{Attack, AttackDamage, AttackEffect, CombatEffect, CombatRequirement},
    comp::{
        character_state::OutputEvents,
        melee::MultiTarget,
        tool::{Stats, ToolKind},
        CharacterState, Melee, StateUpdate,
    },
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
    /// Adds an effect onto the main damage of the attack
    pub damage_effect: Option<CombatEffect>,
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
            damage_effect: self.damage_effect,
        }
    }

    #[must_use]
    pub fn adjusted_by_stats(self, stats: Stats) -> Self {
        Self {
            stage: self.stage,
            base_damage: self.base_damage * stats.power,
            damage_increase: self.damage_increase * stats.power,
            base_poise_damage: self.base_poise_damage * stats.effect_power,
            poise_damage_increase: self.poise_damage_increase * stats.effect_power,
            knockback: self.knockback,
            range: self.range * stats.range,
            angle: self.angle,
            base_buildup_duration: self.base_buildup_duration / stats.speed,
            base_swing_duration: self.base_swing_duration / stats.speed,
            hit_timing: self.hit_timing,
            base_recover_duration: self.base_recover_duration / stats.speed,
            forward_movement: self.forward_movement,
            damage_kind: self.damage_kind,
            damage_effect: self.damage_effect.map(|de| de.adjusted_by_stats(stats)),
        }
    }

    // TODO: name it as using knockback
    #[must_use]
    pub fn modify_strike(mut self, knockback_mult: f32) -> Self {
        self.knockback *= knockback_mult;
        self
    }
}

// TODO: Completely rewrite this with skill tree rework. Don't bother converting
// to melee constructor.
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
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        let combo_counter = data.combo.map_or(0, |c| c.counter());

        handle_move(data, &mut update, 0.4);

        let stage_index = self.stage_index();

        let speed_modifier = 1.0
            + self.static_data.max_speed_increase
                * (1.0 - self.static_data.speed_increase.powi(combo_counter as i32));

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.stage_data[stage_index].base_buildup_duration {
                    handle_orientation(
                        data,
                        &mut update,
                        0.4 * self.static_data.ori_modifier,
                        None,
                    );

                    // Build up
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, Some(speed_modifier)),
                        ..*self
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: Duration::default(),
                        stage_section: StageSection::Action,
                        ..*self
                    });
                }
            },
            StageSection::Action => {
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
                            .min(combo_counter / self.static_data.num_stages)
                            as f32)
                            * self.static_data.stage_data[stage_index].damage_increase;

                    let poise = self.static_data.stage_data[stage_index].base_poise_damage
                        + (self
                            .static_data
                            .scales_from_combo
                            .min(combo_counter / self.static_data.num_stages)
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
                            + combo_counter as f32 * self.static_data.energy_increase,
                    );

                    let energy = AttackEffect::new(None, CombatEffect::EnergyReward(energy))
                        .with_requirement(CombatRequirement::AnyDamage);

                    let mut damage = AttackDamage::new(
                        Damage {
                            source: DamageSource::Melee,
                            kind: self.static_data.stage_data[stage_index].damage_kind,
                            value: damage,
                        },
                        Some(GroupTarget::OutOfGroup),
                        rand::random(),
                    );
                    if let Some(effect) = self.static_data.stage_data[stage_index].damage_effect {
                        damage = damage.with_effect(effect);
                    }

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
                        // TODO: Evaluate if we want to leave this true. State will be removed at
                        // some point anyways and this does preserve behavior
                        multi_target: Some(MultiTarget::Normal),
                        break_block: data
                            .inputs
                            .break_block_pos
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
                    handle_orientation(
                        data,
                        &mut update,
                        0.4 * self.static_data.ori_modifier,
                        None,
                    );

                    // Forward movement
                    handle_forced_movement(
                        data,
                        &mut update,
                        ForcedMovement::Forward(
                            self.static_data.stage_data[stage_index].forward_movement,
                        ),
                    );

                    // Swings
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, Some(speed_modifier)),
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
                    handle_orientation(
                        data,
                        &mut update,
                        0.8 * self.static_data.ori_modifier,
                        None,
                    );
                    // Recovers
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        timer: tick_attack_or_default(data, self.timer, Some(speed_modifier)),
                        ..*self
                    });
                } else {
                    // Done
                    if input_is_pressed(data, self.static_data.ability_info.input) {
                        reset_state(self, data, output_events, &mut update);
                    } else {
                        end_melee_ability(data, &mut update);
                    }
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                end_melee_ability(data, &mut update);
            },
        }

        // At end of state logic so an interrupt isn't overwritten
        handle_interrupts(data, &mut update, output_events);

        update
    }
}

impl Data {
    /// Index should be `self.stage - 1`, however in cases of client-server
    /// desync this can cause panics. This ensures that `self.stage - 1` is
    /// valid, and if it isn't, index of 0 is used, which is always safe.
    pub fn stage_index(&self) -> usize {
        self.static_data
            .stage_data
            .get(self.stage as usize - 1)
            .map_or(0, |_| self.stage as usize - 1)
    }
}

fn reset_state(
    data: &Data,
    join: &JoinData,
    output_events: &mut OutputEvents,
    update: &mut StateUpdate,
) {
    handle_input(
        join,
        output_events,
        update,
        data.static_data.ability_info.input,
    );

    if let CharacterState::ComboMelee(c) = &mut update.character {
        c.stage = (data.stage % data.static_data.num_stages) + 1;
    }
}
