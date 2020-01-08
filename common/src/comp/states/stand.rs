use crate::comp::{EcsStateData, MoveState::*, StateHandler, StateUpdate};
use crate::util::state_utils::*;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State;

impl StateHandler for State {
    fn new(_ecs_data: &EcsStateData) -> Self {
        Self {}
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Try to sit
        if can_sit(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Sit(None);
            return update;
        }

        // Try to climb
        if can_climb(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Climb(None);
            return update;
        }

        // Try to jump
        if can_jump(ecs_data.physics, ecs_data.inputs) {
            update.character.move_state = Jump(None);
            return update;
        }

        // Check gliding
        if can_glide(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Glide(None);
            return update;
        }

        // Else update based on groundedness
        update.character.move_state =
            determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);

        return update;
    }
}
