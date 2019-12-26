use super::TEMP_EQUIP_DELAY;
use crate::comp::{
    ActionState::{Attack, Idle, Wield},
    AttackKind::BasicAttack,
    EcsCharacterState, EcsStateUpdate,
    ItemKind::Tool,
    StateHandle, WieldHandler,
};
use std::time::Duration;

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct BasicAttackHandler {
    /// How long the state has until exitting
    remaining_duration: Duration,
}

impl StateHandle for BasicAttackHandler {
    fn handle(&self, ecs_data: &EcsCharacterState) -> EcsStateUpdate {
        let mut update = EcsStateUpdate {
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
            character: *ecs_data.character,
        };

        // Check if attack duration has expired
        if self.remaining_duration == Duration::default() {
            // If so, go back to wielding or idling
            update.character.action_state = if let Some(Tool { .. }) =
                ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind)
            {
                Wield(WieldHandler {
                    equip_delay: Duration::from_millis(TEMP_EQUIP_DELAY),
                })
            } else {
                Idle
            };

            return update;
        }

        // Otherwise, tick down remaining_duration, and keep rolling
        update.character.action_state = Attack(BasicAttack(BasicAttackHandler {
            remaining_duration: self
                .remaining_duration
                .checked_sub(Duration::from_secs_f32(ecs_data.dt.0))
                .unwrap_or_default(),
        }));

        return update;
    }
}
