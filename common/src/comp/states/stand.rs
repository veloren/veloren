use crate::comp::{
    ClimbState, EcsStateData, GlideState, JumpState, MoveState::*, SitState, StateHandler,
    StateUpdate,
};
use crate::util::state_utils::*;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct StandState;

impl StateHandler for StandState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Try to sit
        if can_sit(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Sit(Some(SitState));
            return update;
        }

        // Try to climb
        if can_climb(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Climb(Some(ClimbState));
            return update;
        }

        // Try to jump
        if can_jump(ecs_data.physics, ecs_data.inputs) {
            update.character.move_state = Jump(Some(JumpState));
            return update;
        }

        // Check gliding
        if can_glide(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Glide(Some(GlideState));
            return update;
        }

        // Else update based on groundedness
        update.character.move_state =
            determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);

        return update;
    }
}
