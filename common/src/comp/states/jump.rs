use super::{CharacterState, ECSStateData, ECSStateUpdate, FallHandler, MoveState::*, StateHandle};
use crate::event::LocalEvent;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct JumpHandler;

impl StateHandle for JumpHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        let mut update = ECSStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        ecs_data
            .local_bus
            .emitter()
            .emit(LocalEvent::Jump(*ecs_data.entity));

        update.character = CharacterState {
            action_state: ecs_data.character.action_state,
            move_state: Fall(FallHandler),
        };

        return update;
    }
}
