use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_wield(data, &mut update);
        handle_jump(&data, &mut update);

        // Try to Fall/Stand up/Move
        if !data.physics.on_ground || data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character = CharacterState::Idle;
        }

        update
    }

    fn wield(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_wield(data, &mut update);
        update
    }

    fn sit(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        attempt_sit(data, &mut update);
        update
    }

    fn stand(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);
        // Try to Fall/Stand up/Move
        update.character = CharacterState::Idle;
        update
    }
}
