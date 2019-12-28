use super::TEMP_EQUIP_DELAY;
use crate::comp::{
    ActionState::Wield, EcsStateData, ItemKind::Tool, StateHandle, StateUpdate, WieldState,
};
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Eq, Hash)]
pub struct IdleState;

impl StateHandle for IdleState {
    fn handle(&self, ecs_data: &EcsStateData) -> StateUpdate {
        let mut update = StateUpdate {
            character: *ecs_data.character,
            pos: *ecs_data.pos,
            vel: *ecs_data.vel,
            ori: *ecs_data.ori,
        };

        // Try to wield
        if ecs_data.inputs.toggle_wield.is_pressed()
            || ecs_data.inputs.primary.is_pressed()
            || ecs_data.inputs.secondary.is_pressed()
        {
            if let Some(Tool { .. }) = ecs_data.stats.equipment.main.as_ref().map(|i| &i.kind) {
                update.character.action_state = Wield(WieldState {
                    equip_delay: Duration::from_millis(TEMP_EQUIP_DELAY),
                })
            }

            // else unarmed stuff?
        }

        return update;
    }
}
