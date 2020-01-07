use crate::comp::{EcsStateData, MoveState::*, StateHandler, StateUpdate};
use crate::sys::phys::GRAVITY;
use vek::{Vec2, Vec3};

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct SwimState;

const HUMANOID_WATER_ACCEL: f32 = 70.0;
const HUMANOID_WATER_SPEED: f32 = 120.0;

impl StateHandler for SwimState {
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

        // Update velocity
        update.vel.0 += Vec2::broadcast(ecs_data.dt.0)
            * ecs_data.inputs.move_dir
            * if update.vel.0.magnitude_squared() < HUMANOID_WATER_SPEED.powf(2.0) {
                HUMANOID_WATER_ACCEL
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
            update.ori.0 = vek::ops::Slerp::slerp(
                update.ori.0,
                ori_dir.into(),
                if ecs_data.physics.on_ground { 9.0 } else { 2.0 } * ecs_data.dt.0,
            );
        }

        if ecs_data.inputs.jump.is_pressed() && !ecs_data.inputs.jump.is_held_down() {
            update.vel.0.z =
                (update.vel.0.z + ecs_data.dt.0 * GRAVITY * 1.25).min(HUMANOID_WATER_SPEED);
        }

        // Not on ground
        if !ecs_data.physics.on_ground {
            update.character.move_state = Swim(None);
            return update;
        }
        // On ground
        else {
            // Return to running or standing based on move inputs
            update.character.move_state = if ecs_data.inputs.move_dir.magnitude_squared() > 0.0 {
                Run(None)
            } else {
                Stand(None)
            };

            return update;
        }
    }
}
