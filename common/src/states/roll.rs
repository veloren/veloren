use crate::{
    comp::{CharacterState, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        utils::*,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// How long until state should roll
    pub buildup_duration: Duration,
    /// How long state is rolling for
    pub movement_duration: Duration,
    /// How long it takes to recover from roll
    pub recover_duration: Duration,
    /// Affects the speed and distance of the roll
    pub roll_strength: f32,
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
    /// Was sneaking
    pub was_sneak: bool,
    /// Was combo, .0 is stage, .1 is combo counter
    pub was_combo: Option<(u32, u32)>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // Smooth orientation
        handle_orientation(data, &mut update, 1.0);

        match self.stage_section {
            StageSection::Buildup => {
                handle_move(data, &mut update, 1.0);
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::Roll(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to movement section of stage
                    update.character = CharacterState::Roll(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Movement,
                        ..*self
                    });
                }
            },
            StageSection::Movement => {
                // Update velocity
                handle_forced_movement(
                    data,
                    &mut update,
                    ForcedMovement::Forward {
                        strength: self.static_data.roll_strength,
                    },
                    0.0,
                );

                if self.timer < self.static_data.movement_duration {
                    // Movement
                    update.character = CharacterState::Roll(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Transitions to recover section of stage
                    update.character = CharacterState::Roll(Data {
                        timer: Duration::default(),
                        stage_section: StageSection::Recover,
                        ..*self
                    });
                }
            },
            StageSection::Recover => {
                if self.timer < self.static_data.recover_duration {
                    // Build up
                    update.character = CharacterState::Roll(Data {
                        timer: self
                            .timer
                            .checked_add(Duration::from_secs_f32(data.dt.0))
                            .unwrap_or_default(),
                        ..*self
                    });
                } else {
                    // Done
                    if self.was_wielded {
                        update.character = CharacterState::Wielding;
                        let combo_data = self.was_combo;

                        if let Some(combo_data) = combo_data {
                            continue_combo(data, &mut update, combo_data);
                        }
                    } else if self.was_sneak {
                        update.character = CharacterState::Sneak;
                    } else {
                        update.character = CharacterState::Idle;
                    }
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                if self.was_wielded {
                    update.character = CharacterState::Wielding;
                    let combo_data = self.was_combo;

                    if let Some(combo_data) = combo_data {
                        continue_combo(data, &mut update, combo_data);
                    }
                } else if self.was_sneak {
                    update.character = CharacterState::Sneak;
                } else {
                    update.character = CharacterState::Idle;
                }
            },
        }

        update
    }
}
