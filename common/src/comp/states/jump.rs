use super::{EcsStateData, FallState, MoveState::*, StateHandle, StateUpdate};
use crate::event::LocalEvent;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct JumpState;

impl StateHandle for JumpState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        ecs_data
            .local_bus
            .emitter()
            .emit(LocalEvent::Jump(*ecs_data.entity));

        // Immediately go to falling state after jump impulse
        update.character.move_state = Fall(FallState);
        return update;
    }
}
