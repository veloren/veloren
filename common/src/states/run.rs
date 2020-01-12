use super::utils::*;
use crate::comp::{ActionState, EcsStateData, MoveState, StateUpdate};
use crate::states::StateHandler;
use vek::vec::{Vec2, Vec3};

const HUMANOID_ACCEL: f32 = 50.0;
const HUMANOID_SPEED: f32 = 120.0;

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

        // Move player according to move_dir
        update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
            * ecs_data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < HUMANOID_SPEED.powf(2.0) {
                HUMANOID_ACCEL
            } else {
                0.0
            };

        // Set direction based on move direction when on the ground
        let ori_dir =
            if let ActionState::Attack(_) | ActionState::Block(_) = update.character.action_state {
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
            update.character.move_state = MoveState::Sit(None);
            return update;
        }

        // Try to climb
        if can_climb(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = MoveState::Climb(None);
            return update;
        }

        // Try to jump
        if can_jump(ecs_data.physics, ecs_data.inputs) {
            update.character.move_state = MoveState::Jump(None);
            return update;
        }

        // Try to glide
        if can_glide(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = MoveState::Glide(None);
            return update;
        }

        // Update based on groundedness
        update.character.move_state =
            determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);

        return update;
    }
}
