use crate::{
    comp::{CharacterState, StateUpdate},
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
    util::Dir,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use vek::Vec3;

const ROLL_SPEED: f32 = 25.0;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should roll
    pub buildup_duration: Duration,
    /// How long state is rolling for
    pub movement_duration: Duration,
    /// How long it takes to recover from roll
    pub recover_duration: Duration,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    /// What section the character stage is in
    pub stage_section: StageSection,
    /// Had weapon
    pub was_wielded: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // Update velocity
        update.vel.0 = Vec3::new(0.0, 0.0, update.vel.0.z)
            + (update.vel.0 * Vec3::new(1.0, 1.0, 0.0)
                + 0.25 * data.inputs.move_dir.try_normalized().unwrap_or_default())
            .try_normalized()
            .unwrap_or_default()
                * ROLL_SPEED;

        // Smooth orientation
        update.ori.0 = Dir::slerp_to_vec3(update.ori.0, update.vel.0.xy().into(), 9.0 * data.dt.0);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::Roll(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        was_wielded: self.was_wielded,
                    });
                } else {
                    // Transitions to movement section of stage
                    update.character = CharacterState::Roll(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Movement,
                        was_wielded: self.was_wielded,
                    });
                }
            },
            StageSection::Movement => {
                if self.timer < self.static_data.movement_duration {
                    // Movement
                    update.character = CharacterState::Roll(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        was_wielded: self.was_wielded,
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::Roll(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        was_wielded: self.was_wielded,
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Build up
                    update.character = CharacterState::Roll(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                        was_wielded: self.was_wielded,
                    });
                } else {
                    // Done
                    if self.was_wielded {
                        update.character = CharacterState::Wielding;
                    } else {
                        update.character = CharacterState::Idle;
                    }
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                if self.was_wielded {
                    update.character = CharacterState::Wielding;
                } else {
                    update.character = CharacterState::Idle;
                }
            },
        }

        update
    }
}
