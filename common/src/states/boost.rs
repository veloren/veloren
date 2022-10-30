use crate::{
    comp::{character_state::OutputEvents, CharacterState, StateUpdate},
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
    pub movement_duration: Duration,
    pub only_up: bool,
    pub speed: f32,
    pub max_exit_velocity: f32,
    pub ability_info: AbilityInfo,
}

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_move(data, &mut update, 1.0);

        if self.timer < self.static_data.movement_duration {
            // Movement
            if self.static_data.only_up {
                update.vel.0.z += self.static_data.speed * data.dt.0;
            } else {
                update.vel.0 += *data.inputs.look_dir * self.static_data.speed * data.dt.0;
            }
            update.character = CharacterState::Boost(Data {
                timer: tick_attack_or_default(data, self.timer, None),
                ..*self
            });
        } else {
            // Done
            if input_is_pressed(data, self.static_data.ability_info.input) {
                reset_state(self, data, output_events, &mut update);
            } else {
                update.vel.0 = update.vel.0.try_normalized().unwrap_or_default()
                    * update
                        .vel
                        .0
                        .magnitude()
                        .min(self.static_data.max_exit_velocity);
                end_ability(data, &mut update);
            }
        }

        update
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
}
