use super::{
    CharacterState, ECSStateData, ECSStateUpdate, MoveState::*, RunHandler, StandHandler,
    StateHandle,
};
use super::{HUMANOID_WATER_ACCEL, HUMANOID_WATER_SPEED};
use crate::sys::phys::GRAVITY;
use vek::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct SwimHandler;

impl StateHandle for SwimHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        let mut update = ECSStateUpdate {
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

        if ecs_data.inputs.jump.is_pressed() {
            update.vel.0.z =
                (update.vel.0.z + ecs_data.dt.0 * GRAVITY * 1.25).min(HUMANOID_WATER_SPEED);
        }

        if ecs_data.inputs.primary.is_pressed() {
            // TODO: PrimaryStart
        } else if ecs_data.inputs.secondary.is_pressed() {
            // TODO: SecondaryStart
        }

        // Not on ground
        if !ecs_data.physics.on_ground {
            update.character = CharacterState {
                action_state: ecs_data.character.action_state,
                move_state: Swim(SwimHandler),
            };

            return update;
        }
        // On ground
        else {
            // Return to running or standing based on move inputs
            update.character = CharacterState {
                action_state: ecs_data.character.action_state,
                move_state: if ecs_data.inputs.move_dir.magnitude_squared() > 0.0 {
                    Run(RunHandler)
                } else {
                    Stand(StandHandler)
                },
            };

            return update;
        }
    }
}
