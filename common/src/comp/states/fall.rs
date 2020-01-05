use super::{HUMANOID_AIR_ACCEL, HUMANOID_AIR_SPEED};
use crate::comp::{ClimbState, EcsStateData, GlideState, MoveState::*, StateHandler, StateUpdate};

use crate::util::state_utils::*;
use vek::{Vec2, Vec3};

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct FallState;

impl StateHandler for FallState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        // Move player according to movement direction vector
        update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
            * ecs_data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < HUMANOID_AIR_SPEED.powf(2.0) {
                HUMANOID_AIR_ACCEL
            } else {
                0.0
            };

        // Set orientation vector based on direction of movement when on the ground
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
                vek::ops::Slerp::slerp(update.ori.0, ori_dir.into(), 2.0 * ecs_data.dt.0);
        }

        // Check to start climbing
        if can_climb(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = Climb(Some(ClimbState));
            return update;
        }

        // Check gliding
        if ecs_data.inputs.glide.is_pressed() {
            update.character.move_state = Glide(Some(GlideState));
            return update;
        }

        // Else update based on groundedness
        update.character.move_state =
            determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);

        return update;
    }
}
