use super::{
    EcsCharacterState, EcsStateUpdate, GlideHandler, MoveState::*, RunHandler, StandHandler,
    StateHandle, SwimHandler,
};
use super::{HUMANOID_AIR_ACCEL, HUMANOID_AIR_SPEED};
use vek::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct FallHandler;

impl StateHandle for FallHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
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

        // Check gliding
        if ecs_data.inputs.glide.is_pressed() {
            update.character.move_state = Glide(GlideHandler);

            return update;
        }

        // Not on ground, go to swim or fall
        if !ecs_data.physics.on_ground {
            // Check if in fluid to go to swimming or back to falling
            if ecs_data.physics.in_fluid {
                update.character.move_state = Swim(SwimHandler);

                return update;
            } else {
                update.character.move_state = Fall(FallHandler);

                return update;
            }
        }
        // On ground
        else {
            // Return to running or standing based on move inputs
            update.character.move_state = if ecs_data.inputs.move_dir.magnitude_squared() > 0.0 {
                Run(RunHandler)
            } else {
                Stand(StandHandler)
            };

            return update;
        }
    }
}
