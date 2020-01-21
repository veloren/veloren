use super::utils::*;
use crate::comp::{CharacterState, EcsStateData, StateUpdate};
use crate::states::StateHandler;

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

        //handle_jump(ecs_data, &mut update);
        handle_wield(ecs_data, &mut update);

        // Try to Fall/Stand up/Move
        if !ecs_data.physics.on_ground
            || ecs_data.inputs.sit.is_just_pressed()
            || ecs_data.inputs.move_dir.magnitude_squared() > 0.0
        {
            update.character = CharacterState::Idle(None);
        }

        update
    }
}
