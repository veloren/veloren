use super::{EcsCharacterState, EcsStateUpdate, FallHandler, MoveState::*, StateHandle};
use crate::event::LocalEvent;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct JumpHandler;

impl StateHandle for JumpHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        ecs_data
            .local_bus
            .emitter()
            .emit(LocalEvent::Jump(*ecs_data.entity));

        update.character.move_state = Fall(FallHandler);

        return update;
    }
}
