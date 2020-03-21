use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_behavior::{CharacterBehavior, JoinData},
};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate::from(data);

        handle_wield(data, &mut update);

        // Try to Fall/Stand up/Move
        if !data.physics.on_ground
            || data.inputs.sit.is_just_pressed()
            || data.inputs.move_dir.magnitude_squared() > 0.0
        {
            update.character = CharacterState::Idle;
        }

        update
    }
}
