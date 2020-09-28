use crate::{
    comp::{Attacking, CharacterState, StateUpdate},
    states::utils::{StageSection, *},
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long the state is moving
    pub movement_duration: Duration,
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the weapon swings
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub base_damage: u32,
    /// Knockback
    pub knockback: f32,
    /// Max range
    pub range: f32,
    /// Max angle (45.0 will give you a 90.0 angle window)
    pub max_angle: f32,
    /// Affects how far forward the player leaps
    pub forward_leap_strength: f32,
    /// Affects how high the player leaps
    pub vertical_leap_strength: f32,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
//#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Whether the attack can deal more damage
    pub exhausted: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.3);
        handle_jump(data, &mut update);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Buildup
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else {
                    // Transitions to leap portion of state
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Movement,
                        exhausted: self.exhausted,
                    });
                }
            },
            StageSection::Movement => {
                // Jumping
                update.vel.0 = Vec3::new(
                    data.inputs.look_dir.x,
                    data.inputs.look_dir.y,
                    self.static_data.vertical_leap_strength,
                ) * 2.0
                    * (1.0
                        - self.timer.as_secs_f32()
                            / self.static_data.movement_duration.as_secs_f32())
                    + (update.vel.0 * Vec3::new(2.0, 2.0, 0.0)
                        + 0.25 * data.inputs.move_dir.try_normalized().unwrap_or_default())
                    .try_normalized()
                    .unwrap_or_default()
                        * self.static_data.forward_leap_strength
                        * (1.0 - data.inputs.look_dir.z.abs());

                if self.timer < self.static_data.movement_duration {
                    // Movement duration
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else {
                    // Transitions to swing portion of state
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                        exhausted: self.exhausted,
                    });
                }
            },
            StageSection::Swing => {
                if self.timer < self.static_data.swing_duration {
                    // Swings weapons
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else {
                    // Transitions to recover portion
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        exhausted: self.exhausted,
                    });
                }
            },
            StageSection::Recover => {
                if !data.physics.on_ground {
                    // Falls
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
                    });
                } else if !self.exhausted {
                    // Hit attempt
                    data.updater.insert(data.entity, Attacking {
                        base_damage: self.static_data.base_damage,
                        base_heal: 0,
                        range: self.static_data.range,
                        max_angle: self.static_data.max_angle.to_radians(),
                        applied: false,
                        hit_count: 0,
                        knockback: self.static_data.knockback,
                    });

                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: true,
                    });
                } else if self.timer < self.static_data.recover_duration {
                    // Recovers
                    update.character = CharacterState::LeapMelee(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        exhausted: self.exhausted,
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

        update
    }
}
