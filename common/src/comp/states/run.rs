use super::{
    ClimbHandler, EcsCharacterState, EcsStateUpdate, FallHandler, GlideHandler, JumpHandler,
    MoveState::*, SitHandler, StandHandler, StateHandle, SwimHandler,
};
use super::{HUMANOID_ACCEL, HUMANOID_SPEED};
use vek::vec::{Vec2, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct RunHandler;

impl StateHandle for RunHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
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
        if ecs_data.inputs.sit.is_pressed()
            && ecs_data.physics.on_ground
            && ecs_data.body.is_humanoid()
        {
            update.character.move_state = Sit(SitHandler);

            return update;
        }

        // Try to climb
        if let (true, Some(_wall_dir)) = (
            ecs_data.inputs.climb.is_pressed() | ecs_data.inputs.climb_down.is_pressed()
                && ecs_data.body.is_humanoid(),
            ecs_data.physics.on_wall,
        ) {
            update.character.move_state = Climb(ClimbHandler);

            return update;
        }

        // Try to swim
        if !ecs_data.physics.on_ground && ecs_data.physics.in_fluid {
            update.character.move_state = Swim(SwimHandler);

            return update;
        }

        // While on ground ...
        if ecs_data.physics.on_ground {
            // Try to jump
            if ecs_data.inputs.jump.is_pressed() && !ecs_data.inputs.jump.is_held_down() {
                update.character.move_state = Jump(JumpHandler);

                return update;
            }
        }
        // While not on ground ...
        else {
            // Try to glide
            if ecs_data.physics.on_wall == None
                && ecs_data.inputs.glide.is_pressed()
                && !ecs_data.inputs.glide.is_held_down()
                && ecs_data.body.is_humanoid()
            {
                update.character.move_state = Glide(GlideHandler);

                return update;
            }
            update.character.move_state = Fall(FallHandler);

            return update;
        }

        if ecs_data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character.move_state = Run(RunHandler);

            return update;
        } else {
            update.character.move_state = Stand(StandHandler);

            return update;
        }
    }
}
