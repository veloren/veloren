use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    states::behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Separated out to condense update portions of character state
#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StaticData {
    /// Time required to draw weapon
    pub buildup_duration: Duration,
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
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_orientation(data, &mut update, 1.0);
        handle_move(data, &mut update, 1.0);
        handle_jump(data, &mut update, 1.0);

        if self.timer < self.static_data.buildup_duration {
            // Draw weapon
            update.character = CharacterState::Equipping(Data {
                timer: tick_attack_or_default(data, self.timer, None),
                ..*self
            });
        } else {
            // Done
            update.character = CharacterState::Wielding;
        }

        update
    }
}
