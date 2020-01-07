use crate::comp::{
    ActionState::Attack, AttackKind::BasicAttack, EcsStateData, ItemKind::Tool, StateHandler,
    StateUpdate, ToolData,
};
use crate::util::state_utils::*;
use std::time::Duration;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct BasicAttackState {
    /// How long the state has until exitting
    pub remaining_duration: Duration,
}

impl StateHandler for BasicAttackState {
    fn new(ecs_data: &EcsStateData) -> Self {
        let tool_data =
            if let Some(Tool(data)) = ecs_data.stats.equipment.main.as_ref().map(|i| i.kind) {
                data
            } else {
                ToolData::default()
            };
        Self {
            remaining_duration: tool_data.attack_duration(),
        }
    }

    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        // Check if attack duration has expired
        if self.remaining_duration == Duration::default() {
            // If so, go back to wielding or idling
            update.character.action_state = attempt_wield(ecs_data.stats);
            return update;
        }

        // Otherwise, tick down remaining_duration, and keep rolling
        update.character.action_state = Attack(BasicAttack(Some(BasicAttackState {
            remaining_duration: self
                .remaining_duration
                .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                .unwrap_or_default(),
        })));

        return update;
    }
}
