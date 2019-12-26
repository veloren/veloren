use super::{
    ActionState::*, CharacterState, ECSStateData, ECSStateUpdate, FallHandler, JumpHandler,
    MoveState::*, RunHandler, StandHandler, StateHandle, SwimHandler,
};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct SitHandler;

impl StateHandle for SitHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        let mut update = ECSStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Falling
        // Idk, maybe the ground disappears,
        // suddenly maybe a water spell appears.
        // Can't hurt to be safe :shrug:
        if !ecs_data.physics.on_ground {
            if ecs_data.physics.in_fluid {
                update.character = CharacterState {
                    action_state: Idle,
                    move_state: Swim(SwimHandler),
                };

                return update;
            } else {
                update.character = CharacterState {
                    action_state: Idle,
                    move_state: Fall(FallHandler),
                };

                return update;
            }
        }
        // Jumping
        if ecs_data.inputs.jump.is_pressed() {
            update.character = CharacterState {
                action_state: Idle,
                move_state: Jump(JumpHandler),
            };

            return update;
        }

        // Moving
        if ecs_data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character = CharacterState {
                action_state: Idle,
                move_state: Run(RunHandler),
            };

            return update;
        }

        // Standing back up (unsitting)
        if ecs_data.inputs.sit.is_just_pressed() {
            update.character = CharacterState {
                action_state: Idle,
                move_state: Stand(StandHandler),
            };

            return update;
        }

        // no move_state has occurred
        update.character = CharacterState {
            action_state: Idle,
            move_state: Sit(SitHandler),
        };

        return update;
    }
}
