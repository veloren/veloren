use crate::{
    comp::{shockwave, CharacterState, StateUpdate},
    event::ServerEvent,
    states::utils::*,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should deal damage
    pub buildup_duration: Duration,
    /// How long the state is swinging for
    pub swing_duration: Duration,
    /// How long the state has until exiting
    pub recover_duration: Duration,
    /// Base damage
    pub damage: u32,
    /// Knockback
    pub knockback: f32,
    /// Angle of the shockwave
    pub shockwave_angle: f32,
    /// Speed of the shockwave
    pub shockwave_speed: f32,
    /// How long the shockwave travels for
    pub shockwave_duration: Duration,
    /// Whether the shockwave requires the target to be on the ground
    pub requires_ground: bool,
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
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 0.05);

        match self.stage_section {
            StageSection::Buildup => {
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::Shockwave(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                    });
                } else {
                    // Attack
                    let properties = shockwave::Properties {
                        angle: self.static_data.shockwave_angle,
                        speed: self.static_data.shockwave_speed,
                        duration: self.static_data.shockwave_duration,
                        damage: self.static_data.damage,
                        knockback: self.static_data.knockback,
                        requires_ground: self.static_data.requires_ground,
                        owner: Some(*data.uid),
                    };
                    update.server_events.push_front(ServerEvent::Shockwave {
                        properties,
                        pos: *data.pos,
                        ori: *data.ori,
                    });

                    // Transitions to swing
                    update.character = CharacterState::Shockwave(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Swing,
                    });
                }
            },
            StageSection::Swing => {
                if self.timer < self.static_data.swing_duration {
                    // Swings
                    update.character = CharacterState::Shockwave(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                    });
                } else {
                    // Transitions to recover
                    update.character = CharacterState::Shockwave(Data {
                        static_data: self.static_data,
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.swing_duration {
                    // Recovers
                    update.character = CharacterState::Shockwave(Data {
                        static_data: self.static_data,
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        stage_section: self.stage_section,
                    });
                } else {
                    // Done
                    update.character = CharacterState::Wielding;
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Wielding;
            },
        }

        update
    }
}
