use super::utils::*;
use crate::{comp::StateUpdate, sys::character_state::JoinData};
use std::collections::VecDeque;

pub fn behavior(ecs_data: &JoinData) -> StateUpdate {
    let mut update = StateUpdate {
        character: *ecs_data.character,
        pos: *ecs_data.pos,
        vel: *ecs_data.vel,
        ori: *ecs_data.ori,
        energy: *ecs_data.energy,
        local_events: VecDeque::new(),
        server_events: VecDeque::new(),
    };

    handle_move(&ecs_data, &mut update);
    handle_jump(&ecs_data, &mut update);
    handle_sit(&ecs_data, &mut update);
    handle_climb(&ecs_data, &mut update);
    handle_glide(&ecs_data, &mut update);
    handle_unwield(&ecs_data, &mut update);
    handle_primary(&ecs_data, &mut update);
    handle_secondary(&ecs_data, &mut update);
    handle_dodge(&ecs_data, &mut update);

    update
}
