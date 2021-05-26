use crate::{
    comp::{
        buff::{BuffChange, BuffKind},
        CharacterState, InputKind, StateUpdate,
    },
    event::ServerEvent,
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
    /// Affects whether you are immune to melee attacks while rolling
    pub immune_melee: bool,
    /// Information about the ability
    pub ability_info: AbilityInfo,
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
    /// Was in state with combo
    pub was_combo: Option<(InputKind, u32)>,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        // Smooth orientation
        handle_orientation(data, &mut update, 2.5);

        match self.stage_section {
            StageSection::Buildup => {
                handle_move(data, &mut update, 1.0);
                if self.timer < self.static_data.buildup_duration {
                    // Build up
                    update.character = CharacterState::Roll(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Remove burning effect if active
                    update.server_events.push_front(ServerEvent::Buff {
                        entity: data.entity,
                        buff_change: BuffChange::RemoveByKind(BuffKind::Burning),
                    });
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
                handle_forced_movement(data, &mut update, ForcedMovement::Forward {
                    strength: self.static_data.roll_strength
                        * ((1.0
                            - self.timer.as_secs_f32()
                                / self.static_data.movement_duration.as_secs_f32())
                            / 2.0
                            + 0.5),
                });

                if self.timer < self.static_data.movement_duration {
                    // Movement
                    update.character = CharacterState::Roll(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Keeps rolling if sufficient energy, else transitions to recover section of
                    // stage
                    if input_is_pressed(data, self.static_data.ability_info.input) {
                        reset_state(self, data, &mut update);
                    } else {
                        update.character = CharacterState::Roll(Data {
                            timer: Duration::default(),
                            stage_section: StageSection::Recover,
                            ..*self
                        });
                    }
                }
            },
            StageSection::Recover => {
                // Allows for jumps to interrupt recovery in roll
                if self.timer < self.static_data.recover_duration
                    && !handle_jump(data, &mut update, 1.5)
                {
                    // Recover
                    update.character = CharacterState::Roll(Data {
                        timer: tick_attack_or_default(data, self.timer, None),
                        ..*self
                    });
                } else {
                    // Done
                    if let Some((input, stage)) = self.was_combo {
                        if input_is_pressed(data, input) {
                            handle_input(data, &mut update, input);
                            // If other states are introduced that progress through stages, add them
                            // here
                            if let CharacterState::ComboMelee(c) = &mut update.character {
                                c.stage = stage;
                            }
                        } else {
                            update.character = CharacterState::Wielding;
                        }
                    } else if self.was_wielded {
                        update.character = CharacterState::Wielding;
                    } else if self.was_sneak {
                        update.character = CharacterState::Sneak;
                    } else {
                        update.character = CharacterState::Idle;
                    }
                }
            },
            _ => {
                // If it somehow ends up in an incorrect stage section
                update.character = CharacterState::Idle;
            },
        }

        update
    }
}

fn reset_state(data: &Data, join: &JoinData, update: &mut StateUpdate) {
    handle_input(join, update, data.static_data.ability_info.input);

    if let CharacterState::Roll(r) = &mut update.character {
        r.was_combo = data.was_combo;
        r.was_sneak = data.was_sneak;
        r.was_wielded = data.was_wielded;
        if matches!(r.stage_section, StageSection::Movement) {
            r.timer = Duration::default();
            r.stage_section = StageSection::Recover;
        } else {
            r.stage_section = StageSection::Movement;
        }
    }
}
