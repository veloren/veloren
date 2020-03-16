use super::utils::*;
use crate::{
    comp::StateUpdate,
    sys::character_behavior::{CharacterBehavior, JoinData},
};
use std::collections::VecDeque;

// const BLOCK_ACCEL: f32 = 30.0;
// const BLOCK_SPEED: f32 = 75.0;

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct Data;

impl CharacterBehavior for Data {
    fn behavior(&self, data: &JoinData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *data.pos,
            vel: *data.vel,
            ori: *data.ori,
            energy: *data.energy,
            character: data.character.clone(),
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        handle_move(&data, &mut update);

        if !data.physics.on_ground
            || !(data.inputs.secondary.is_pressed() || data.inputs.primary.is_pressed())
        {
            attempt_wield(data, &mut update);
        }
        update
    }
}
