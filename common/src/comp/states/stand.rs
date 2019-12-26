use super::{
    ClimbHandler, EcsCharacterState, EcsStateUpdate, FallHandler, GlideHandler, JumpHandler,
    MoveState::*, RunHandler, SitHandler, StateHandle, SwimHandler,
};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct StandHandler;

impl StateHandle for StandHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

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
            if ecs_data.inputs.jump.is_pressed() {
                update.character.move_state = Jump(JumpHandler);

                return update;
            }
        }
        // While not on ground ...
        else {
            // Try to glide
            if ecs_data.physics.on_wall == None
                && ecs_data.inputs.glide.is_pressed()
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
