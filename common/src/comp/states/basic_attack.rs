use super::{ECSStateData, ECSStateUpdate, StateHandle};
use std::time::Duration;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct BasicAttackHandler {
    /// How long the state has until exitting
    remaining_duration: Duration,
}

impl StateHandle for BasicAttackHandler {
    fn handle(&self, ecs_data: &ECSStateData) -> ECSStateUpdate {
        return ECSStateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };
    }
}
