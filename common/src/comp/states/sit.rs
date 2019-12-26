use crate::comp::{
    ActionState::*, EcsCharacterState, EcsStateUpdate, FallHandler, JumpHandler, MoveState::*,
    RunHandler, StandHandler, StateHandle, SwimHandler,
};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct SitHandler;

impl StateHandle for SitHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Prevent action state handling
        update.character.action_disabled = true;
        update.character.action_state = Idle;
        update.character.move_state = Sit(SitHandler);

        // Falling
        // Idk, maybe the ground disappears,
        // suddenly maybe a water spell appears.
        // Can't hurt to be safe :shrug:
        if !ecs_data.physics.on_ground {
            if ecs_data.physics.in_fluid {
                update.character.move_state = Swim(SwimHandler);

                update.character.action_disabled = false;
                return update;
            } else {
                update.character.move_state = Fall(FallHandler);

                update.character.action_disabled = false;
                return update;
            }
        }
        // Jumping
        if ecs_data.inputs.jump.is_pressed() {
            update.character.move_state = Jump(JumpHandler);

            update.character.action_disabled = false;
            return update;
        }

        // Moving
        if ecs_data.inputs.move_dir.magnitude_squared() > 0.0 {
            update.character.move_state = Run(RunHandler);

            update.character.action_disabled = false;
            return update;
        }

        // Standing back up (unsitting)
        if ecs_data.inputs.sit.is_just_pressed() {
            update.character.move_state = Stand(StandHandler);

            update.character.action_disabled = false;
            return update;
        }

        // No move has occurred, keep sitting
        return update;
    }
}
