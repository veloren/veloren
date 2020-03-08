use super::utils::*;
use crate::{comp::StateUpdate, sys::character_behavior::JoinData};
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

    handle_move(&data, &mut update);
    handle_jump(&data, &mut update);
    handle_sit(&data, &mut update);
    handle_climb(&data, &mut update);
    handle_glide(&data, &mut update);
    handle_unwield(&data, &mut update);
    handle_primary_input(&data, &mut update);
    handle_secondary_input(&data, &mut update);
    handle_dodge_input(&data, &mut update);

    update
}
