use super::utils::*;
use crate::{
    comp::{character_state::OutputEvents, CharacterState, StateUpdate},
    states::{
        behavior::{CharacterBehavior, JoinData},
        wielding,
    },
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticData {
    /// Time required to draw weapon
    pub buildup_duration: Duration,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Data {
    /// Struct containing data that does not change over the course of the
    /// character state
    pub static_data: StaticData,
    /// Timer for each stage
    pub timer: Duration,
    pub is_sneaking: bool,
}

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData, output_events: &mut OutputEvents) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0, None);
        handle_move(data, &mut update, if self.is_sneaking { 0.4 } else { 1.0 });
        handle_jump(data, output_events, &mut update, 1.0);

        if self.timer < self.static_data.buildup_duration {
            // Draw weapon
            update.character = CharacterState::Equipping(Data {
                timer: tick_attack_or_default(data, self.timer, None),
                ..*self
            });
        } else {
            // Done
            update.character = CharacterState::Wielding(wielding::Data {
                is_sneaking: self.is_sneaking,
            });
        }

        update
    }
}
