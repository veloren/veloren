use super::{ECSStateData, ECSStateUpdate, StateHandle};
use std::time::Duration;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct RollHandler {
    /// How long the state has until exitting
    remaining_duration: Duration,
}

impl StateHandle for RollHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        ECSStateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        }
    }
}
