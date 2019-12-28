use super::{HUMANOID_ACCEL, HUMANOID_SPEED};
use crate::comp::{
    ClimbState, EcsStateData, GlideState, JumpState, MoveState::*, SitState, StateHandle,
    StateUpdate,
};
use crate::util::movement_utils::*;
use vek::vec::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct RunState;

impl StateHandle for RunState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Move player according to move_dir
        update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
            * ecs_data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) {
                HUMANOID_ACCEL
            } else {
                0.0
            };

        // Set direction based on move direction when on the ground
        let ori_dir = if update.character.action_state.is_attacking()
            || update.character.action_state.is_blocking()
        {
            Vec2::from(ecs_data.inputs.look_dir).normalized()
        } else {
            Vec2::from(update.vel.0)
        };

        if ori_dir.magnitude_squared() > 0.0001
            && (update.ori.0.normalized() - Vec3::from(ori_dir).normalized()).magnitude_squared()
                > 0.001
        {
            update.ori.0 =
                vek::ops::Slerp::slerp(update.ori.0, ori_dir.into(), 9.0 * ecs_data.dt.0);
        }

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

        // Try to glide
        if can_glide(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Glide(GlideState);
            return update;
        }

        // Update based on groundedness
        update.character.move_state =
            determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);

        return update;
    }
}
