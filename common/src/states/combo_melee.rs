use crate::{
    comp::{Attacking, CharacterState, EnergySource, StateUpdate},
    states::utils::{StageSection, *},
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stage {
    /// Specifies which stage the combo attack is in
    pub stage: u32,
    /// Initial damage of stage
    pub base_damage: u32,
    /// Max damage of stage
    pub max_damage: u32,
    /// Damage scaling per combo
    pub damage_increase: u32,
    /// Knockback of stage
    pub knockback: f32,
    /// Range of attack
    pub range: f32,
    /// Angle of attack
    pub angle: f32,
    /// Initial buildup duration of stage (how long until state can deal damage)
    pub base_buildup_duration: Duration,
    /// Duration of stage spent in swing (controls animation stuff, and can also
    /// be used to handle movement separately to buildup)
    pub base_swing_duration: Duration,
    /// Initial recover duration of stage (how long until character exits state)
    pub base_recover_duration: Duration,
    /// How much forward movement there is in the swing portion of the stage
    pub forward_movement: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
/// Separated out to condense update portions of character state
pub struct StaticData {
    /// Indicates number of stages in combo
    pub num_stages: u32,
    /// Data for each stage
    pub stage_data: Vec<Stage>,
    /// Initial energy gain per strike
    pub initial_energy_gain: u32,
    /// Max energy gain per strike
    pub max_energy_gain: u32,
    /// Energy gain increase per combo
    pub energy_increase: u32,
    /// (100% - speed_increase) is percentage speed increases from current to
    /// max when combo increases
    pub speed_increase: f32,
    /// (100% + max_speed_increase) is the max attack speed
    pub max_speed_increase: f32,
    /// Whether the state can be interrupted by other abilities
    pub is_interruptible: bool,
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

        let stage_index = (self.stage - 1) as usize;

        // Allows for other states to interrupt this state
        if self.static_data.is_interruptible && !data.inputs.primary.is_pressed() {
            handle_interrupt(data, &mut update);
            match update.character {
                CharacterState::ComboMelee(_) => {},
                _ => {
                    return update;
                },
            }
        }

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.stage_data[stage_index].base_buildup_duration {
                    // Build up
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        stage: self.stage,
                        combo: self.combo,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(
                                (1.0 + self.static_data.max_speed_increase
                                    * (1.0
                                        - self.static_data.speed_increase.powi(self.combo as i32)))
                                    * data.dt.0,
                            ))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        next_stage: self.next_stage,
                    });
                } else {
                    // Transitions to swing section of stage
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        stage: self.stage,
                        combo: self.combo,
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        next_stage: self.next_stage,
                    });

                    // Hit attempt
                    data.updater.insert(data.entity, Attacking {
                        base_damage: self.static_data.stage_data[stage_index].max_damage.min(
                            self.static_data.stage_data[stage_index].base_damage
                                + self.combo / self.static_data.num_stages
                                    * self.static_data.stage_data[stage_index].damage_increase,
                        ),
                        base_heal: 0,
                        range: self.static_data.stage_data[stage_index].range,
                        max_angle: self.static_data.stage_data[stage_index].angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                        knockback: self.static_data.stage_data[stage_index].knockback,
                    });
                }
            },
            StageSection::Swing => {
                if self.timer < self.static_data.stage_data[stage_index].base_swing_duration {
                    // Forward movement
                    forward_move(
                        data,
                        &mut update,
                        0.3,
                        self.static_data.stage_data[stage_index].forward_movement,
                    );

                    // Swings
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        stage: self.stage,
                        combo: self.combo,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(
                                (1.0 + self.static_data.max_speed_increase
                                    * (1.0
                                        - self.static_data.speed_increase.powi(self.combo as i32)))
                                    * data.dt.0,
                            ))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        next_stage: self.next_stage,
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        stage: self.stage,
                        combo: self.combo,
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        next_stage: self.next_stage,
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.stage_data[stage_index].base_recover_duration {
                    // Recovers
                    if data.inputs.primary.is_pressed() {
                        // Checks if state will transition to next stage after recover
                        update.character = CharacterState::ComboMelee(Data {
                            static_data: self.static_data.clone(),
                            stage: self.stage,
                            combo: self.combo,
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(
                                    (1.0 + self.static_data.max_speed_increase
                                        * (1.0
                                            - self
                                                .static_data
                                                .speed_increase
                                                .powi(self.combo as i32)))
                                        * data.dt.0,
                                ))
                                .unwrap_or_default(),
                            stage_section: self.stage_section,
                            next_stage: true,
                        });
                    } else {
                        update.character = CharacterState::ComboMelee(Data {
                            static_data: self.static_data.clone(),
                            stage: self.stage,
                            combo: self.combo,
                            timer: self
                                .timer
                                .checked_add(Duration::from_secs_f32(
                                    (1.0 + self.static_data.max_speed_increase
                                        * (1.0
                                            - self
                                                .static_data
                                                .speed_increase
                                                .powi(self.combo as i32)))
                                        * data.dt.0,
                                ))
                                .unwrap_or_default(),
                            stage_section: self.stage_section,
                            next_stage: self.next_stage,
                        });
                    }
                } else if self.next_stage {
                    // Transitions to buildup section of next stage
                    update.character = CharacterState::ComboMelee(Data {
                        static_data: self.static_data.clone(),
                        stage: (self.stage % self.static_data.num_stages) + 1,
                        combo: self.combo,
                        timer: Duration::default(),
                        stage_section: StageSection::Buildup,
                        next_stage: false,
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                    // Make sure attack component is removed
                    data.updater.remove::<Attacking>(data.entity);
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
                // Make sure attack component is removed
                data.updater.remove::<Attacking>(data.entity);
            },
        }

        // Grant energy on successful hit
        if let Some(attack) = data.attacking {
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
                data.updater.remove::<Attacking>(data.entity);
                update.energy.change_by(energy, EnergySource::HitEnemy);
            }
        }

        update
    }
}
