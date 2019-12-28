use crate::comp::{
    ClimbState, EcsStateData, GlideState, JumpState, MoveState::*, SitState, StateHandle,
    StateUpdate,
};
use crate::util::movement_utils::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct StandState;

impl StateHandle for StandState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Try to sit
        if can_sit(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Sit(SitState);
            return update;
        }

        // Try to climb
        if can_climb(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Climb(ClimbState);
            return update;
        }

        // Try to jump
        if can_jump(ecs_data.physics, ecs_data.inputs) {
            update.character.move_state = Jump(JumpState);
            return update;
        }

        // Check gliding
        if can_glide(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Glide(GlideState);
            return update;
        }

        // Else update based on groundedness
        update.character.move_state =
            determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);

        return update;
    }
}
