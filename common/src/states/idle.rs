use super::utils::*;
use crate::comp::{EcsStateData, StateUpdate};
use std::collections::VecDeque;

use crate::states::StateHandler;
#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State;

impl StateHandler for State {
    fn new(_ecs_data: &EcsStateData) -> Self { Self {} }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            energy: *ecs_data.energy,
            local_events: VecDeque::new(),
            server_events: VecDeque::new(),
        };

        handle_move_dir(ecs_data, &mut update);
        handle_jump(ecs_data, &mut update);
        handle_wield(ecs_data, &mut update);
        handle_sit(ecs_data, &mut update);
        handle_climb(ecs_data, &mut update);
        handle_glide(ecs_data, &mut update);
        handle_dodge(ecs_data, &mut update);

        update
    }
}
