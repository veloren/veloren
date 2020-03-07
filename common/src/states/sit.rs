use super::utils::*;
use crate::{
    comp::{CharacterState, StateUpdate},
    sys::character_state::JoinData,
};
use std::collections::VecDeque;

pub fn behavior(data: &JoinData) -> StateUpdate {
    let mut update = StateUpdate {
        character: *data.character,
        pos: *data.pos,
        vel: *data.vel,
        ori: *data.ori,
        energy: *data.energy,
        local_events: VecDeque::new(),
        server_events: VecDeque::new(),
    };

    handle_wield(data, &mut update);

    // Try to Fall/Stand up/Move
    if !data.physics.on_ground
        || data.inputs.sit.is_just_pressed()
        || data.inputs.move_dir.magnitude_squared() > 0.0
    {
        update.character = CharacterState::Idle {};
    }

    update
}
