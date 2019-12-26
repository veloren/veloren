use super::{
    ActionState::*, CharacterState, ClimbHandler, ECSStateData, ECSStateUpdate, FallHandler,
    GlideHandler, JumpHandler, MoveState::*, RunHandler, SitHandler, StateHandle, SwimHandler,
};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct StandHandler;

impl StateHandle for StandHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        let mut update = ECSStateUpdate {
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
            update.character = CharacterState {
                move_state: Sit(SitHandler),
                action_state: update.character.action_state,
            };

            return update;
        }

        // Try to climb
        if let (true, Some(_wall_dir)) = (
            ecs_data.inputs.climb.is_pressed() | ecs_data.inputs.climb_down.is_pressed()
                && ecs_data.body.is_humanoid(),
            ecs_data.physics.on_wall,
        ) {
            update.character = CharacterState {
                action_state: update.character.action_state,
                move_state: Climb(ClimbHandler),
            };

            return update;
        }

        // Try to swim
        if !ecs_data.physics.on_ground && ecs_data.physics.in_fluid {
            update.character = CharacterState {
                action_state: update.character.action_state,
                move_state: Swim(SwimHandler),
            };

            return update;
        }

        // While on ground ...
        if ecs_data.physics.on_ground {
            // Try to jump
            if ecs_data.inputs.jump.is_pressed() {
                update.character = CharacterState {
                    action_state: update.character.action_state,
                    move_state: Jump(JumpHandler),
                };

                return update;
            }

            // // Try to charge
            // if inputs.charge.is_pressed() && !inputs.charge.is_held_down() {
            // }

            // Try to roll
            if ecs_data.inputs.roll.is_pressed() && ecs_data.body.is_humanoid() {
                // updater.insert(entity, DodgeStart);
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
                update.character = CharacterState {
                    action_state: Idle,
                    move_state: Glide(GlideHandler),
                };

                return update;
            }
            update.character = CharacterState {
                action_state: update.character.action_state,
                move_state: Fall(FallHandler),
            };

            return update;
        }

        if ecs_data.inputs.primary.is_pressed() {
            // updater.insert(entity, PrimaryStart);
        } else if ecs_data.inputs.secondary.is_pressed() {
            // updater.insert(entity, SecondaryStart);
        }

        if ecs_data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character = CharacterState {
                action_state: update.character.action_state,
                move_state: Run(RunHandler),
            };

            return update;
        } else {
            update.character = CharacterState {
                action_state: update.character.action_state,
                move_state: Stand(StandHandler),
            };

            return update;
        }
    }
}
