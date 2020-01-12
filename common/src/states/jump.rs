use super::{EcsStateData, MoveState, StateHandler, StateUpdate};
use crate::event::LocalEvent;

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

        ecs_data
            .local_bus
            .emitter()
            .emit(LocalEvent::Jump(*ecs_data.entity));

        // Immediately go to falling state after jump impulse
        update.character.move_state = MoveState::Fall(None);
        return update;
    }
}
