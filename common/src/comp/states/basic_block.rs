use super::{ECSStateData, ECSStateUpdate, StateHandle};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct BasicBlockHandler {
    /// How long the blocking state has been active
    active_duration: Duration,
}

impl StateHandle for BasicBlockHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        return ECSStateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };
    }
}
