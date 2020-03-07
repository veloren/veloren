use crate::comp::{ActionState, CharacterEntityData, MoveState, StateUpdate};

use super::utils::*;
use crate::states::StateHandler;
use vek::{Vec2, Vec3};

const HUMANOID_AIR_ACCEL: f32 = 10.0;
const HUMANOID_AIR_SPEED: f32 = 100.0;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct State;

impl StateHandler for State {
    fn new(_ecs_data: &CharacterEntityData) -> Self { Self {} }

    fn handle(&self, ecs_data: &CharacterEntityData) -> StateUpdate {
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
                vek::ops::Slerp::slerp(update.ori.0, ori_dir.into(), 2.0 * ecs_data.dt.0);
        }

        // Check to start climbing
        if can_climb(ecs_data.physics, ecs_data.inputs, ecs_data.body) {
            update.character.move_state = MoveState::Climb(None);
            return update;
        }

        // Check gliding
        if ecs_data.inputs.glide.is_pressed() {
            update.character.move_state = MoveState::Glide(None);
            return update;
        }

        // Else update based on groundedness
        update.character.move_state =
            determine_move_from_grounded_state(ecs_data.physics, ecs_data.inputs);

        update
    }
}
